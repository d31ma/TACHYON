#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidVerifiedLinksGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidVerifiedLinksGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static host = process.env.TAC_LINKS_HOST ?? 'd31ma.github.io';
  static packageName = 'dev.tachyon.link_gate';

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidVerifiedLinksGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidVerifiedLinksGate.run(AndroidVerifiedLinksGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, relative, source) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static stage(project) {
    const { write, host } = AndroidVerifiedLinksGate;
    const application = { id: 'dev.tachyon.link-gate', version: '0.0.1', build: 1, links: { schemes: ['linkgate'], hosts: [host] } };
    write(project, 'client/manifest.json', JSON.stringify({ name: 'LinkGate', start_url: '/', tac: { origin: `https://${host}`, application } }));
    for (const [prefix, page] of [['', 'home'], ['orders/', 'orders']]) {
      write(project, 'client/pages/' + prefix + 'tac.html', `<link-${page} />`);
      write(project, 'client/components/link/' + page + '/tac.html', `<main><h1>${page} page</h1></main>`);
    }
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidVerifiedLinksGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(project) {
    const result = spawnSync(AndroidVerifiedLinksGate.binary(), ['build', project, '--target', 'android'], {
      cwd: AndroidVerifiedLinksGate.repo, encoding: 'utf8', timeout: 600000, maxBuffer: 8 * 1024 * 1024,
    });
    assert.equal(result.status, 0, `ty build --target android: ${result.error || result.stderr || result.stdout}`);
    return path.join(project, 'dist/android/LinkGate/LinkGate.apk');
  }

  static fingerprint(apk) {
    const { sdk, run } = AndroidVerifiedLinksGate;
    const tools = path.join(sdk, 'build-tools');
    const version = readdirSync(tools).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }))[0];
    const printed = run(path.join(tools, version, process.platform === 'win32' ? 'apksigner.bat' : 'apksigner'), ['verify', '--print-certs', apk]);
    const digest = printed.match(/certificate SHA-256 digest: ([0-9a-f]{64})/)?.[1];
    assert.ok(digest, `apksigner printed no SHA-256 digest: ${printed}`);
    return digest.toUpperCase().match(/../g).join(':');
  }

  static async checkServedFile(fingerprint) {
    const { host, packageName } = AndroidVerifiedLinksGate;
    const address = `https://${host}/.well-known/assetlinks.json`;
    const response = await fetch(address, { redirect: 'manual' });
    assert.equal(response.status, 200, `${address} answered ${response.status}; publish the files from scripts/native/verified-links-files.mjs first`);
    const statements = JSON.parse(await response.text());
    const listed = statements.some(({ target }) => target?.package_name === packageName && target.sha256_cert_fingerprints?.includes(fingerprint));
    assert.ok(listed, `${address} does not list ${packageName} with ${fingerprint}. Rebuild the files with TAC_ANDROID_CERT_SHA256=${fingerprint} and publish them.`);
    console.log(`OK: ${address} lists ${packageName} with the APK's certificate ${fingerprint}`);
  }

  static verificationState() {
    const { adbShell, packageName, host } = AndroidVerifiedLinksGate;
    const printed = adbShell('pm', 'get-app-links', packageName);
    return { printed, state: printed.match(new RegExp(`${host.replaceAll('.', '\\.')}: (\\S+)`))?.[1] ?? 'missing' };
  }

  static async verify() {
    const { adbShell, packageName, pause, verificationState, host } = AndroidVerifiedLinksGate;
    adbShell('pm', 'set-app-links', '--package', packageName, '0', 'all');
    adbShell('pm', 'verify-app-links', '--re-verify', packageName);
    let seen = verificationState();
    for (let attempt = 0; attempt < 90 && seen.state !== 'verified'; attempt++) {
      await pause(1000);
      seen = verificationState();
    }
    assert.equal(seen.state, 'verified', `Android did not verify ${host}:\n${seen.printed}`);
    console.log(`OK: Android verified ${host} for ${packageName}`);
  }

  static resumedActivity() {
    const dump = AndroidVerifiedLinksGate.adbShell('dumpsys', 'activity', 'activities');
    return dump.match(/(?:topResumedActivity|mResumedActivity)[^\n]*\{[^}]*\s(\S+\/\S+)/)?.[1] ?? '';
  }

  static async open(link) {
    const { adbShell, packageName, pause, resumedActivity } = AndroidVerifiedLinksGate;
    const started = adbShell('am', 'start', '-W', '-a', 'android.intent.action.VIEW', '-c', 'android.intent.category.BROWSABLE', '-d', `'${link}'`);
    let activity = '';
    for (let attempt = 0; attempt < 20 && !activity.startsWith(packageName + '/'); attempt++) {
      await pause(500);
      activity = resumedActivity();
    }
    assert.ok(activity.startsWith(packageName + '/'), `${link} must open the app without asking which app opens it; the resumed activity is ${activity}:\n${started}`);
  }

  static async page() {
    const { adb, run, packageName, pause } = AndroidVerifiedLinksGate;
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim();
      if (pid && AndroidVerifiedLinksGate.adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, 'the link app must start');
    const port = run(adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
    return { browser, port, page: browser.contexts()[0].pages()[0] };
  }

  static async waitForRoute(page, pathname, search, message) {
    for (let attempt = 0; attempt < 60; attempt++) {
      const address = new URL(page.url());
      if (address.pathname === pathname && address.search === search) {
        await page.getByRole('heading', { level: 1 }).waitFor();
        return;
      }
      await AndroidVerifiedLinksGate.pause(250);
    }
    assert.fail(`${message}: ${page.url()}`);
  }

  static async main() {
    const gate = AndroidVerifiedLinksGate;
    assert.ok(gate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-verified-links-'));
    let session;
    try {
      gate.stage(project);
      const apk = gate.build(project);
      await gate.checkServedFile(gate.fingerprint(apk));
      gate.run(gate.adb, ['wait-for-device']);
      gate.run(gate.adb, ['install', '-r', apk]);
      await gate.verify();
      gate.adbShell('am', 'force-stop', gate.packageName);
      await gate.open(`https://${gate.host}/orders?x=1`);
      session = await gate.page();
      await gate.waitForRoute(session.page, '/orders', '?x=1', 'a cold start from the verified link opens its route');
      assert.equal(await session.page.getByRole('heading', { level: 1 }).textContent(), 'orders page');
      gate.adbShell('input', 'keyevent', 'KEYCODE_HOME');
      await gate.open(`https://${gate.host}/orders?x=2`);
      await gate.waitForRoute(session.page, '/orders', '?x=2', 'a verified link while the app runs reaches the open page');
      console.log(`PASS: Android verified app links: ${gate.host} is verified, and its https links open the app at their route on a cold start and while it runs`);
    } finally {
      if (session) {
        await session.browser.close();
        gate.run(gate.adb, ['forward', '--remove', `tcp:${session.port}`]);
      }
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', gate.packageName], { stdio: 'ignore' });
      spawnSync(gate.adb, ['uninstall', gate.packageName], { stdio: 'ignore' });
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidVerifiedLinksGate.main();

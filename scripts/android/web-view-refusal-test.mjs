#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

class AndroidWebViewRefusalGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidWebViewRefusalGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static evidence = process.env.TAC_ANDROID_WEB_VIEW_REFUSAL_EVIDENCE ?? path.join(AndroidWebViewRefusalGate.repo, 'target/android-web-view-refusal-evidence');
  static name = 'RefusalGate';
  static applicationId = 'dev.tachyon.refusal-gate';
  static packageName = 'dev.tachyon.refusal_gate';
  static heading = 'Refusal gate home';
  static refusal = `${AndroidWebViewRefusalGate.name} needs an Android System WebView that supports the web message listener (WEB_MESSAGE_LISTENER).`;
  static lastWithoutListener = 83;
  static firstWithListener = 86;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidWebViewRefusalGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidWebViewRefusalGate.run(AndroidWebViewRefusalGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, relative, source) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidWebViewRefusalGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(project) {
    const { write, name, applicationId, heading } = AndroidWebViewRefusalGate;
    write(project, 'client/manifest.json', JSON.stringify({ name, start_url: '/', tac: { application: { id: applicationId, version: '0.0.1', build: 1 } } }));
    write(project, 'client/pages/tac.html', '<refusal-home />');
    write(project, 'client/components/refusal/home/tac.html', `<main><h1>${heading}</h1></main>`);
    const result = spawnSync(AndroidWebViewRefusalGate.binary(), ['build', project, '--target', 'android'], {
      cwd: AndroidWebViewRefusalGate.repo, encoding: 'utf8', timeout: 900000, maxBuffer: 8 * 1024 * 1024,
    });
    assert.equal(result.status, 0, `ty build --target android: ${result.error || result.stderr || result.stdout}`);
    return path.join(project, `dist/android/${name}/${name}.apk`);
  }

  static webView() {
    const { adbShell } = AndroidWebViewRefusalGate;
    const update = adbShell('dumpsys', 'webviewupdate');
    const current = update.match(/Current WebView package \(name, version\): \(([^,]+), ([^)]+)\)/);
    assert.ok(current, `dumpsys webviewupdate names no current WebView package: ${update}`);
    const packageName = current[1].trim();
    const versionName = adbShell('dumpsys', 'package', packageName).match(/versionName=(\S+)/)?.[1] ?? current[2].trim();
    return { packageName, versionName, major: Number(versionName.split('.')[0]), update };
  }

  static listener(webView) {
    const { lastWithoutListener, firstWithListener } = AndroidWebViewRefusalGate;
    const named = process.env.TAC_ANDROID_WEB_MESSAGE_LISTENER;
    if (named) {
      assert.ok(['absent', 'present'].includes(named), 'TAC_ANDROID_WEB_MESSAGE_LISTENER is absent or present');
      return named === 'present';
    }
    assert.ok(webView.major <= lastWithoutListener || webView.major >= firstWithListener,
      `WebView ${webView.versionName} may or may not have WEB_MESSAGE_LISTENER; set TAC_ANDROID_WEB_MESSAGE_LISTENER=absent|present`);
    return webView.major >= firstWithListener;
  }

  static pid() {
    const { adb, packageName } = AndroidWebViewRefusalGate;
    return spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8', timeout: 60000 }).stdout.trim();
  }

  static telemetry() {
    const { adb, packageName, applicationId } = AndroidWebViewRefusalGate;
    const read = spawnSync(adb, ['shell', 'run-as', packageName, 'cat', `files/tachyon/${applicationId}.jsonl`], { encoding: 'utf8', timeout: 60000 });
    return read.status === 0 ? read.stdout : '';
  }

  static events() {
    return AndroidWebViewRefusalGate.telemetry().split('\n').filter(Boolean).map(line => JSON.parse(line).event);
  }

  static screen() {
    const { adb, adbShell, repo } = AndroidWebViewRefusalGate;
    const dump = spawnSync(adb, ['shell', 'uiautomator', 'dump', '/sdcard/tachyon-refusal.xml'], { cwd: repo, encoding: 'utf8', timeout: 60000 });
    if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return '';
    return adbShell('cat', '/sdcard/tachyon-refusal.xml');
  }

  static screenshot(label) {
    const { adb, evidence } = AndroidWebViewRefusalGate;
    const file = path.join(evidence, `${label}.png`);
    const result = spawnSync(adb, ['exec-out', 'screencap', '-p'], { maxBuffer: 64 * 1024 * 1024, timeout: 60000 });
    if (result.status === 0) writeFileSync(file, result.stdout);
    return file;
  }

  static async waitFor(check, message) {
    const { screen, events, pause, evidence } = AndroidWebViewRefusalGate;
    let state = { xml: '', events: [] };
    for (let attempt = 0; attempt < 120; attempt++) {
      state = { xml: screen(), events: events() };
      if (check(state)) return state;
      await pause(500);
    }
    writeFileSync(path.join(evidence, 'dump-failed.xml'), state.xml);
    assert.fail(`${message}: events ${JSON.stringify(state.events)}`);
  }

  static async launch(apk) {
    const { run, adb, adbShell, packageName } = AndroidWebViewRefusalGate;
    run(adb, ['wait-for-device']);
    spawnSync(adb, ['uninstall', packageName], { encoding: 'utf8', timeout: 60000 });
    run(adb, ['install', '-r', apk]);
    const started = adbShell('date', "'+%m-%d %H:%M:%S.000'");
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    return started;
  }

  static crashes(started) {
    const { run, adb, packageName, evidence } = AndroidWebViewRefusalGate;
    const log = run(adb, ['logcat', '-d', '-v', 'threadtime', '-T', started], { maxBuffer: 64 * 1024 * 1024 });
    writeFileSync(path.join(evidence, 'logcat.txt'), log + '\n');
    return log.split('\n').filter(line => line.includes(packageName) && /AndroidRuntime|ANR in|has died|Force finishing/.test(line));
  }

  static async refused(webView, checks) {
    const gate = AndroidWebViewRefusalGate;
    const shown = await gate.waitFor(({ xml, events }) => xml.includes(gate.refusal) && events.includes('bridge.refused'), 'the refusal page reaches the screen');
    writeFileSync(path.join(gate.evidence, 'refusal.xml'), shown.xml);
    checks.push(`the screen shows "${gate.refusal}"`);
    assert.ok(shown.xml.includes(`This device has ${webView.packageName} ${webView.versionName}, which does not.`), `the refusal names ${webView.packageName} ${webView.versionName}`);
    checks.push(`the refusal names the installed WebView, ${webView.packageName} ${webView.versionName}`);
    assert.ok(!shown.xml.includes('android.webkit.WebView'), 'the refusal page holds no WebView');
    checks.push('the screen holds no WebView');
    await gate.pause(3000);
    const events = gate.events();
    assert.ok(events.includes('bridge.refused'), 'the host recorded bridge.refused');
    assert.ok(!events.includes('controller.mounted'), `no page mounted: ${JSON.stringify(events)}`);
    checks.push(`the jsonl records bridge.refused and no controller.mounted (${JSON.stringify(events)})`);
    return gate.screenshot('refusal');
  }

  static async opened(checks) {
    const gate = AndroidWebViewRefusalGate;
    const shown = await gate.waitFor(({ xml, events }) => xml.includes('android.webkit.WebView') && xml.includes(gate.heading) && events.includes('controller.mounted'), 'the app opens its page');
    writeFileSync(path.join(gate.evidence, 'opened.xml'), shown.xml);
    assert.ok(!shown.xml.includes(gate.refusal), 'no refusal page');
    checks.push(`the screen holds the app WebView with its "${gate.heading}" heading and no refusal text`);
    const events = gate.events();
    assert.ok(!events.includes('bridge.refused'), `no bridge.refused: ${JSON.stringify(events)}`);
    checks.push(`the jsonl records controller.mounted and no bridge.refused (${JSON.stringify(events)})`);
    return gate.screenshot('opened');
  }

  static async main() {
    const gate = AndroidWebViewRefusalGate;
    assert.ok(gate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    mkdirSync(gate.evidence, { recursive: true });
    const project = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'tachyon-android-refusal-'));
    try {
      const webView = gate.webView();
      writeFileSync(path.join(gate.evidence, 'webviewupdate.txt'), webView.update + '\n');
      const listener = gate.listener(webView);
      const apk = gate.build(project);
      const started = await gate.launch(apk);
      const checks = [];
      const screenshot = listener ? await gate.opened(checks) : await gate.refused(webView, checks);
      const pid = gate.pid();
      assert.match(pid, /^\d+$/, 'the app process is still running');
      const crashes = gate.crashes(started);
      assert.deepEqual(crashes, [], 'logcat shows no crash');
      checks.push(`the app process ${pid} is still running and logcat shows no crash`);
      const device = gate.adbShell('getprop', 'ro.build.version.release');
      const level = gate.adbShell('getprop', 'ro.build.version.sdk');
      const report = { device, level, webView: { packageName: webView.packageName, versionName: webView.versionName }, listener, checks, screenshot };
      writeFileSync(path.join(gate.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n');
      for (const check of checks) console.log(`OK: ${check}`);
      const outcome = listener ? 'opened its page normally' : 'showed the startup refusal page and loaded no page';
      console.log(`PASS: on the Android ${device} (API ${level}) emulator with ${webView.packageName} ${webView.versionName}, the app ${outcome}. Report: ${path.join(gate.evidence, 'report.json')}`);
    } finally {
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', gate.packageName], { stdio: 'ignore' });
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidWebViewRefusalGate.main();

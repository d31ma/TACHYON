#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { PageServicesFixture } from '../native/page-services-fixture.mjs';

class AndroidFilePickerGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidFilePickerGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static languages = PageServicesFixture.list('TAC_ANDROID_PICKER_CALLERS', 'js,kt,rs,swift,cs');
  static linked = ['kt', 'rs'];
  static fileName = 'tachyon-fp2-notes.txt';
  static device = `/sdcard/Download/${AndroidFilePickerGate.fileName}`;
  static evidence = process.env.TAC_ANDROID_PICKER_EVIDENCE ?? path.join(AndroidFilePickerGate.repo, 'target/android-file-picker-evidence');
  static pickerPackages = ['com.google.android.documentsui', 'com.android.documentsui'];
  static waitAttempts = 120;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidFilePickerGate.repo, encoding: 'utf8', timeout: 1800000, maxBuffer: 16 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidFilePickerGate.run(AndroidFilePickerGate.adb, ['shell', ...args], { timeout: 120000 });
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidFilePickerGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static screen() {
    const { adbShell } = AndroidFilePickerGate;
    adbShell('uiautomator', 'dump', '/sdcard/tachyon-file-picker.xml');
    return adbShell('cat', '/sdcard/tachyon-file-picker.xml');
  }

  static nodes(shown) {
    return [...shown.matchAll(/<node [^>]*>/g)].map(([node]) => ({
      text: node.match(/ text="([^"]*)"/)?.[1] ?? '',
      description: node.match(/ content-desc="([^"]*)"/)?.[1] ?? '',
      packageName: node.match(/ package="([^"]*)"/)?.[1] ?? '',
      bounds: (node.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/) ?? []).slice(1).map(Number),
    })).filter(node => node.bounds.length === 4);
  }

  static tap(node) {
    const [left, top, right, bottom] = node.bounds;
    AndroidFilePickerGate.adbShell('input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2)));
    return node.text || node.description;
  }

  static pickerShown(nodes) {
    return nodes.some(node => AndroidFilePickerGate.pickerPackages.includes(node.packageName));
  }

  static async waitForPicker() {
    const gate = AndroidFilePickerGate;
    for (let attempt = 0; attempt < gate.waitAttempts; attempt++) {
      const nodes = gate.nodes(gate.screen());
      if (gate.pickerShown(nodes)) return nodes;
      await gate.pause(250);
    }
    assert.fail('The system document picker did not open');
  }

  static async choose() {
    const gate = AndroidFilePickerGate;
    const taps = [];
    for (let attempt = 0; attempt < gate.waitAttempts; attempt++) {
      const nodes = gate.nodes(gate.screen());
      const file = nodes.find(node => node.text === gate.fileName);
      const downloads = nodes.find(node => ['Downloads', 'Download'].includes(node.text) && gate.pickerShown([node]));
      const roots = nodes.find(node => node.description === 'Show roots');
      const next = file ?? downloads ?? roots;
      if (next) taps.push(gate.tap(next));
      if (file) return taps;
      await gate.pause(next ? 1000 : 250);
    }
    assert.fail(`The picker never listed ${gate.fileName}: ${taps.join(' > ')}`);
  }

  static screenshot(name) {
    const file = path.join(AndroidFilePickerGate.evidence, `${name}.png`);
    const result = spawnSync(AndroidFilePickerGate.adb, ['exec-out', 'screencap', '-p'], { maxBuffer: 64 * 1024 * 1024, timeout: 60000 });
    if (result.status === 0) writeFileSync(file, result.stdout);
    return file;
  }

  static async launch(apk, packageName) {
    const { run, adb, adbShell, pause } = AndroidFilePickerGate;
    spawnSync(adb, ['uninstall', packageName], { encoding: 'utf8', timeout: 60000 });
    run(adb, ['install', '-r', apk], { timeout: 300000 });
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 120; attempt++) {
      pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8', timeout: 30000 }).stdout.trim();
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, `${packageName} must start`);
    return pid;
  }

  static async status(page, expected) {
    const paragraph = page.locator('p').first();
    await page.waitForFunction(text => document.querySelector('p')?.textContent.trim() === text, expected, { timeout: 90000 })
      .catch(async () => assert.fail(`the page shows "${await paragraph.textContent()}", not "${expected}"`));
    return (await paragraph.textContent()).trim();
  }

  static async check(language, binary) {
    const gate = AndroidFilePickerGate;
    const project = mkdtempSync(path.join(tmpdir(), `tachyon-android-picker-${language}-`));
    const packageName = `dev.tachyon.picker_${language}`;
    let browser;
    let port;
    try {
      const { application } = PageServicesFixture.stagePicker(project, language);
      const [read, cancelled] = PageServicesFixture.pickerStatuses[language];
      gate.run(binary, ['build', project, '--target', 'android']);
      const pid = await gate.launch(path.join(project, 'dist/android', application, `${application}.apk`), packageName);
      port = gate.run(gate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, 'Android WebView exposes a live document');
      const choose = page.getByRole('button', { name: 'Choose file' });
      await choose.waitFor({ timeout: 120000 });
      assert.equal(await gate.status(page, 'Ready'), 'Ready');
      const wasm = await page.evaluate(() => performance.getEntriesByType('resource').some(entry => entry.name.endsWith('.wasm')));
      assert.equal(wasm, !gate.linked.includes(language) && language !== 'js', `${language}: ${gate.linked.includes(language) ? 'linked into the app' : 'runs in the page'}`);
      await choose.click();
      await gate.waitForPicker();
      gate.screenshot(`${language}-picker`);
      const taps = await gate.choose();
      const chosen = await gate.status(page, read);
      gate.screenshot(`${language}-read`);
      await choose.click();
      await gate.waitForPicker();
      gate.adbShell('input', 'keyevent', 'KEYCODE_BACK');
      const closed = await gate.status(page, cancelled);
      const result = { language, linked: gate.linked.includes(language), webAssembly: wasm, taps, chosen, closed };
      console.log(`==> ${language}: ${result.linked ? 'linked' : wasm ? 'WebAssembly' : 'page'} FilePicker chose ${gate.fileName} (${taps.join(' > ')}) and read it (${chosen}), then Back closed the picker (${closed})`);
      return result;
    } finally {
      if (browser) await browser.close();
      if (port) spawnSync(gate.adb, ['forward', '--remove', `tcp:${port}`], { timeout: 30000 });
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', packageName], { timeout: 30000 });
      spawnSync(gate.adb, ['uninstall', packageName], { timeout: 60000 });
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(project, { recursive: true, force: true });
    }
  }

  static async main() {
    const gate = AndroidFilePickerGate;
    assert.ok(gate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    mkdirSync(gate.evidence, { recursive: true });
    gate.run(gate.adb, ['wait-for-device'], { timeout: 120000 });
    const notes = path.join(mkdtempSync(path.join(tmpdir(), 'tachyon-android-picker-file-')), gate.fileName);
    writeFileSync(notes, 'Quarterly notes\n');
    gate.run(gate.adb, ['push', notes, gate.device], { timeout: 120000 });
    const binary = gate.binary();
    const report = { device: gate.adbShell('getprop', 'ro.product.model'), release: gate.adbShell('getprop', 'ro.build.version.release'), results: [], failures: [] };
    for (const language of gate.languages) {
      try {
        report.results.push(await gate.check(language, binary));
      } catch (error) {
        console.log(`==> ${language}: FAIL ${String(error.message).slice(0, 2000)}`);
        report.failures.push({ language, error: String(error.message).slice(0, 4000) });
      }
      writeFileSync(path.join(gate.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n');
    }
    spawnSync(gate.adb, ['shell', 'rm', '-f', gate.device], { timeout: 30000 });
    assert.deepEqual(report.failures.map(({ language }) => language), [], 'some FilePicker callers failed; see the report');
    console.log(`PASS: Android ${report.release} (${report.device}) FilePicker callers (${gate.languages.join(', ')}) chose and read a file through the system document picker and answered cancelled after Back. Report: ${path.join(gate.evidence, 'report.json')}`);
  }
}

await AndroidFilePickerGate.main();

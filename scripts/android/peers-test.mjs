#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { PeersNativeProbe } from '../native/peers-native-probe.mjs';

class AndroidPeersGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidPeersGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.peers_gate';
  static apk = 'dist/android/PeersGate/PeersGate.apk';
  static internet = 'android.permission.INTERNET: granted=true';
  static waitRounds = 160;
  static waitStep = 250;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidPeersGate.repo, encoding: 'utf8', timeout: 240000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidPeersGate.run(AndroidPeersGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static stage(project) {
    const files = PeersNativeProbe.files({ id: 'dev.tachyon.peers-gate', name: 'Peers Gate', iceServers: [], relayOnly: false });
    for (const [name, contents] of Object.entries(files)) {
      const file = path.join(project, name);
      mkdirSync(path.dirname(file), { recursive: true });
      writeFileSync(file, contents);
    }
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidPeersGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static async launch(apk) {
    const { run, adb, adbShell, packageName, pause } = AndroidPeersGate;
    run(adb, ['wait-for-device']);
    spawnSync(adb, ['uninstall', packageName], { encoding: 'utf8', timeout: 60000 });
    run(adb, ['install', '-r', apk]);
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = adbShell('pidof', packageName);
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, 'The peers app must start');
    return pid;
  }

  static async result(page) {
    const { waitRounds, waitStep, pause } = AndroidPeersGate;
    let text = '';
    for (let round = 0; round < waitRounds; round++) {
      text = await page.evaluate((label) => document.querySelector(`main[aria-label="${label}"]`)?.textContent?.trim() ?? '', PeersNativeProbe.LABEL);
      if (text.startsWith(PeersNativeProbe.PREFIX) && !text.endsWith(PeersNativeProbe.CHECKING)) return text.slice(PeersNativeProbe.PREFIX.length);
      await pause(waitStep);
    }
    return text;
  }

  static async main() {
    assert.ok(AndroidPeersGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const gate = AndroidPeersGate;
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-peers-'));
    let browser;
    let port;
    try {
      gate.stage(project);
      gate.run(gate.binary(), ['build', project, '--target', 'android']);
      const pid = await gate.launch(path.join(project, gate.apk));
      assert.ok(gate.adbShell('dumpsys', 'package', gate.packageName).includes(gate.internet), 'an app that grants peers holds the INTERNET permission');
      port = gate.run(gate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, 'Android WebView exposes a live document');
      const result = await gate.result(page);
      assert.equal(result, PeersNativeProbe.PASSED, `two Peers connections in the Android WebView must connect and carry ${PeersNativeProbe.TOTAL} bytes, got ${result}`);
      console.log(`PASS: Android peers: the app holds the INTERNET permission for peers, and two Peers connections in the WebView (https://appassets.tachyon.local) connected and delivered ${PeersNativeProbe.TOTAL} bytes on an ordered channel`);
    } finally {
      if (browser) await browser.close();
      if (port) gate.run(gate.adb, ['forward', '--remove', `tcp:${port}`]);
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', gate.packageName], { encoding: 'utf8', timeout: 60000 });
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidPeersGate.main();

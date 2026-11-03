#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidPythonServiceGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static feature = path.join(AndroidPythonServiceGate.repo, 'website/content/features/web-assembly-services');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidPythonServiceGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.tally_python';
  static apk = 'dist/android/Tally/Tally.apk';
  static appOrigin = 'https://appassets.tachyon.local';
  static runtimeFolder = '.tachyon/services/python/314.0.7';
  static runtimeFiles = ['pyodide.mjs', 'pyodide.asm.mjs', 'pyodide.asm.wasm', 'python_stdlib.zip', 'pyodide-lock.json'];
  static readyMilliseconds = 120000;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidPythonServiceGate.repo, encoding: 'utf8', timeout: 900000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidPythonServiceGate.run(AndroidPythonServiceGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static stage(project) {
    const { feature } = AndroidPythonServiceGate;
    for (const entry of ['pages', 'components']) cpSync(path.join(feature, 'client', entry), path.join(project, 'client', entry), { recursive: true });
    const manifest = JSON.parse(readFileSync(path.join(feature, 'client/manifest.json'), 'utf8'));
    manifest.name = 'Tally';
    manifest.start_url = '/';
    manifest.tac = { ...manifest.tac, application: { id: 'dev.tachyon.tally-python', version: '0.1.0', build: 1 } };
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest));
    mkdirSync(path.join(project, 'client/services'), { recursive: true });
    cpSync(path.join(feature, 'client/services/TallyService.py'), path.join(project, 'client/services/TallyService.py'));
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidPythonServiceGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static checkBundle(project) {
    const { run, apk, runtimeFolder, runtimeFiles } = AndroidPythonServiceGate;
    const host = JSON.parse(readFileSync(path.join(project, 'dist/android/tachyon.host.json'), 'utf8'));
    assert.deepEqual(host.companions, [], 'A Python service is not linked into the Android host');
    const entries = run('unzip', ['-Z1', path.join(project, apk)]).split('\n');
    for (const file of runtimeFiles) {
      assert.ok(entries.includes(`assets/WebBundle/${runtimeFolder}/${file}`), `The APK carries ${file} in the web bundle`);
    }
    assert.ok(existsSync(path.join(project, 'dist/android/web/.tachyon/services/TallyService.js')), 'The Python service has its page module');
    return statSync(path.join(project, apk)).size;
  }

  static async launch() {
    const { adbShell, packageName, pause } = AndroidPythonServiceGate;
    adbShell('am', 'force-stop', packageName);
    const started = Date.now();
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 120; attempt++) {
      pid = spawnSync(AndroidPythonServiceGate.adb, ['shell', 'pidof', packageName], { encoding: 'utf8', timeout: 60000 }).stdout.trim();
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(250);
    }
    assert.match(pid, /^\d+$/, 'The Tally app must start');
    return { pid, started };
  }

  static async connect(pid) {
    const port = AndroidPythonServiceGate.run(AndroidPythonServiceGate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
    const page = browser.contexts()[0].pages()[0];
    assert.ok(page, 'Android WebView exposes a live document');
    page.on('pageerror', error => console.error('Android page error:', error.message));
    page.on('console', message => { if (message.type() === 'error') console.error('Android console:', message.text()); });
    return { port, browser, page };
  }

  static async disconnect(connection) {
    if (!connection) return;
    await connection.browser.close();
    AndroidPythonServiceGate.run(AndroidPythonServiceGate.adb, ['forward', '--remove', `tcp:${connection.port}`]);
  }

  static kilobytes(text) {
    return Number(text.match(/TOTAL PSS:\s+(\d+)/)?.[1] ?? text.match(/^\s*TOTAL\s+(\d+)/m)?.[1]);
  }

  static memory(pid) {
    const { adbShell, kilobytes } = AndroidPythonServiceGate;
    const app = kilobytes(adbShell('dumpsys', 'meminfo', pid));
    const renderers = adbShell('ps', '-A', '-o', 'PID,NAME').split('\n').filter(line => /webview:sandboxed_process/.test(line)).map(line => line.trim().split(/\s+/)[0]);
    const renderer = renderers.map(id => kilobytes(adbShell('dumpsys', 'meminfo', id))).filter(Number.isFinite).reduce((total, value) => total + value, 0);
    return { appMiB: Math.round(app / 1024), rendererMiB: Math.round(renderer / 1024), renderers: renderers.length };
  }

  static async checkTally(page, started) {
    const { appOrigin, runtimeFolder, readyMilliseconds } = AndroidPythonServiceGate;
    await page.getByText('0 in the basket', { exact: true }).waitFor({ timeout: readyMilliseconds });
    await page.getByRole('button', { name: 'Add two' }).click();
    await page.getByText('2 in the basket', { exact: true }).waitFor({ timeout: readyMilliseconds });
    await page.getByText('2 items', { exact: true }).waitFor({ timeout: readyMilliseconds });
    const answered = Date.now() - started;
    const loads = await page.evaluate(() => performance.getEntriesByType('resource').map(entry => ({ name: entry.name, end: Math.round(entry.responseEnd) })));
    const runtime = loads.filter(load => load.name.includes('/.tachyon/services/python/'));
    assert.ok(runtime.length > 0, `The page loads the Pyodide runtime: ${loads.map(load => load.name).join(', ')}`);
    for (const load of runtime) {
      assert.ok(load.name.startsWith(`${appOrigin}/${runtimeFolder}/`), `Pyodide loads from the app bundle origin: ${load.name}`);
    }
    assert.ok(runtime.some(load => load.name.endsWith('/pyodide.asm.wasm')), 'The page loads pyodide.asm.wasm from the bundle');
    return { answered, runtime };
  }

  static async main() {
    const gate = AndroidPythonServiceGate;
    assert.ok(gate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-python-'));
    let connection;
    try {
      gate.stage(project);
      const building = Date.now();
      gate.run(gate.binary(), ['build', project, '--target', 'android']);
      const built = Math.round((Date.now() - building) / 1000);
      const apkBytes = gate.checkBundle(project);
      gate.run(gate.adb, ['wait-for-device']);
      gate.run(gate.adb, ['install', '-r', path.join(project, gate.apk)]);
      gate.adbShell('pm', 'clear', gate.packageName);
      const { pid, started } = await gate.launch();
      connection = await gate.connect(pid);
      const { answered, runtime } = await gate.checkTally(connection.page, started);
      const memory = gate.memory(pid);
      const files = runtime.map(load => `${load.name.slice(gate.appOrigin.length)} by ${load.end} ms`).join(', ');
      console.log(`PASS: Android Python page service: Pyodide 314.0.7 ran TallyService.py in the web view of process ${pid}; the first answer came ${(answered / 1000).toFixed(1)} s after am start; the runtime loaded from ${gate.appOrigin} (${files}); memory ${memory.appMiB} MiB PSS in the app process and ${memory.rendererMiB} MiB in ${memory.renderers} web view renderer process(es); APK ${(apkBytes / 1048576).toFixed(1)} MiB, built in ${built} s`);
    } finally {
      await gate.disconnect(connection).catch(() => {});
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', gate.packageName], { encoding: 'utf8', timeout: 60000 });
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidPythonServiceGate.main();

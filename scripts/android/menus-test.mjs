#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidMenusGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidMenusGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.menu_gate';
  static menus = {
    file: [{ id: 'status', label: 'Status', route: '/status', shortcut: { key: 'k', modifiers: ['command'] } }],
    help: [{ id: 'guide', label: 'Guide', route: '/guide' }],
  };

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidMenusGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidMenusGate.run(AndroidMenusGate.adb, ['shell', ...args]);
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
    const { write, menus } = AndroidMenusGate;
    write(project, 'client/manifest.json', JSON.stringify({ name: 'MenuGate', start_url: '/', tac: { application: { id: 'dev.tachyon.menu-gate', version: '0.0.1', build: 1, menus } } }));
    for (const [prefix, page] of [['', 'home'], ['status/', 'status'], ['guide/', 'guide']]) {
      write(project, 'client/pages/' + prefix + 'tac.html', `<menu-${page} />`);
      write(project, 'client/components/menu/' + page + '/tac.html', `<main><h1>${page} page</h1></main>`);
    }
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidMenusGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(project) {
    const result = spawnSync(AndroidMenusGate.binary(), ['build', project, '--target', 'android'], {
      cwd: AndroidMenusGate.repo, encoding: 'utf8', timeout: 600000, maxBuffer: 8 * 1024 * 1024,
    });
    assert.equal(result.status, 0, `ty build --target android: ${result.error || result.stderr || result.stdout}`);
    assert.match(result.stdout + result.stderr, /app bar overflow menu/, 'the build notes how Android adapts the menus');
    return path.join(project, 'dist/android/MenuGate/MenuGate.apk');
  }

  static async launch(apk) {
    const { run, adb, adbShell, packageName, pause } = AndroidMenusGate;
    run(adb, ['wait-for-device']);
    run(adb, ['install', '-r', apk]);
    adbShell('am', 'force-stop', packageName);
    adbShell('pm', 'clear', packageName);
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim();
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, 'the menus app must start');
    return pid;
  }

  static screen(windows = false) {
    const { adb, adbShell, repo } = AndroidMenusGate;
    const scope = windows ? ['--windows'] : [];
    const dump = spawnSync(adb, ['shell', 'uiautomator', 'dump', ...scope, '/sdcard/tachyon-menus.xml'], { cwd: repo, encoding: 'utf8', timeout: 60000 });
    if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return '';
    return adbShell('cat', '/sdcard/tachyon-menus.xml');
  }

  static async tap(attribute, value) {
    const { screen, adbShell, pause } = AndroidMenusGate;
    for (let attempt = 0; attempt < 20; attempt++) {
      const node = screen().split('<node ').find(line => line.includes(`${attribute}="${value}"`));
      const bounds = node?.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/);
      if (bounds) {
        const [left, top, right, bottom] = bounds.slice(1).map(Number);
        adbShell('input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2)));
        return;
      }
      await pause(500);
    }
    assert.fail(`No ${attribute}="${value}" on screen: ${screen()}`);
  }

  static async waitForPath(page, expected, message) {
    for (let attempt = 0; attempt < 60; attempt++) {
      if (new URL(page.url()).pathname === expected) {
        await page.getByRole('heading', { level: 1 }).waitFor();
        return;
      }
      await AndroidMenusGate.pause(250);
    }
    assert.fail(`${message}: ${page.url()}`);
  }

  static async checkShortcutList() {
    const { adbShell, screen, pause } = AndroidMenusGate;
    adbShell('input', 'keycombination', 'KEYCODE_META_LEFT', 'KEYCODE_SLASH');
    let list = '';
    for (let attempt = 0; attempt < 20 && !list.includes('text="Status"'); attempt++) {
      await pause(500);
      list = screen(true);
    }
    assert.ok(['text="File"', 'text="Status"', 'text="Ctrl"', 'text="K"'].every(text => list.includes(text)),
      `The system keyboard shortcuts list shows File > Status, Ctrl+K: ${list}`);
    adbShell('input', 'keyevent', 'KEYCODE_ESCAPE');
    await pause(500);
  }

  static async checkShortcut(page) {
    AndroidMenusGate.adbShell('input', 'keycombination', 'KEYCODE_CTRL_LEFT', 'KEYCODE_K');
    await AndroidMenusGate.waitForPath(page, '/status', 'Ctrl+K opens the Status menu item');
    assert.equal(await page.getByRole('heading', { level: 1 }).textContent(), 'status page');
  }

  static async checkOverflowMenu(page) {
    const { tap, screen, waitForPath } = AndroidMenusGate;
    await tap('content-desc', 'More options');
    const menu = screen();
    assert.ok(menu.includes('text="Status"') && menu.includes('text="Guide"'), `The overflow menu lists Status and Guide: ${menu}`);
    await tap('text', 'Guide');
    await waitForPath(page, '/guide', 'The Guide overflow item opens its page');
    assert.equal(await page.getByRole('heading', { level: 1 }).textContent(), 'guide page');
  }

  static async main() {
    assert.ok(AndroidMenusGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-menus-'));
    let browser;
    let port;
    try {
      AndroidMenusGate.stage(project);
      const apk = AndroidMenusGate.build(project);
      const source = readFileSync(path.join(project, 'dist/android/MenuGate/project/app/src/main/java/dev/tachyon/menu_gate/MainActivity.java'), 'utf8');
      assert.ok(source.includes('new MenuRoute("status", "Status", "file", "/status", "k", KeyEvent.META_CTRL_ON)'));
      const pid = await AndroidMenusGate.launch(apk);
      port = AndroidMenusGate.run(AndroidMenusGate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, 'Android WebView exposes a live document');
      await page.getByRole('heading', { name: 'home page' }).waitFor();
      await AndroidMenusGate.checkShortcutList();
      await AndroidMenusGate.checkShortcut(page);
      await AndroidMenusGate.checkOverflowMenu(page);
      console.log('PASS: Android menus: the shortcuts list shows File > Status, and Ctrl+K and the app bar overflow menu open their pages');
    } finally {
      if (browser) await browser.close();
      if (port) AndroidMenusGate.run(AndroidMenusGate.adb, ['forward', '--remove', `tcp:${port}`]);
      AndroidMenusGate.adbShell('am', 'force-stop', AndroidMenusGate.packageName);
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidMenusGate.main();

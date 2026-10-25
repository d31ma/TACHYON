#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidWindowSettingsGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static root = mkdtempSync(path.join(tmpdir(), 'tachyon-android-window-'));
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidWindowSettingsGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.windowsettings';
  static apps = {
    standard: { theme_color: '#0f6cbd', background_color: '#ffaa00', orientation: 'portrait', window: {} },
    integrated: { theme_color: '#ffffff', background_color: '#202020', orientation: 'portrait-primary', window: { titleBar: 'integrated' } },
    fullscreen: { display: 'fullscreen', orientation: 'landscape', window: {} },
  };

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidWindowSettingsGate.repo, timeout: 600000, maxBuffer: 64 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout;
  }

  static text(command, args) {
    return AndroidWindowSettingsGate.run(command, args, { encoding: 'utf8' }).trim();
  }

  static adbShell(...args) {
    return AndroidWindowSettingsGate.text(AndroidWindowSettingsGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, relative, source) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static stage(name, settings) {
    const { write, packageName } = AndroidWindowSettingsGate;
    const project = path.join(AndroidWindowSettingsGate.root, name);
    const { window, ...members } = settings;
    write(project, 'client/manifest.json', JSON.stringify({
      name: 'WindowSettings', start_url: '/', ...members,
      tac: { application: { id: packageName, version: '1.0.0', window }, styles: ['/shared/window-probe.css'] },
    }));
    write(project, 'client/shared/window-probe.css', 'html, body { margin: 0; background: transparent; font: 14px sans-serif; }\n');
    write(project, 'client/pages/tac.html', '<window-probe />\n');
    write(project, 'client/components/window/probe/tac.html', '<header id="bar" style="height: var(--tac-titlebar-height, 0px); background: #ff00ff"></header><main><p>Window probe</p></main>');
    write(project, 'client/components/window/probe/tac.css', 'main { padding: 8px; }\n');
    return project;
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const { text } = AndroidWindowSettingsGate;
    text('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(text('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(binary, name) {
    const project = AndroidWindowSettingsGate.stage(name, AndroidWindowSettingsGate.apps[name]);
    console.log(`Building the ${name} Android app...`);
    AndroidWindowSettingsGate.text(binary, ['build', project, '--target', 'android']);
    return path.join(project, 'dist/android/WindowSettings/WindowSettings.apk');
  }

  static screenOrientation(apk) {
    const tools = path.join(AndroidWindowSettingsGate.sdk, 'build-tools');
    const newest = readdirSync(tools).sort((a, b) => a.localeCompare(b, undefined, { numeric: true })).at(-1);
    const tree = AndroidWindowSettingsGate.text(path.join(tools, newest, 'aapt2'), ['dump', 'xmltree', '--file', 'AndroidManifest.xml', apk]);
    return tree.match(/screenOrientation\(0x[0-9a-f]+\)=(\d+)/)?.[1] ?? 'unspecified';
  }

  static async launch(apk, name) {
    const { run, text, adb, adbShell, packageName, pause } = AndroidWindowSettingsGate;
    text(adb, ['wait-for-device']);
    spawnSync(adb, ['uninstall', packageName], { stdio: 'ignore', timeout: 60000 });
    text(adb, ['install', '-r', apk]);
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim();
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, `The ${name} app must start`);
    const port = text(adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
    try {
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, `${name}: Android WebView exposes a live document`);
      await page.locator('#bar').waitFor({ state: 'attached' });
      await pause(1500);
      const facts = await page.evaluate(async () => {
        const style = getComputedStyle(document.documentElement);
        const variable = name => style.getPropertyValue(name).trim();
        return {
          width: innerWidth, height: innerHeight, ratio: devicePixelRatio,
          titleBarHeight: variable('--tac-titlebar-height'),
          leadingInset: variable('--tac-titlebar-leading-inset'),
          trailingInset: variable('--tac-titlebar-trailing-inset'),
          insetsAnswer: await __tachyonNativeHostCall('window.titleBarInsets', '{}'),
          barHeight: document.querySelector('#bar').getBoundingClientRect().height,
        };
      });
      const image = AndroidWindowSettingsGate.screenshot();
      console.log(`${name}: ${JSON.stringify(facts)} screen ${image.width}x${image.height}`);
      const bars = name === 'fullscreen' ? {} : {
        statusFrame: AndroidWindowSettingsGate.barFrame('status'),
        navigationFrame: AndroidWindowSettingsGate.barFrame('navigation'),
        appearance: AndroidWindowSettingsGate.appearance(),
      };
      return { ...facts, ...bars, image };
    } finally {
      await browser.close();
      spawnSync(adb, ['forward', '--remove', `tcp:${port}`], { stdio: 'ignore', timeout: 30000 });
      spawnSync(adb, ['shell', 'am', 'force-stop', packageName], { stdio: 'ignore', timeout: 30000 });
    }
  }

  static screenshot() {
    const raw = AndroidWindowSettingsGate.run(AndroidWindowSettingsGate.adb, ['exec-out', 'screencap']);
    const width = raw.readUInt32LE(0);
    const height = raw.readUInt32LE(4);
    const header = raw.length - width * height * 4;
    assert.ok(header === 12 || header === 16, `unexpected screencap size ${raw.length} for ${width}x${height}`);
    return { width, height, pixel: (x, y) => {
      const at = header + (Math.round(y) * width + Math.round(x)) * 4;
      return [raw[at], raw[at + 1], raw[at + 2]];
    } };
  }

  static near(pixel, hex) {
    return [1, 3, 5].every((start, index) => Math.abs(Number.parseInt(hex.slice(start, start + 2), 16) - pixel[index]) <= 8);
  }

  static rowShare(image, y, hex) {
    let matches = 0;
    for (let x = 0; x < image.width; x++) if (AndroidWindowSettingsGate.near(image.pixel(x, y), hex)) matches++;
    return matches / image.width;
  }

  static barFrame(type) {
    const names = { status: 'statusBars|ITYPE_STATUS_BAR', navigation: 'navigationBars|ITYPE_NAVIGATION_BAR' }[type];
    const pattern = new RegExp(`InsetsSource[^\\n]*type=(?:${names})[^\\n]*?frame=\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]`, 'g');
    const frames = [...AndroidWindowSettingsGate.adbShell('dumpsys', 'window').matchAll(pattern)].map(found => found.slice(1).map(Number));
    const frame = frames.find(([left, top, right, bottom]) => right > left && bottom > top);
    assert.ok(frame, `dumpsys window names the ${type} bar frame: ${JSON.stringify(frames)}`);
    const [left, top, right, bottom] = frame;
    return { left, top, right, bottom };
  }

  static appearance() {
    const { adbShell, packageName } = AndroidWindowSettingsGate;
    const block = adbShell('dumpsys', 'window', 'windows').split(/\n(?=\s*Window #)/).find(part => part.includes(`${packageName}/`)) ?? '';
    return block.match(/apr=([A-Z_ |]+)/)?.[1]?.trim() ?? 'none';
  }

  static luminance([red, green, blue]) {
    return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
  }

  static barIcons(image, frame, spans) {
    const { luminance } = AndroidWindowSettingsGate;
    const counts = new Map();
    const pixels = [];
    for (const [start, end] of spans) {
      for (let y = frame.top + Math.round((frame.bottom - frame.top) * 0.2); y < frame.top + Math.round((frame.bottom - frame.top) * 0.8); y++) {
        for (let x = Math.round(image.width * start); x < Math.round(image.width * end); x++) {
          const pixel = image.pixel(x, y);
          pixels.push(pixel);
          counts.set(pixel.join(','), (counts.get(pixel.join(',')) ?? 0) + 1);
        }
      }
    }
    const bar = [...counts.entries()].sort((a, b) => b[1] - a[1])[0][0].split(',').map(Number);
    const icons = pixels.filter(pixel => pixel.some((channel, index) => Math.abs(channel - bar[index]) > 64)).map(luminance).sort((a, b) => a - b);
    return { bar, icons: icons.length, median: icons.length ? Math.round(icons[Math.floor(icons.length / 2)]) : -1 };
  }

  static checkBarIcons(result, name, light) {
    const { barIcons } = AndroidWindowSettingsGate;
    const status = barIcons(result.image, result.statusFrame, [[0.03, 0.35], [0.65, 0.97]]);
    const navigation = barIcons(result.image, result.navigationFrame, [[0.3, 0.7]]);
    console.log(`${name} bar icons (${light ? 'light' : 'dark'} expected, apr=${result.appearance}): status ${JSON.stringify(status)} navigation ${JSON.stringify(navigation)}`);
    for (const [bar, found] of [['status', status], ['navigation', navigation]]) {
      assert.ok(found.icons > 40, `${name}: the ${bar} bar shows icons: ${JSON.stringify(found)}`);
      assert.ok(light ? found.median > 160 : found.median < 128, `${name}: the ${bar} bar icons are ${light ? 'light' : 'dark'}: ${JSON.stringify(found)}`);
    }
  }

  static async checkSplash(apk) {
    const { text, adb, adbShell, packageName, pause, screenshot, near } = AndroidWindowSettingsGate;
    spawnSync(adb, ['uninstall', packageName], { stdio: 'ignore', timeout: 60000 });
    text(adb, ['install', '-r', apk]);
    for (let attempt = 1; attempt <= 3; attempt++) {
      adbShell('am', 'force-stop', packageName);
      await pause(1000);
      const pid = text(adb, ['shell', `am start -n ${packageName}/.MainActivity >/dev/null; i=0; until p=$(pidof ${packageName}) || [ $i -gt 50000 ]; do i=$((i+1)); done; run-as ${packageName} kill -STOP $p; echo $p`]);
      await pause(1500);
      const shown = adbShell('dumpsys', 'window', 'windows').includes(`Splash Screen ${packageName}`);
      const image = screenshot();
      spawnSync(adb, ['shell', 'run-as', packageName, 'kill', '-CONT', pid], { stdio: 'ignore', timeout: 30000 });
      adbShell('am', 'force-stop', packageName);
      const samples = [0.2, 0.35, 0.65, 0.8].map(share => image.pixel(image.width / 2, image.height * share));
      console.log(`splash attempt ${attempt}: process ${pid} held before its first frame, splash window ${shown ? 'shown' : 'absent'}, samples ${JSON.stringify(samples)}`);
      if (!shown) continue;
      assert.ok(samples.every(pixel => near(pixel, '#ffaa00')), `the splash screen shows background_color: ${JSON.stringify(samples)}`);
      return;
    }
    assert.fail('the splash window was never on screen while the app process was held before its first frame');
  }

  static environment() {
    const { adbShell } = AndroidWindowSettingsGate;
    const webView = adbShell('dumpsys', 'webviewupdate').match(/Current WebView package \(name, version\): \(([^)]*)\)/)?.[1] ?? 'unknown';
    return `API ${adbShell('getprop', 'ro.build.version.sdk')}, ${adbShell('getprop', 'ro.product.model')}, WebView ${webView}`;
  }

  static checkStandard(result, apk) {
    const { rowShare, near } = AndroidWindowSettingsGate;
    assert.equal(AndroidWindowSettingsGate.screenOrientation(apk), '12', 'portrait becomes userPortrait');
    assert.ok(result.height > result.width, 'a portrait app opens in portrait');
    assert.equal(result.titleBarHeight, '', 'a standard window sets no title bar variables');
    assert.ok(rowShare(result.image, 4, '#0f6cbd') > 0.5, `theme_color fills the status bar: ${result.image.pixel(result.image.width / 2, 4)}`);
    assert.ok(rowShare(result.image, result.image.height - 4, '#0f6cbd') > 0.5, `theme_color fills the navigation bar: ${result.image.pixel(result.image.width / 2, result.image.height - 4)}`);
    const middle = result.image.pixel(result.image.width / 2, result.image.height * 0.6);
    assert.ok(near(middle, '#ffaa00'), `background_color shows through the page: ${middle}`);
  }

  static checkIntegrated(result, apk) {
    const { rowShare } = AndroidWindowSettingsGate;
    assert.equal(AndroidWindowSettingsGate.screenOrientation(apk), '1', 'portrait-primary becomes portrait');
    const answer = JSON.parse(result.insetsAnswer);
    assert.ok(answer.height > 0, `the status bar height reaches the page: ${result.insetsAnswer}`);
    assert.equal(result.titleBarHeight, `${answer.height}px`);
    assert.equal(result.leadingInset, `${answer.leading}px`);
    assert.equal(result.trailingInset, `${answer.trailing}px`);
    assert.equal(result.barHeight, answer.height, 'the page sizes its bar from --tac-titlebar-height');
    assert.ok(rowShare(result.image, 4, '#ff00ff') > 0.5, `the page draws under the status bar: ${result.image.pixel(result.image.width / 2, 4)}`);
    const below = Math.round(answer.height * result.ratio) - 4;
    assert.ok(rowShare(result.image, below, '#ff00ff') > 0.5, `the bar reaches the bottom of the status bar: ${result.image.pixel(result.image.width / 2, below)}`);
  }

  static checkFullscreen(result, apk) {
    assert.equal(AndroidWindowSettingsGate.screenOrientation(apk), '11', 'landscape becomes userLandscape');
    assert.ok(result.width > result.height, `a landscape app opens in landscape: ${result.width}x${result.height}`);
    assert.ok(Math.round(result.height * result.ratio) >= result.image.height - 2, `display fullscreen hides the status and navigation bars: ${result.height * result.ratio} of ${result.image.height}`);
  }

  static async main() {
    assert.ok(AndroidWindowSettingsGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    try {
      const binary = AndroidWindowSettingsGate.binary();
      const apks = Object.fromEntries(Object.keys(AndroidWindowSettingsGate.apps).map(name => [name, AndroidWindowSettingsGate.build(binary, name)]));
      const device = AndroidWindowSettingsGate.environment();
      console.log(`Device: ${device}`);
      const standard = await AndroidWindowSettingsGate.launch(apks.standard, 'standard');
      AndroidWindowSettingsGate.checkStandard(standard, apks.standard);
      AndroidWindowSettingsGate.checkBarIcons(standard, 'standard', true);
      await AndroidWindowSettingsGate.checkSplash(apks.standard);
      const integrated = await AndroidWindowSettingsGate.launch(apks.integrated, 'integrated');
      AndroidWindowSettingsGate.checkIntegrated(integrated, apks.integrated);
      AndroidWindowSettingsGate.checkBarIcons(integrated, 'integrated', false);
      AndroidWindowSettingsGate.checkFullscreen(await AndroidWindowSettingsGate.launch(apks.fullscreen, 'fullscreen'), apks.fullscreen);
      console.log(`PASS: Android window settings on ${device}: theme_color system bars with light icons on a dark theme and dark icons on a light one, the background_color splash screen, background_color web view, integrated title bar insets, fullscreen and orientation`);
    } finally {
      spawnSync(AndroidWindowSettingsGate.adb, ['uninstall', AndroidWindowSettingsGate.packageName], { stdio: 'ignore', timeout: 60000 });
      rmSync(AndroidWindowSettingsGate.root, { recursive: true, force: true });
    }
  }
}

await AndroidWindowSettingsGate.main();

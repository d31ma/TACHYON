#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidFoldableGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static root = mkdtempSync(path.join(tmpdir(), 'tachyon-android-fold-'));
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidFoldableGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.foldable';
  static page = '#ffaa00';
  static bars = '#0f6cbd';
  static configChanges = { orientation: 0x80, screenLayout: 0x100, screenSize: 0x400, smallestScreenSize: 0x800, density: 0x1000 };
  static apps = {
    free: {},
    portrait: { orientation: 'portrait' },
    landscape: { orientation: 'landscape' },
  };

  static evidence() {
    const option = process.argv.find(argument => argument.startsWith('--evidence='));
    const folder = option ? path.resolve(option.slice('--evidence='.length)) : mkdtempSync(path.join(tmpdir(), 'tachyon-fold-evidence-'));
    mkdirSync(folder, { recursive: true });
    return folder;
  }

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidFoldableGate.repo, timeout: 600000, maxBuffer: 64 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout;
  }

  static text(command, args) {
    return AndroidFoldableGate.run(command, args, { encoding: 'utf8' }).trim();
  }

  static adbShell(...args) {
    return AndroidFoldableGate.text(AndroidFoldableGate.adb, ['shell', ...args]);
  }

  static console(...args) {
    return spawnSync(AndroidFoldableGate.adb, ['emu', ...args], { encoding: 'utf8', timeout: 30000 }).stdout?.trim() ?? '';
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, relative, source) {
    const file = path.join(project, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, source);
  }

  static stage(name, members) {
    const { write, packageName, page, bars } = AndroidFoldableGate;
    const project = path.join(AndroidFoldableGate.root, name);
    write(project, 'client/manifest.json', JSON.stringify({
      name: 'Foldable', start_url: '/', theme_color: bars, background_color: page, ...members,
      tac: { application: { id: packageName, version: '1.0.0' }, styles: ['/shared/fold-probe.css'] },
    }));
    write(project, 'client/shared/fold-probe.css', `html, body { margin: 0; background: ${page}; font: 14px sans-serif; }\n`);
    write(project, 'client/pages/tac.html', '<fold-probe />\n');
    write(project, 'client/pages/second/tac.html', '<fold-probe />\n');
    write(project, 'client/components/fold/probe/tac.html', '<main><p>Fold probe</p><label for="kept">Kept text</label><input id="kept" value="" /></main>');
    write(project, 'client/components/fold/probe/tac.css', 'main { padding: 8px; }\n');
    return project;
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const { text } = AndroidFoldableGate;
    text('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(text('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(binary, name) {
    const project = AndroidFoldableGate.stage(name, AndroidFoldableGate.apps[name]);
    console.log(`Building the ${name} Android app...`);
    AndroidFoldableGate.text(binary, ['build', project, '--target', 'android']);
    return path.join(project, 'dist/android/Foldable/Foldable.apk');
  }

  static manifestTree(apk) {
    const tools = path.join(AndroidFoldableGate.sdk, 'build-tools');
    const newest = readdirSync(tools).sort((a, b) => a.localeCompare(b, undefined, { numeric: true })).at(-1);
    return AndroidFoldableGate.text(path.join(tools, newest, 'aapt2'), ['dump', 'xmltree', '--file', 'AndroidManifest.xml', apk]);
  }

  static attribute(element, name) {
    return element.match(new RegExp(`:${name}\\(0x[0-9a-f]+\\)=(?:\\(type 0x[0-9a-f]+\\))?(\\S+)`))?.[1];
  }

  static number(value) {
    if (value === undefined) return 0;
    return value.startsWith('0x') ? Number.parseInt(value, 16) : Number.parseInt(value, 10);
  }

  static checkManifest(apk, name) {
    const { attribute, number } = AndroidFoldableGate;
    const tree = AndroidFoldableGate.manifestTree(apk);
    const activity = tree.slice(tree.indexOf('E: activity'));
    const resizeable = attribute(activity, 'resizeableActivity');
    const changes = number(attribute(activity, 'configChanges'));
    console.log(`${name} manifest: resizeableActivity=${resizeable ?? 'unset'} configChanges=0x${changes.toString(16)} (${activity.split('\n').filter(line => /resizeableActivity|configChanges/.test(line)).map(line => line.trim()).join('; ')})`);
    assert.ok(['true', '0xffffffff', '-1'].includes(resizeable), `${name}: the activity declares android:resizeableActivity="true": ${resizeable}`);
    for (const [flag, bit] of Object.entries(AndroidFoldableGate.configChanges)) {
      assert.ok((changes & bit) === bit, `${name}: android:configChanges names ${flag} (0x${changes.toString(16)})`);
    }
    assert.ok(!/maxAspectRatio|minAspectRatio/.test(activity), `${name}: the activity sets no aspect ratio limit`);
  }

  static foldControl() {
    const { console: emulator } = AndroidFoldableGate;
    const name = emulator('avd', 'name');
    assert.match(name, /OK/, `the emulator console answers: ${name}`);
    const states = AndroidFoldableGate.adbShell('cmd', 'device_state', 'print-states');
    console.log(`AVD ${name.split('\n')[0]}; device states: ${states.replace(/\s+/g, ' ')}`);
    assert.match(states, /CLOSED|FOLDED/i, `the device has a folded state; start the tachyon-fold AVD: ${states}`);
    return states;
  }

  static async setPosture(posture) {
    const { console: emulator, adbShell, pause, screenshot } = AndroidFoldableGate;
    const before = screenshot();
    const answer = emulator(posture);
    console.log(`adb emu ${posture}: ${answer.replace(/\s+/g, ' ')}`);
    assert.match(answer, /OK/, `the emulator console changes the fold state with "${posture}": ${answer}`);
    let last = '';
    let steady = 0;
    for (let attempt = 0; attempt < 60; attempt++) {
      await pause(500);
      const size = adbShell('wm', 'size');
      steady = size === last ? steady + 1 : 0;
      last = size;
      const image = screenshot();
      if (steady >= 3 && (image.width !== before.width || image.height !== before.height || attempt > 20)) break;
    }
    await pause(1500);
    return { state: adbShell('cmd', 'device_state', 'print-state'), size: last };
  }

  static activeDisplay() {
    const devices = AndroidFoldableGate.adbShell('dumpsys', 'display').split('DisplayDeviceInfo{').slice(1);
    const shown = devices.find(device => device.match(/mCurrentLayerStack=(-?\d+)/)?.[1] === '0');
    const id = shown?.match(/uniqueId="local:(\d+)"/)?.[1];
    assert.ok(id, `dumpsys display names the screen that shows the default display: ${devices.length} screens`);
    return id;
  }

  static screenshot() {
    const raw = AndroidFoldableGate.run(AndroidFoldableGate.adb, ['exec-out', 'screencap', '-d', AndroidFoldableGate.activeDisplay()]);
    const width = raw.readUInt32LE(0);
    const height = raw.readUInt32LE(4);
    const header = raw.length - width * height * 4;
    assert.ok(header === 12 || header === 16, `unexpected screencap size ${raw.length} for ${width}x${height}`);
    return { width, height, pixel: (x, y) => {
      const at = header + (Math.round(y) * width + Math.round(x)) * 4;
      return [raw[at], raw[at + 1], raw[at + 2]];
    } };
  }

  static keep(name) {
    const file = path.join(AndroidFoldableGate.evidenceFolder, `${name}.png`);
    writeFileSync(file, AndroidFoldableGate.run(AndroidFoldableGate.adb, ['exec-out', 'screencap', '-p', '-d', AndroidFoldableGate.activeDisplay()]));
    return file;
  }

  static near(pixel, hex) {
    return [1, 3, 5].every((start, index) => Math.abs(Number.parseInt(hex.slice(start, start + 2), 16) - pixel[index]) <= 8);
  }

  static black(pixel) {
    return pixel.every(channel => channel < 24);
  }

  static columnShare(image, x, top, bottom, test) {
    let matches = 0;
    for (let y = top; y < bottom; y++) if (test(image.pixel(x, y))) matches++;
    return matches / Math.max(1, bottom - top);
  }

  static rowShare(image, y, test) {
    let matches = 0;
    for (let x = 0; x < image.width; x++) if (test(image.pixel(x, y))) matches++;
    return matches / image.width;
  }

  static appWindow() {
    const { adbShell, packageName } = AndroidFoldableGate;
    const block = adbShell('dumpsys', 'window', 'windows').split(/\n(?=\s*Window #)/).find(part => part.includes(`${packageName}/`)) ?? '';
    const frame = block.match(/Frames: [^\n]*?\bframe=\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]/)?.slice(1).map(Number);
    assert.ok(frame, `dumpsys window names the app window frame: ${block.slice(0, 400)}`);
    const [left, top, right, bottom] = frame;
    return { left, top, right, bottom };
  }

  static letterboxReport() {
    const lines = AndroidFoldableGate.adbShell('dumpsys', 'activity', 'activities').split('\n');
    return lines.filter(line => /letterbox|isInSizeCompatMode|mCompatDisplayInsets|inSizeCompatMode/i.test(line)).map(line => line.trim()).slice(0, 8);
  }

  static sideMargins(image) {
    const { near, bars } = AndroidFoldableGate;
    const y = Math.round(image.height / 2);
    let left = 0;
    while (left < image.width && near(image.pixel(left, y), bars)) left++;
    let right = 0;
    while (right < image.width && near(image.pixel(image.width - 1 - right, y), bars)) right++;
    return left + right;
  }

  static checkFill(image, frame, name) {
    const { near, black, columnShare, rowShare, page, bars } = AndroidFoldableGate;
    assert.deepEqual(frame, { left: 0, top: 0, right: image.width, bottom: image.height }, `${name}: the app window fills the ${image.width}x${image.height} display`);
    const top = Math.round(image.height * 0.15);
    const bottom = Math.round(image.height * 0.85);
    const edges = {
      left: columnShare(image, 1, top, bottom, pixel => near(pixel, page) || near(pixel, bars)),
      right: columnShare(image, image.width - 2, top, bottom, pixel => near(pixel, page) || near(pixel, bars)),
      leftBlack: columnShare(image, 1, top, bottom, black),
      rightBlack: columnShare(image, image.width - 2, top, bottom, black),
      topBlack: rowShare(image, 1, black),
      bottomBlack: rowShare(image, image.height - 2, black),
      topBar: rowShare(image, 1, pixel => near(pixel, bars)),
      bottomBar: rowShare(image, image.height - 2, pixel => near(pixel, bars)),
    };
    console.log(`${name}: edges ${JSON.stringify(edges)}`);
    assert.ok(edges.left > 0.95 && edges.right > 0.95, `${name}: the page or its theme_color cutout area reaches the left and right edges (no pillarbox): ${JSON.stringify(edges)}`);
    assert.ok(edges.leftBlack < 0.02 && edges.rightBlack < 0.02, `${name}: no black side bars: ${JSON.stringify(edges)}`);
    assert.ok(edges.topBlack < 0.2 && edges.bottomBlack < 0.2, `${name}: no black top or bottom bars (no letterbox): ${JSON.stringify(edges)}`);
    assert.ok(edges.topBar > 0.5 && edges.bottomBar > 0.5, `${name}: the theme_color system bars reach the top and bottom edges: ${JSON.stringify(edges)}`);
  }

  static async connect(name) {
    const { text, adb, adbShell, packageName, pause } = AndroidFoldableGate;
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim();
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, `The ${name} app must start`);
    const port = text(adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
    return { pid, port, browser };
  }

  static appPages(link) {
    const pages = link.browser.contexts()[0]?.pages() ?? [];
    return pages.filter(candidate => candidate.url().startsWith('https://appassets.tachyon.local/') && !candidate.url().includes('__tachyon_bootstrap__'));
  }

  static async visiblePage(link) {
    const pages = AndroidFoldableGate.appPages(link);
    const states = await Promise.all(pages.map(page => page.evaluate(() => ({ visible: document.visibilityState, origin: performance.timeOrigin })).catch(() => null)));
    const shown = pages.map((page, index) => ({ page, state: states[index] })).filter(entry => entry.state?.visible === 'visible');
    shown.sort((first, second) => second.state.origin - first.state.origin);
    return { page: shown[0]?.page, pages: pages.length, shown: shown.length };
  }

  static async document(link, name) {
    for (let attempt = 0; attempt < 20; attempt++) {
      const found = await AndroidFoldableGate.visiblePage(link);
      if (found.page) {
        await found.page.locator('#kept').waitFor({ state: 'attached' });
        link.pages = found.pages;
        link.shown = found.shown;
        return found.page;
      }
      await AndroidFoldableGate.pause(500);
    }
    assert.fail(`${name}: the Android WebView shows the app page`);
  }

  static async facts(link, name) {
    const page = await AndroidFoldableGate.document(link, name);
    const facts = await page.evaluate(() => ({
      width: innerWidth, height: innerHeight, ratio: devicePixelRatio,
      origin: performance.timeOrigin, marker: globalThis.__foldMarker ?? null, typed: document.querySelector('#kept').value,
      url: location.href,
    }));
    return { ...facts, pages: link.pages, shown: link.shown };
  }

  static async reconnect(link, name) {
    await AndroidFoldableGate.disconnect(link);
    return AndroidFoldableGate.connect(name);
  }

  static async mark(link, name) {
    const page = await AndroidFoldableGate.document(link, name);
    const marker = `kept-${Date.now()}`;
    await page.locator('#kept').fill(marker);
    await page.evaluate(value => { globalThis.__foldMarker = value; }, marker);
    return marker;
  }

  static disconnect(link) {
    spawnSync(AndroidFoldableGate.adb, ['forward', '--remove', `tcp:${link.port}`], { stdio: 'ignore', timeout: 30000 });
    return link.browser.close().catch(() => {});
  }

  static async exercise(apk, name) {
    const { text, adb, adbShell, packageName, setPosture, screenshot, appWindow, checkFill, keep, facts, mark, connect, disconnect } = AndroidFoldableGate;
    await setPosture('unfold');
    spawnSync(adb, ['uninstall', packageName], { stdio: 'ignore', timeout: 60000 });
    text(adb, ['install', '-r', apk]);
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let link = await connect(name);
    try {
      const first = await facts(link, name);
      const marker = await mark(link, name);
      const createdBefore = AndroidFoldableGate.created();
      const results = [];
      for (const posture of ['unfold', 'fold', 'unfold']) {
        const reached = await setPosture(posture);
        const label = `${name}-${results.length + 1}-${posture === 'fold' ? 'folded' : 'unfolded'}`;
        const image = screenshot();
        const frame = appWindow();
        const shot = keep(label);
        console.log(`${label}: ${reached.state.replace(/\s+/g, ' ')} ${reached.size.replace(/\s+/g, ' ')} screen ${image.width}x${image.height} window ${JSON.stringify(frame)} letterbox ${JSON.stringify(AndroidFoldableGate.letterboxReport())} evidence ${shot}`);
        const focus = AndroidFoldableGate.focusedWindow();
        assert.ok(focus.includes(`${packageName}/`), `${label}: the app has the focus, no other window covers it: ${focus}`);
        checkFill(image, frame, label);
        const pid = spawnSync(adb, ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim();
        assert.equal(pid, link.pid, `${label}: the app process keeps running across the fold change`);
        link = await AndroidFoldableGate.reconnect(link, label);
        const now = await facts(link, label);
        console.log(`${label}: page ${JSON.stringify(now)}`);
        assert.equal(now.origin, first.origin, `${label}: the page is not reloaded by the fold change`);
        assert.equal(now.pages, 1, `${label}: the process holds one app page, not a page of a destroyed activity`);
        assert.equal(now.marker, marker, `${label}: the page keeps its script state`);
        assert.equal(now.typed, marker, `${label}: the page keeps typed text`);
        const margins = AndroidFoldableGate.sideMargins(image);
        assert.ok(Math.abs(Math.round(now.width * now.ratio) - (image.width - margins)) <= 3, `${label}: the page is as wide as the display less its theme_color cutout margins: ${now.width * now.ratio} of ${image.width} - ${margins}`);
        results.push({ label, image, now });
      }
      assert.equal(AndroidFoldableGate.created(), createdBefore, `${name}: the activity is not recreated by the fold changes`);
      const [unfolded, folded] = results;
      const short = ({ width, height }) => Math.min(width, height);
      assert.ok(short(folded.image) < short(unfolded.image), `${name}: folding makes the display smaller: ${unfolded.image.width}x${unfolded.image.height} -> ${folded.image.width}x${folded.image.height}`);
      assert.ok(short(folded.now) < short(unfolded.now), `${name}: the page reflows to the folded size: ${unfolded.now.width}x${unfolded.now.height} -> ${folded.now.width}x${folded.now.height}`);
      link = await AndroidFoldableGate.checkRestart(link, name);
    } finally {
      await disconnect(link);
      spawnSync(adb, ['shell', 'am', 'force-stop', packageName], { stdio: 'ignore', timeout: 30000 });
    }
  }

  static async checkRestart(link, name) {
    const { adbShell, pause, facts, connect, disconnect, document } = AndroidFoldableGate;
    const page = await document(link, name);
    await page.evaluate(() => location.assign('/second'));
    let opened;
    for (let attempt = 0; attempt < 40; attempt++) {
      await pause(500);
      opened = await facts(link, name).catch(() => null);
      if (opened?.url.includes('/second')) break;
    }
    assert.ok(opened?.url.includes('/second'), `${name}: the probe opens its second page: ${JSON.stringify(opened)}`);
    const night = adbShell('cmd', 'uimode', 'night');
    const restore = /yes/.test(night) ? 'yes' : 'no';
    adbShell('cmd', 'uimode', 'night', restore === 'yes' ? 'no' : 'yes');
    try {
      await disconnect(link);
      await pause(3000);
      let restarted;
      for (let attempt = 0; attempt < 20; attempt++) {
        link = await connect(name);
        restarted = await facts(link, name).catch(() => null);
        if (restarted && restarted.origin !== opened.origin) break;
        await disconnect(link);
        await pause(1000);
      }
      const shot = AndroidFoldableGate.keep(`${name}-4-restarted`);
      console.log(`${name}: after a restart (night mode ${restore} -> ${restore === 'yes' ? 'no' : 'yes'}): ${JSON.stringify(restarted)} evidence ${shot}`);
      assert.ok(restarted && restarted.origin !== opened.origin, `${name}: the night mode switch restarts the activity: ${JSON.stringify(restarted)}`);
      assert.equal(new URL(restarted.url).pathname, new URL(opened.url).pathname, `${name}: a restarted activity opens the page that was open`);
      return link;
    } finally {
      adbShell('cmd', 'uimode', 'night', restore);
    }
  }

  static created() {
    const lines = spawnSync(AndroidFoldableGate.adb, ['logcat', '-d', '-b', 'events', '-s', 'wm_create_activity'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }).stdout ?? '';
    return lines.split('\n').filter(line => line.includes('wm_create_activity') && line.includes(AndroidFoldableGate.packageName)).length;
  }

  static environment() {
    const { adbShell } = AndroidFoldableGate;
    const webView = adbShell('dumpsys', 'webviewupdate').match(/Current WebView package \(name, version\): \(([^)]*)\)/)?.[1] ?? 'unknown';
    return `API ${adbShell('getprop', 'ro.build.version.sdk')}, ${adbShell('getprop', 'ro.product.model')}, WebView ${webView}`;
  }

  static restoreErrorDialogs() {
    const kept = AndroidFoldableGate.errorDialogs;
    if (kept === undefined) return;
    const command = kept === 'null' ? ['delete', 'global', 'hide_error_dialogs'] : ['put', 'global', 'hide_error_dialogs', kept];
    spawnSync(AndroidFoldableGate.adb, ['shell', 'settings', ...command], { stdio: 'ignore', timeout: 30000 });
  }

  static focusedWindow() {
    return AndroidFoldableGate.adbShell('dumpsys', 'window').match(/mCurrentFocus=([^\n]*)/)?.[1]?.trim() ?? 'unknown';
  }

  static async main() {
    assert.ok(AndroidFoldableGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    AndroidFoldableGate.evidenceFolder = AndroidFoldableGate.evidence();
    console.log(`Screenshots: ${AndroidFoldableGate.evidenceFolder}`);
    try {
      AndroidFoldableGate.text(AndroidFoldableGate.adb, ['wait-for-device']);
      AndroidFoldableGate.errorDialogs = AndroidFoldableGate.adbShell('settings', 'get', 'global', 'hide_error_dialogs');
      AndroidFoldableGate.adbShell('settings', 'put', 'global', 'hide_error_dialogs', '1');
      AndroidFoldableGate.foldControl();
      const device = AndroidFoldableGate.environment();
      console.log(`Device: ${device}`);
      const binary = AndroidFoldableGate.binary();
      const failures = [];
      for (const name of Object.keys(AndroidFoldableGate.apps)) {
        const apk = AndroidFoldableGate.build(binary, name);
        await AndroidFoldableGate.exercise(apk, name).catch(error => failures.push(error.message));
        try {
          AndroidFoldableGate.checkManifest(apk, name);
        } catch (error) {
          failures.push(error.message);
        }
      }
      assert.deepEqual(failures, [], `Android foldable checks failed on ${device}; screenshots in ${AndroidFoldableGate.evidenceFolder}`);
      console.log(`PASS: Android foldable on ${device}: folded and unfolded, the app window fills the display with no letterbox or pillarbox, the page keeps its state across fold changes, and the manifest declares resizeableActivity and the size configChanges; screenshots in ${AndroidFoldableGate.evidenceFolder}`);
    } finally {
      spawnSync(AndroidFoldableGate.adb, ['uninstall', AndroidFoldableGate.packageName], { stdio: 'ignore', timeout: 60000 });
      AndroidFoldableGate.console('unfold');
      AndroidFoldableGate.restoreErrorDialogs();
      rmSync(AndroidFoldableGate.root, { recursive: true, force: true });
    }
  }
}

await AndroidFoldableGate.main();

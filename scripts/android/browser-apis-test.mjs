#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidBrowserApisGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidBrowserApisGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'dev.tachyon.browser_apis';
  static allowButtons = [
    'com.android.permissioncontroller:id/permission_allow_foreground_only_button',
    'com.android.permissioncontroller:id/permission_allow_button',
  ];
  static manifest = {
    name: 'Browser APIs',
    start_url: '/',
    tac: {
      browserApis: ['clipboard', 'share', 'notifications', 'geolocation'],
      application: {
        id: 'dev.tachyon.browser-apis',
        version: '0.1.0',
        build: 1,
        permissions: { geolocation: 'Shows stores near you', notifications: 'Order updates' },
      },
    },
  };
  static service = `@Service
export default class ProbeService {
  /** @type {TacClipboard} */
  @Needs
  clipboard;
  /** @type {TacShare} */
  @Needs
  share;
  /** @type {TacNotifications} */
  @Needs
  notifications;
  /** @type {TacGeolocation} */
  @Needs
  geolocation;
  pasted = '';
  shared = '';
  permission = '';
  place = '';
  problem = '';

  copy() {
    this.clipboard.writeText('Copied by Tachyon');
  }

  paste() {
    this.clipboard.readText();
  }

  open() {
    this.share.open('Tachyon', 'Shared by Tachyon', 'https://example.com/');
  }

  ask() {
    this.notifications.requestPermission();
  }

  notify() {
    this.problem = this.#shown();
  }

  #shown() {
    try {
      this.notifications.show('Order 42', 'Your order is ready');
      return 'notified';
    } catch (refusal) {
      return refusal.message;
    }
  }

  locate() {
    this.geolocation.currentPosition();
  }

  /** @param {ClipboardTextRead} read */
  @OnEvent
  read(read) {
    this.pasted = read.text;
  }

  /** @param {ShareFinished} finished */
  @OnEvent
  finished(finished) {
    this.shared = 'finished';
  }

  /** @param {NotificationPermissionChanged} changed */
  @OnEvent
  changed(changed) {
    this.permission = changed.permission;
  }

  /** @param {PositionFound} found */
  @OnEvent
  found(found) {
    this.place = \`\${found.latitude.toFixed(3)}, \${found.longitude.toFixed(3)}\`;
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    this.problem = failed.message;
  }
}
`;
  static view = `<section aria-label="Browser APIs">
<button type="button" on:click="probe.copy()">Copy</button>
<button type="button" on:click="probe.paste()">Paste</button>
<button type="button" on:click="probe.open()">Share</button>
<button type="button" on:click="probe.ask()">Ask</button>
<button type="button" on:click="probe.notify()">Notify</button>
<button type="button" on:click="probe.locate()">Locate</button>
<output aria-label="Pasted">{probe.pasted}</output>
<output aria-label="Shared">{probe.shared}</output>
<output aria-label="Permission">{probe.permission}</output>
<output aria-label="Place">{probe.place}</output>
<output aria-label="Problem">{probe.problem}</output>
</section>`;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidBrowserApisGate.repo, encoding: 'utf8', timeout: 240000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidBrowserApisGate.run(AndroidBrowserApisGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static write(project, name, contents) {
    const file = path.join(project, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, contents);
  }

  static stage(project) {
    const { write, manifest, service, view } = AndroidBrowserApisGate;
    write(project, 'client/manifest.json', JSON.stringify(manifest));
    write(project, 'client/services/ProbeService.js', service);
    write(project, 'client/components/browser/probe/tac.html', view);
    write(project, 'client/pages/tac.html', '<browser-probe hydrate="load" />\n');
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidBrowserApisGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static async launch(apk) {
    const { run, adb, adbShell, packageName, pause } = AndroidBrowserApisGate;
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
    assert.match(pid, /^\d+$/, 'The browser APIs app must start');
    return pid;
  }

  static resumedActivity() {
    return AndroidBrowserApisGate.adbShell('dumpsys', 'activity', 'activities').split('\n').find(line => line.includes('topResumedActivity=')) ?? '';
  }

  static async waitFor(condition, message) {
    for (let attempt = 0; attempt < 60; attempt++) {
      if (await condition()) return;
      await AndroidBrowserApisGate.pause(250);
    }
    assert.fail(typeof message === 'function' ? await message() : message);
  }

  static output(page, name) {
    return page.getByRole('status', { name }).textContent();
  }

  static async allowPermissionPrompt() {
    const { adbShell, allowButtons, waitFor } = AndroidBrowserApisGate;
    let bounds;
    await waitFor(() => {
      adbShell('uiautomator', 'dump', '/sdcard/tachyon-permission.xml');
      const screen = adbShell('cat', '/sdcard/tachyon-permission.xml');
      bounds = allowButtons
        .map(id => screen.match(new RegExp(`resource-id="${id.replaceAll('.', '\\.')}"[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`)))
        .find(Boolean);
      return Boolean(bounds);
    }, 'Android shows its permission question');
    const [, left, top, right, bottom] = bounds.map(Number);
    adbShell('input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2)));
  }

  static async checkClipboard(page) {
    const { output, waitFor } = AndroidBrowserApisGate;
    await page.getByRole('button', { name: 'Copy' }).click();
    await page.getByRole('button', { name: 'Paste' }).click();
    await waitFor(async () => await output(page, 'Pasted') === 'Copied by Tachyon', async () => `readText answers the text writeText put on the Android clipboard: ${await output(page, 'Pasted')} ${await output(page, 'Problem')}`);
  }

  static async checkShare(page) {
    const { output, waitFor, resumedActivity, adbShell, packageName } = AndroidBrowserApisGate;
    await page.getByRole('button', { name: 'Share' }).click();
    await waitFor(() => /Chooser|Resolver/.test(resumedActivity()), () => `The Android share sheet opens: ${resumedActivity()}`);
    await waitFor(async () => await output(page, 'Shared') === 'finished', 'share.open answers ShareFinished once the share sheet opens');
    adbShell('input', 'keyevent', 'KEYCODE_BACK');
    await waitFor(() => resumedActivity().includes(packageName), () => `Closing the share sheet returns to the app: ${resumedActivity()}`);
  }

  static async checkNotifications(page) {
    const { output, waitFor, adbShell, allowPermissionPrompt } = AndroidBrowserApisGate;
    assert.equal(await page.evaluate(() => globalThis.__tachyonNotificationPermission), 'default', 'The host tells the page that nobody has answered yet');
    await page.getByRole('button', { name: 'Notify' }).click();
    assert.equal(await output(page, 'Problem'), 'Notifications are not allowed. Call requestPermission first.');
    await page.getByRole('button', { name: 'Ask' }).click();
    await allowPermissionPrompt();
    await waitFor(async () => await output(page, 'Permission') === 'granted', async () => `requestPermission answers granted after Allow: ${await output(page, 'Permission')}`);
    await page.getByRole('button', { name: 'Notify' }).click();
    await waitFor(async () => await output(page, 'Problem') === 'notified', async () => `show posts the notification: ${await output(page, 'Problem')}`);
    await waitFor(() => adbShell('dumpsys', 'notification', '--noredact').includes('Order 42'), 'Android lists the notification');
    assert.match(adbShell('dumpsys', 'notification', '--noredact'), /mName='?Order updates/, 'The channel takes its name from tac.application.permissions.notifications');
  }

  static async checkGeolocation(page) {
    const { output, waitFor, adbShell, allowPermissionPrompt } = AndroidBrowserApisGate;
    adbShell('appops', 'set', 'com.android.shell', 'android:mock_location', 'allow');
    try {
      adbShell('cmd', 'location', 'providers', 'add-test-provider', 'gps');
      adbShell('cmd', 'location', 'providers', 'set-test-provider-enabled', 'gps', 'true');
      adbShell('cmd', 'location', 'providers', 'set-test-provider-location', 'gps', '--location', '37.422,-122.084');
      await page.getByRole('button', { name: 'Locate' }).click();
      await allowPermissionPrompt();
      await waitFor(async () => await output(page, 'Place') === '37.422, -122.084', async () => `currentPosition answers the test provider's position: ${await output(page, 'Place')} ${await output(page, 'Problem')}`);
    } finally {
      spawnSync(AndroidBrowserApisGate.adb, ['shell', 'cmd', 'location', 'providers', 'remove-test-provider', 'gps'], { encoding: 'utf8', timeout: 60000 });
      spawnSync(AndroidBrowserApisGate.adb, ['shell', 'appops', 'set', 'com.android.shell', 'android:mock_location', 'default'], { encoding: 'utf8', timeout: 60000 });
    }
  }

  static async main() {
    assert.ok(AndroidBrowserApisGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const gate = AndroidBrowserApisGate;
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-browser-apis-'));
    let browser;
    let port;
    try {
      gate.stage(project);
      gate.run(gate.binary(), ['build', project, '--target', 'android']);
      const pid = await gate.launch(path.join(project, 'dist/android/BrowserAPIs/BrowserAPIs.apk'));
      port = gate.run(gate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, 'Android WebView exposes a live document');
      await page.getByRole('button', { name: 'Copy' }).waitFor();
      assert.deepEqual(await page.evaluate(() => globalThis.__tachyonBrowserApiBridge), ['clipboard', 'share', 'notifications']);
      await gate.checkClipboard(page);
      await gate.checkShare(page);
      await gate.checkNotifications(page);
      await gate.checkGeolocation(page);
      console.log('PASS: Android browser APIs: clipboard round trip, share sheet, notification question and post, and geolocation after the location question');
    } finally {
      if (browser) await browser.close();
      if (port) gate.run(gate.adb, ['forward', '--remove', `tcp:${port}`]);
      gate.adbShell('am', 'force-stop', gate.packageName);
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidBrowserApisGate.main();

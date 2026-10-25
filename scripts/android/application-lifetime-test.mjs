#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { ApplicationLifetimeFixture } from '../native/application-lifetime-fixture.mjs';

class AndroidLifetimeGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidLifetimeGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = ApplicationLifetimeFixture.androidPackage;
  static browser = undefined;
  static port = undefined;
  static autoRotate = '1';

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidLifetimeGate.repo, encoding: 'utf8', timeout: 300000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidLifetimeGate.run(AndroidLifetimeGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidLifetimeGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static build(project) {
    ApplicationLifetimeFixture.write(project);
    const result = spawnSync(AndroidLifetimeGate.binary(), ['build', project, '--target', 'android'], {
      cwd: AndroidLifetimeGate.repo, encoding: 'utf8', timeout: 900000, maxBuffer: 8 * 1024 * 1024,
    });
    assert.equal(result.status, 0, `ty build --target android: ${result.error || result.stderr || result.stdout}`);
    assert.match(result.stdout + result.stderr, /Action items call a method of an application service/, 'the build notes how Android adapts the menu actions');
    const name = ApplicationLifetimeFixture.name;
    const source = readFileSync(path.join(project, `dist/android/${name}/project/app/src/main/java/dev/tachyon/lifetime_gate/MainActivity.java`), 'utf8');
    assert.ok(source.includes('MenuRoute.action("show-running", "What\'s Running", "view", "SectionChosen", "1", KeyEvent.META_CTRL_ON)'), 'the overflow menu lists the action item');
    return path.join(project, `dist/android/${name}/${name}.apk`);
  }

  static journal() {
    const read = spawnSync(AndroidLifetimeGate.adb, ['shell', 'run-as', AndroidLifetimeGate.packageName, 'cat', 'files/lifetime.txt'], { encoding: 'utf8', timeout: 60000 });
    return ApplicationLifetimeFixture.journal(read.status === 0 ? read.stdout : '');
  }

  static telemetry() {
    const read = spawnSync(AndroidLifetimeGate.adb, ['shell', 'run-as', AndroidLifetimeGate.packageName, 'cat', `files/tachyon/${ApplicationLifetimeFixture.applicationId}.jsonl`], { encoding: 'utf8', timeout: 60000 });
    return read.status === 0 ? read.stdout : '';
  }

  static async waitFor(check, message) {
    for (let attempt = 0; attempt < 120; attempt++) {
      if (check(AndroidLifetimeGate.journal())) return AndroidLifetimeGate.journal();
      await AndroidLifetimeGate.pause(250);
    }
    assert.fail(`${message}: ${JSON.stringify(AndroidLifetimeGate.journal().lines)}`);
  }

  static pid() {
    return spawnSync(AndroidLifetimeGate.adb, ['shell', 'pidof', AndroidLifetimeGate.packageName], { encoding: 'utf8' }).stdout.trim();
  }

  static async attach() {
    const { run, adb, adbShell, pause, pid } = AndroidLifetimeGate;
    let process = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      process = pid();
      if (process && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${process}`)) break;
      await pause(500);
    }
    assert.match(process, /^\d+$/, 'the lifetime app must start');
    await AndroidLifetimeGate.detach();
    AndroidLifetimeGate.port = run(adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${process}`]);
    AndroidLifetimeGate.browser = await chromium.connectOverCDP(`http://127.0.0.1:${AndroidLifetimeGate.port}`, { noDefaults: true });
    return process;
  }

  static async detach() {
    const { run, adb } = AndroidLifetimeGate;
    if (AndroidLifetimeGate.browser) await AndroidLifetimeGate.browser.close();
    if (AndroidLifetimeGate.port) run(adb, ['forward', '--remove', `tcp:${AndroidLifetimeGate.port}`]);
    AndroidLifetimeGate.browser = undefined;
    AndroidLifetimeGate.port = undefined;
  }

  static async page(heading) {
    for (let attempt = 0; attempt < 60; attempt++) {
      for (const context of AndroidLifetimeGate.browser.contexts()) {
        for (const page of context.pages()) {
          const found = await page.getByRole('heading', { level: 1, name: heading }).count().catch(() => 0);
          if (found > 0) return page;
        }
      }
      await AndroidLifetimeGate.pause(250);
    }
    assert.fail(`No live page shows the ${heading} heading`);
  }

  static async text(page, id, expected, message) {
    for (let attempt = 0; attempt < 60; attempt++) {
      if ((await page.locator('#' + id).textContent().catch(() => '')) === expected) return;
      await AndroidLifetimeGate.pause(250);
    }
    assert.fail(`${message}: #${id} is ${await page.locator('#' + id).textContent().catch(() => 'missing')}`);
  }

  static press(key) {
    AndroidLifetimeGate.adbShell('input', 'keycombination', 'KEYCODE_CTRL_LEFT', key);
  }

  static menuItem(screen, label) {
    const ancestors = [];
    for (const tag of screen.match(/<node [^>]*?\/?>|<\/node>/g) ?? []) {
      if (tag === '</node>') {
        ancestors.pop();
        continue;
      }
      if (tag.includes(`text="${label}"`)) return `enabled="${[...ancestors, tag].every(node => node.includes('enabled="true"'))}"`;
      if (!tag.endsWith('/>')) ancestors.push(tag);
    }
    return '';
  }

  static async overflow() {
    const { adbShell, pause, run, adb, repo } = AndroidLifetimeGate;
    const read = () => {
      const dump = spawnSync(adb, ['shell', 'uiautomator', 'dump', '/sdcard/tachyon-lifetime.xml'], { cwd: repo, encoding: 'utf8', timeout: 60000 });
      return `${dump.stdout}${dump.stderr}`.includes('dumped to') ? run(adb, ['shell', 'cat', '/sdcard/tachyon-lifetime.xml']) : '';
    };
    for (let attempt = 0; attempt < 20; attempt++) {
      const node = read().split('<node ').find(line => line.includes('content-desc="More options"'));
      const bounds = node?.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/);
      if (bounds) {
        const [left, top, right, bottom] = bounds.slice(1).map(Number);
        adbShell('input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2)));
        break;
      }
      await pause(500);
    }
    for (let attempt = 0; attempt < 20; attempt++) {
      await pause(500);
      const screen = read();
      if (screen.includes(`text="What's Running"`)) {
        adbShell('input', 'keyevent', 'KEYCODE_ESCAPE');
        await pause(500);
        return screen;
      }
    }
    assert.fail('The overflow menu did not open');
  }

  static async checkTwoPagesShareOneInstance() {
    const { waitFor, page, text, press } = AndroidLifetimeGate;
    const first = await waitFor(journal => journal.visits.length >= 1, 'the home page reaches the application service');
    assert.equal(first.created.length, 1, 'the application service starts once at launch');
    const instance = first.created[0];
    assert.deepEqual(first.visits[0], ['visit', instance, '1']);
    const home = await page('Home');
    await text(home, 'instance', instance, 'the home page reads the application instance');
    press('KEYCODE_K');
    const second = await waitFor(journal => journal.visits.length >= 2, 'the status page reaches the application service');
    assert.deepEqual(second.visits[1], ['visit', instance, '2'], 'the second page talks to the same instance');
    assert.equal(second.created.length, 1, 'opening a page does not start the service again');
    const status = await page('Status');
    await text(status, 'visits', '2', 'the status page shows the shared count');
    return { instance, status };
  }

  static async checkActions(instance, status) {
    const { waitFor, text, press, overflow, menuItem, page, telemetry } = AndroidLifetimeGate;
    const enabled = await overflow();
    assert.match(menuItem(enabled, "What's Running"), /enabled="true"/, 'the action is enabled while the page listens');
    press('KEYCODE_1');
    await waitFor(journal => journal.lines.includes('heard running') && journal.lines.includes('forwarded'), 'Ctrl+1 forwards SectionChosen to the page');
    await text(status, 'section', 'running', 'the page shows the forwarded section without a reload');
    await text(status, 'visits', '2', 'the page kept its state');
    press('KEYCODE_2');
    await waitFor(journal => journal.lines.includes(`refreshed ${instance}`), 'Ctrl+2 runs the action without an event');
    press('KEYCODE_3');
    await page('Guide');
    const disabled = await overflow();
    assert.match(menuItem(disabled, "What's Running"), /enabled="false"/, 'the action is disabled on a page without a listener');
    assert.match(menuItem(disabled, 'Refresh'), /enabled="true"/, 'an action without an event stays enabled');
    const forwarded = AndroidLifetimeGate.journal().lines.filter(line => line === 'forwarded' || line === 'unheard').length;
    press('KEYCODE_1');
    await AndroidLifetimeGate.pause(1500);
    assert.equal(AndroidLifetimeGate.journal().lines.filter(line => line === 'forwarded' || line === 'unheard').length, forwarded, 'a disabled shortcut does not call the service');
    assert.match(telemetry(), /"event":"menu.refused"/, 'the host records the refused shortcut');
    assert.match(telemetry(), /"event":"menu.action"/, 'the host records the performed actions');
  }

  static async checkRotationKeepsTheInstance(instance, processId) {
    const { adbShell, waitFor, pause, attach, pid, press } = AndroidLifetimeGate;
    const before = AndroidLifetimeGate.journal().visits.length;
    press('KEYCODE_K');
    await waitFor(journal => journal.visits.length > before, 'Ctrl+K opens the status page again');
    const settled = AndroidLifetimeGate.journal().visits.length;
    adbShell('settings', 'put', 'system', 'accelerometer_rotation', '0');
    adbShell('settings', 'put', 'system', 'user_rotation', '1');
    try {
      const rotated = await waitFor(journal => journal.visits.length > settled, 'the recreated activity opens a page that reaches the service');
      assert.equal(rotated.created.length, 1, 'a rotation keeps the application service');
      assert.equal(rotated.visits.at(-1)[1], instance, 'the new activity talks to the same instance');
      assert.equal(pid(), processId, 'the rotation stayed in the same process');
      adbShell('settings', 'put', 'system', 'user_rotation', '0');
      const restored = await waitFor(journal => journal.visits.length > rotated.visits.length, 'rotating back recreates the activity again');
      assert.equal(restored.created.length, 1, 'rotating back keeps the application service');
      await pause(500);
      await attach();
    } finally {
      adbShell('settings', 'put', 'system', 'user_rotation', '0');
    }
  }

  static async checkBackStopsAndRelaunchStartsAgain(instance, processId) {
    const { adbShell, waitFor, packageName, pid, detach } = AndroidLifetimeGate;
    await detach();
    const open = AndroidLifetimeGate.journal().visits.at(-1)[2];
    adbShell('input', 'keyevent', 'KEYCODE_BACK');
    const stopped = await waitFor(journal => journal.lines.includes(`stopped ${instance}`), 'Back finishes the activity and stops the application service');
    assert.equal(stopped.lines.filter(line => line.startsWith('stopped ')).length, 1, 'the service stops once');
    const left = stopped.lines.lastIndexOf(`left ${instance} ${open}`);
    assert.ok(left >= 0 && left < stopped.lines.indexOf(`stopped ${instance}`), `onDestroy disposes the open page's SectionService (visit ${open}) before the application service stops: ${JSON.stringify(stopped.lines.slice(-6))}`);
    const kept = pid();
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    const again = await waitFor(journal => journal.created.length >= 2, 'a new launch starts the application service again');
    assert.notEqual(again.created[1], instance, 'the new launch gets a new instance');
    return { sameProcess: kept === processId && again.created[1].startsWith(processId + '-') };
  }

  static async main() {
    const gate = AndroidLifetimeGate;
    assert.ok(gate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-lifetime-'));
    try {
      const apk = gate.build(project);
      gate.run(gate.adb, ['wait-for-device']);
      gate.autoRotate = gate.adbShell('settings', 'get', 'system', 'accelerometer_rotation');
      gate.run(gate.adb, ['install', '-r', apk]);
      gate.adbShell('am', 'force-stop', gate.packageName);
      gate.adbShell('pm', 'clear', gate.packageName);
      gate.adbShell('am', 'start', '-W', '-n', `${gate.packageName}/.MainActivity`);
      const processId = await gate.attach();
      const { instance, status } = await gate.checkTwoPagesShareOneInstance();
      await gate.checkActions(instance, status);
      await gate.checkRotationKeepsTheInstance(instance, processId);
      const { sameProcess } = await gate.checkBackStopsAndRelaunchStartsAgain(instance, processId);
      console.log(`PASS: Android application lifetime: one LifetimeService started at launch, served two pages and a rotation, stopped once on Back after onDestroy disposed the open page's companion, and started again on the next launch${sameProcess ? ' in the same process' : ''}`);
      console.log('PASS: Android menu actions: Ctrl+1 and Ctrl+2 called the application service, the page showed the forwarded section without a reload, and the overflow item was disabled on a page without a listener');
    } finally {
      await gate.detach().catch(() => {});
      spawnSync(gate.adb, ['shell', 'settings', 'put', 'system', 'user_rotation', '0'], { timeout: 60000 });
      spawnSync(gate.adb, ['shell', 'settings', 'put', 'system', 'accelerometer_rotation', gate.autoRotate], { timeout: 60000 });
      spawnSync(gate.adb, ['shell', 'am', 'force-stop', gate.packageName], { timeout: 60000 });
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidLifetimeGate.main();

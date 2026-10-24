#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class AndroidWindowControlsGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static feature = path.join(AndroidWindowControlsGate.repo, 'website/content/features/window-controls');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  static adb = path.join(AndroidWindowControlsGate.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static packageName = 'com.example.catalog';
  static refusedControls = ['maximize', 'fullscreen', 'close'];
  static allControls = ['minimize', 'resize', 'title', 'maximize', 'fullscreen', 'close'];
  static grantService = `@Service
@NeedsAll
class WindowGrantService {
    private val windowControls: WindowControls
    var problem: String = ""

    fun maximize() {
        keepProblem { windowControls.maximize() }
    }

    fun fullscreen() {
        keepProblem { windowControls.fullscreen() }
    }

    private fun keepProblem(call: () -> Unit) {
        try { call(); problem = "acted" }
        catch (refusal: IllegalStateException) { problem = refusal.message.orEmpty() }
    }
}
`;
  static probeService = `@Service
@NeedsAll
class WindowProbeService {
    private val windowControls: WindowControls
    var refusals: List<String> = emptyList()
    var grants: List<String> = emptyList()

    fun probe() {
        grants = listOf("minimize", "resize", "title", "maximize", "fullscreen", "close").filter { windowControls.granted(it) }
        refusals = listOf(
            refusal { windowControls.maximize() },
            refusal { windowControls.fullscreen() },
            refusal { windowControls.close() },
        )
    }

    private fun refusal(call: () -> Unit): String =
        try { call(); "acted" }
        catch (refused: IllegalStateException) { refused.message.orEmpty() }
}
`;

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: AndroidWindowControlsGate.repo, encoding: 'utf8', timeout: 240000, maxBuffer: 8 * 1024 * 1024, ...options });
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`);
    return result.stdout.trim();
  }

  static adbShell(...args) {
    return AndroidWindowControlsGate.run(AndroidWindowControlsGate.adb, ['shell', ...args]);
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds));
  }

  static stage(project) {
    const feature = AndroidWindowControlsGate.feature;
    for (const entry of ['components', 'manifest.json']) cpSync(path.join(feature, 'client', entry), path.join(project, 'client', entry), { recursive: true });
    mkdirSync(path.join(project, 'client/services'), { recursive: true });
    cpSync(path.join(feature, 'native/services/ReportService.kt'), path.join(project, 'client/services/ReportService.kt'));
    writeFileSync(path.join(project, 'client/services/WindowProbeService.kt'), AndroidWindowControlsGate.probeService);
    mkdirSync(path.join(project, 'client/components/window/probe'), { recursive: true });
    writeFileSync(path.join(project, 'client/components/window/probe/tac.html'), '<section aria-label="Window probe"><button type="button" on:click="windowProbe.probe()">Probe</button><output aria-label="Grants">{windowProbe.grants.join(",")}</output><output aria-label="Refusals">{windowProbe.refusals.join("|")}</output></section>');
    mkdirSync(path.join(project, 'client/pages'), { recursive: true });
    writeFileSync(path.join(project, 'client/pages/tac.html'), '<report-window hydrate="load" />\n<window-probe />\n');
  }

  static stageGranted(project) {
    const { feature, allControls, grantService } = AndroidWindowControlsGate;
    AndroidWindowControlsGate.stage(project);
    const manifest = JSON.parse(readFileSync(path.join(feature, 'client/manifest.json'), 'utf8'));
    manifest.tac.application.window.controls = allControls;
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest));
    writeFileSync(path.join(project, 'client/services/WindowGrantService.kt'), grantService);
    mkdirSync(path.join(project, 'client/components/window/grant'), { recursive: true });
    writeFileSync(path.join(project, 'client/components/window/grant/tac.html'), '<section aria-label="Window grant"><button type="button" on:click="windowGrant.maximize()">Maximize</button><button type="button" on:click="windowGrant.fullscreen()">Fullscreen</button><output aria-label="Grant problem">{windowGrant.problem}</output></section>');
    writeFileSync(path.join(project, 'client/pages/tac.html'), '<report-window hydrate="load" />\n<window-grant />\n');
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN;
    const run = AndroidWindowControlsGate.run;
    run('cargo', ['build', '--locked', '--bin', 'ty']);
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty');
  }

  static async launch(apk) {
    const { run, adb, adbShell, packageName, pause } = AndroidWindowControlsGate;
    run(adb, ['wait-for-device']);
    run(adb, ['install', '-r', apk]);
    adbShell('am', 'force-stop', packageName);
    adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`);
    let pid = '';
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = adbShell('pidof', packageName);
      if (pid && adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)) break;
      await pause(500);
    }
    assert.match(pid, /^\d+$/, 'The window controls app must start');
    return pid;
  }

  static resumedActivity() {
    return AndroidWindowControlsGate.adbShell('dumpsys', 'activity', 'activities').split('\n').find(line => line.includes('topResumedActivity=')) ?? '';
  }

  static applicationWindowTitle() {
    const { adb, adbShell, repo } = AndroidWindowControlsGate;
    const dump = spawnSync(adb, ['shell', 'uiautomator', 'dump', '--windows', '/sdcard/tachyon-windows.xml'], { cwd: repo, encoding: 'utf8', timeout: 60000 });
    if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return '';
    const windows = adbShell('cat', '/sdcard/tachyon-windows.xml');
    return windows.match(/<window [^>]*title="([^"]*)"[^>]*type="TYPE_APPLICATION"/)?.[1] ?? '';
  }

  static pageSize(page) {
    return page.evaluate(() => [innerWidth, innerHeight]);
  }

  static async waitFor(condition, message) {
    for (let attempt = 0; attempt < 40; attempt++) {
      if (await condition()) return;
      await AndroidWindowControlsGate.pause(250);
    }
    assert.fail(message);
  }

  static async checkRefusals(page) {
    const { refusedControls, resumedActivity, pageSize, packageName } = AndroidWindowControlsGate;
    const size = await pageSize(page);
    for (const control of refusedControls) {
      const answer = await page.evaluate(async control => JSON.parse(await globalThis.__tachyonNativeHostCall(`window.${control}`, '{}')), control);
      assert.deepEqual(answer, { ok: false, error: 'Window control is not granted.' }, `The page cannot run the ungranted ${control} control`);
    }
    await page.getByRole('button', { name: 'Probe' }).click();
    await page.getByRole('status', { name: 'Grants' }).filter({ hasText: 'minimize,resize,title' }).waitFor();
    const refusals = await page.getByRole('status', { name: 'Refusals' }).textContent();
    assert.deepEqual(refusals.split('|'), refusedControls.map(control =>
      `The ${control} window control is not granted. Add "${control}" to tac.application.window.controls in client/manifest.json.`));
    assert.ok(resumedActivity().includes(packageName), `Refused close leaves the app resumed: ${resumedActivity()}`);
    assert.deepEqual(await pageSize(page), size, 'Refused maximize and fullscreen leave the window size as it was');
  }

  static async checkGranted(page) {
    const { applicationWindowTitle, resumedActivity, pageSize, waitFor, packageName } = AndroidWindowControlsGate;
    assert.notEqual(applicationWindowTitle(), 'Quarterly report');
    await page.getByRole('button', { name: 'Rename' }).click();
    await waitFor(() => applicationWindowTitle() === 'Quarterly report', `Rename sets the Android window title: ${applicationWindowTitle()}`);
    await page.getByRole('button', { name: 'Resize' }).click();
    await waitFor(async () => JSON.stringify(await pageSize(page)) === '[1280,800]', `Resize makes the window 1280 by 800: ${await pageSize(page)}`);
    assert.equal(await page.locator('main p').textContent(), '', 'Granted controls report no problem');
    await page.getByRole('button', { name: 'Minimise' }).click();
    await waitFor(() => !resumedActivity().includes(packageName), `Minimise moves the app behind the launcher: ${resumedActivity()}`);
  }

  static systemBarsShown() {
    const source = AndroidWindowControlsGate.adbShell('dumpsys', 'window').match(/InsetsSource id=\S+ type=statusBars [^\n]*visible=(true|false)/);
    return source ? { true: 'shown', false: 'hidden' }[source[1]] : 'unknown';
  }

  static hostCall(page, control) {
    return page.evaluate(async control => JSON.parse(await globalThis.__tachyonNativeHostCall(`window.${control}`, '{}')), control);
  }

  static activityRecords() {
    const { adbShell, packageName } = AndroidWindowControlsGate;
    return adbShell('dumpsys', 'activity', 'activities').split('\n').filter(line => line.includes(`${packageName}/.MainActivity`) && /ActivityRecord\{/.test(line));
  }

  static async checkGrantedOnActivity(page) {
    const { pageSize, waitFor, hostCall, systemBarsShown, resumedActivity, activityRecords, packageName } = AndroidWindowControlsGate;
    const problem = () => page.getByRole('status', { name: 'Grant problem' }).textContent();
    const full = await pageSize(page);
    await page.getByRole('button', { name: 'Resize' }).click();
    await waitFor(async () => JSON.stringify(await pageSize(page)) === '[1280,800]', `Resize makes the window 1280 by 800 before maximize: ${await pageSize(page)}`);
    await page.getByRole('button', { name: 'Maximize' }).click();
    await waitFor(async () => JSON.stringify(await pageSize(page)) === JSON.stringify(full), `A granted maximize from the Kotlin service fills the screen again (${full}): ${await pageSize(page)}`);
    await waitFor(async () => await problem() === 'acted', async () => `The granted maximize reports no problem: ${await problem()}`);
    const before = { bars: systemBarsShown(), size: await pageSize(page) };
    assert.equal(before.bars, 'shown', 'The system bars show before fullscreen');
    await page.getByRole('button', { name: 'Fullscreen' }).click();
    await waitFor(async () => systemBarsShown() === 'hidden', `A granted fullscreen from the Kotlin service hides the system bars: ${systemBarsShown()} ${await pageSize(page)} (before ${JSON.stringify(before)})`);
    const fullscreen = await pageSize(page);
    assert.ok(fullscreen[1] >= before.size[1], `Fullscreen does not shrink the page: ${before.size} -> ${fullscreen}`);
    assert.deepEqual(await hostCall(page, 'fullscreen'), { ok: true }, 'A granted fullscreen from the page answers ok');
    await waitFor(async () => systemBarsShown() === 'shown', `A second fullscreen from the page shows the system bars again: ${systemBarsShown()}`);
    const [closed, afterClose] = await page.evaluate(async () => Promise.all(['window.close', 'window.maximize'].map(async capability => JSON.parse(await globalThis.__tachyonNativeHostCall(capability, '{}')))));
    assert.deepEqual(closed, { ok: true }, 'A granted close from the page answers ok');
    assert.deepEqual(afterClose, { ok: false, error: 'Native window is unavailable.' }, 'A granted control that reaches the host after close started is refused by the closing activity');
    await waitFor(() => !resumedActivity().includes(packageName) && activityRecords().length === 0, `A granted close finishes the activity: ${resumedActivity()} ${activityRecords().join(' ')}`);
    return { fullscreen, before };
  }

  static async main() {
    assert.ok(AndroidWindowControlsGate.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required');
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-window-'));
    let browser;
    let port;
    try {
      AndroidWindowControlsGate.stage(project);
      AndroidWindowControlsGate.run(AndroidWindowControlsGate.binary(), ['build', project, '--target', 'android']);
      const pid = await AndroidWindowControlsGate.launch(path.join(project, 'dist/android/Catalog/Catalog.apk'));
      port = AndroidWindowControlsGate.run(AndroidWindowControlsGate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, 'Android WebView exposes a live document');
      await page.getByRole('button', { name: 'Rename' }).waitFor();
      await AndroidWindowControlsGate.checkRefusals(page);
      await AndroidWindowControlsGate.checkGranted(page);
      console.log('PASS: Android window controls: granted minimize, resize and title act; maximize, fullscreen and close are refused');
    } finally {
      if (browser) await browser.close();
      if (port) AndroidWindowControlsGate.run(AndroidWindowControlsGate.adb, ['forward', '--remove', `tcp:${port}`]);
      AndroidWindowControlsGate.adbShell('am', 'force-stop', AndroidWindowControlsGate.packageName);
      rmSync(project, { recursive: true, force: true });
    }
    await AndroidWindowControlsGate.mainGranted();
  }

  static async mainGranted() {
    const gate = AndroidWindowControlsGate;
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-window-granted-'));
    let browser;
    let port;
    try {
      gate.stageGranted(project);
      gate.run(gate.binary(), ['build', project, '--target', 'android']);
      const pid = await gate.launch(path.join(project, 'dist/android/Catalog/Catalog.apk'));
      port = gate.run(gate.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`]);
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true });
      const page = browser.contexts()[0].pages()[0];
      assert.ok(page, 'Android WebView exposes a live document');
      await page.getByRole('button', { name: 'Maximize' }).waitFor();
      const { before, fullscreen } = await gate.checkGrantedOnActivity(page);
      console.log(`PASS: Android window controls: granted maximize and fullscreen act from a Kotlin service, fullscreen from the page hides and shows the system bars (page ${before.size} -> ${fullscreen}), close from the page finishes the activity, and a control that arrives after close started is refused`);
    } finally {
      if (browser) await browser.close().catch(() => {});
      if (port) gate.run(gate.adb, ['forward', '--remove', `tcp:${port}`]);
      gate.adbShell('am', 'force-stop', gate.packageName);
      rmSync(project, { recursive: true, force: true });
    }
  }
}

await AndroidWindowControlsGate.main();

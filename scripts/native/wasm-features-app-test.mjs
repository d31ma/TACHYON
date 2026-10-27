#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { WasmFeaturesFixture } from './wasm-features-fixture.mjs'
import { IosWasmApp } from './wasm-features-ios.mjs'

class LinuxWasmApp {
  static screen = '1280x1000x24'
  static waitSeconds = 240
  static lines = ['ok - ', 'not ok - ']

  /** @param {string} project @param {string} application */
  static bundle(project, application) {
    return path.join(project, 'dist/linux', application, 'resources/WebBundle')
  }

  /** @param {string[]} gestures */
  static steps(gestures) {
    const clicks = gestures.flatMap((label) => [['wait', [`Probe waiting for ${label}`]], ['pointer', label]])
    return [...clicks, ['verdict', { passed: WasmFeaturesFixture.passed, failed: WasmFeaturesFixture.failed, lines: this.lines, seconds: this.waitSeconds }]]
  }

  /** @param {string} project @param {{ application: string, gestures: string[] }} staged */
  static async drive(project, staged) {
    const app = path.join(project, 'dist/linux', staged.application, 'bin', staged.application)
    const script = `set -euo pipefail
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
timeout ${this.waitSeconds + 120}s python3 "$PROBE" "$STEPS"`
    const env = {
      ...process.env,
      APP: app,
      PROBE: path.join(WasmFeaturesAppTest.repository, 'scripts/linux/page-steps-probe.py'),
      STEPS: JSON.stringify(this.steps(staged.gestures)),
      TAC_PROBE_APPLICATION: staged.application,
      XDG_STATE_HOME: path.join(project, 'state'),
      XDG_DATA_HOME: path.join(project, 'data'),
      XDG_CACHE_HOME: path.join(project, 'cache'),
      GDK_BACKEND: 'x11',
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const output = await WasmFeaturesAppTest.run('xvfb-run', ['--auto-servernum', `--server-args=-screen 0 ${this.screen}`, 'dbus-run-session', '--', 'bash', '-c', script], { env, timeoutMilliseconds: (this.waitSeconds + 180) * 1000 })
    return output.split('\n').filter((line) => /^(OK|LOADED|REPORT|PASS)/.test(line))
  }

  /** @param {string} project @param {{ application: string, gestures: string[] }} staged */
  static async run(project, staged) {
    await WasmFeaturesAppTest.run(WasmFeaturesAppTest.binary, ['build', project, '--target', 'linux'], { timeoutMilliseconds: 1800000 })
    const modules = WasmFeaturesFixture.assertWebAssembly(path.join(project, 'dist/linux'), this.bundle(project, staged.application))
    const lines = await this.drive(project, staged)
    return { modules, lines }
  }
}

class MacOsWasmApp {
  static waitSeconds = 240
  static gestureSeconds = 180
  static escapeSeconds = 60
  static lines = ['ok - ', 'not ok - ']
  static gesture = process.env.TAC_MACOS_GESTURE ?? 'press'
  static allowNotifications = process.env.TAC_MACOS_NOTIFY_ALLOW !== '0'
  static after = { 'Ask to notify': 'allow', 'Share a link': 'escape' }
  static websiteData = ['WebKit', 'Caches', 'HTTPStorages']
  static probe = ''

  /** @param {string} root */
  static async prepare(root) {
    assert.equal(process.platform, 'darwin', 'the macOS in-app WebAssembly features gate needs macOS')
    assert.ok(['press', 'click'].includes(this.gesture), `TAC_MACOS_GESTURE is press (an Accessibility press) or click (a real pointer click), not ${this.gesture}`)
    this.probe = path.join(root, 'page-checks-macos-probe')
    const source = path.join(WasmFeaturesAppTest.repository, 'scripts/native/page-checks-macos-probe.swift')
    await WasmFeaturesAppTest.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit', source, '-o', this.probe], { timeoutMilliseconds: 600000 })
  }

  /** @param {string} id @param {string} action @param {string[]} values */
  static async step(id, action, ...values) {
    const seconds = Number(values[0]) || 60
    return promisify(execFile)(this.probe, [id, action, ...values], { timeout: (seconds + 120) * 1000 })
  }

  /** @param {string} id */
  static async verdict(id) {
    const values = [String(this.waitSeconds), WasmFeaturesFixture.passed, WasmFeaturesFixture.failed, ...this.lines]
    const answer = await this.step(id, 'verdict', ...values).then((result) => ({ ...result, passed: true }), (error) => ({ stdout: error.stdout ?? '', stderr: error.stderr ?? String(error), passed: false }))
    const lines = answer.stdout.split('\n').filter(Boolean)
    assert.ok(answer.passed, `the in-app probe did not pass:\n${lines.join('\n')}\n${answer.stderr}`)
    return lines
  }

  /** @param {string} id @param {string} label */
  static async follow(id, label) {
    const next = this.after[label]
    if (next === 'allow' && this.allowNotifications) void this.step('-', 'allow', 'Allow').catch(() => undefined)
    if (next === 'escape') await this.step(id, 'escape', String(this.escapeSeconds), WasmFeaturesFixture.passed, WasmFeaturesFixture.failed)
  }

  /** @param {string} app @param {{ id: string, gestures: string[] }} staged */
  static async drive(app, staged) {
    await promisify(execFile)('/usr/bin/open', ['-n', app])
    try {
      for (const label of staged.gestures) {
        await this.step(staged.id, 'within', String(this.gestureSeconds), `Probe waiting for ${label}`)
        await this.step(staged.id, this.gesture, label)
        await this.follow(staged.id, label)
      }
      return await this.verdict(staged.id)
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(app, 'Contents/MacOS')])
      for (const folder of this.websiteData) rmSync(path.join(homedir(), 'Library', folder, staged.id), { recursive: true, force: true })
    }
  }

  /** @param {string} project @param {{ application: string, id: string, gestures: string[] }} staged */
  static async run(project, staged) {
    await WasmFeaturesAppTest.run(WasmFeaturesAppTest.binary, ['build', project, '--target', 'macos'], { timeoutMilliseconds: 1800000 })
    const output = path.join(project, 'dist/macos')
    const bundle = readdirSync(output).find((name) => name.endsWith('.app'))
    assert.ok(bundle, `no .app in ${output}`)
    const app = path.join(output, bundle)
    const modules = WasmFeaturesFixture.assertWebAssembly(output, path.join(app, 'Contents/Resources/WebBundle'))
    const lines = await this.drive(app, staged)
    return { modules, lines }
  }
}

class AndroidWasmApp {
  static waitSeconds = 300
  static promptSeconds = 60
  static pollMilliseconds = 500
  static origin = 'https://appassets.tachyon.local'
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
  static adbPath = path.join(AndroidWasmApp.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  static evidence = process.env.TAC_ANDROID_WASM_EVIDENCE ?? fileURLToPath(new URL('../../target/android-wasm-features-evidence', import.meta.url))
  static allowButtons = ['com.android.permissioncontroller:id/permission_allow_button', 'com.android.permissioncontroller:id/permission_allow_foreground_only_button']
  static shareSheet = /Chooser|Resolver/
  static after = { 'Ask to notify': 'allow', 'Share a link': 'back' }

  /** @param {string[]} args @param {number} [timeout] */
  static adb(args, timeout = 120000) {
    const result = spawnSync(this.adbPath, args, { encoding: 'utf8', timeout, maxBuffer: 64 * 1024 * 1024 })
    assert.equal(result.status, 0, `adb ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  /** @param {string[]} args */
  static attempt(args) {
    return spawnSync(this.adbPath, args, { encoding: 'utf8', timeout: 60000, maxBuffer: 64 * 1024 * 1024 }).stdout?.trim() ?? ''
  }

  static async prepare() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required for the Android in-app WebAssembly features gate')
    this.adb(['wait-for-device'])
    mkdirSync(this.evidence, { recursive: true })
    const webView = this.adb(['shell', 'dumpsys', 'webviewupdate']).match(/Current WebView package \(name, version\): \((.*)\)/)?.[1] ?? 'unknown'
    console.log(`==> Android ${this.adb(['shell', 'getprop', 'ro.build.version.release'])} (${this.adb(['shell', 'getprop', 'ro.product.cpu.abi'])}), System WebView ${webView}`)
  }

  /** @param {string} apk @param {string} packageName */
  static async launch(apk, packageName) {
    this.attempt(['uninstall', packageName])
    this.adb(['install', '-r', apk], 300000)
    this.adb(['shell', 'am', 'start', '-W', '-n', `${packageName}/.MainActivity`])
    let pid = ''
    for (let attempt = 0; attempt < 120 && !(pid && this.adb(['shell', 'cat', '/proc/net/unix']).includes(`webview_devtools_remote_${pid}`)); attempt++) {
      await sleep(this.pollMilliseconds)
      pid = this.attempt(['shell', 'pidof', packageName])
    }
    assert.match(pid, /^\d+$/, `${packageName} must start`)
    return pid
  }

  static resumed() {
    return this.adb(['shell', 'dumpsys', 'activity', 'activities']).split('\n').find((line) => line.includes('topResumedActivity=')) ?? ''
  }

  /** @param {string[]} ids */
  static tapAny(ids) {
    this.attempt(['shell', 'uiautomator', 'dump', '/sdcard/tachyon-wasm-screen.xml'])
    const shown = this.attempt(['shell', 'cat', '/sdcard/tachyon-wasm-screen.xml'])
    const bounds = ids.map((id) => shown.match(new RegExp(`resource-id="${id.replaceAll('.', '\\.')}"[^>]*bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`))).find(Boolean)
    if (!bounds) return false
    const [left, top, right, bottom] = bounds.slice(1).map(Number)
    this.adb(['shell', 'input', 'tap', String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2))])
    return true
  }

  /** @param {import('playwright').Page} page */
  static lines(page) {
    return page.evaluate(() => [...document.querySelectorAll('#wasm-probe-phase, .wasm-probe-line')].map((node) => node.textContent.trim()).filter(Boolean))
  }

  /** @param {import('playwright').Page} page @param {string[]} phases */
  static async phase(page, phases) {
    const reached = await page.waitForFunction((wanted) => wanted.find((text) => document.querySelector('#wasm-probe-phase')?.textContent === text), phases, { timeout: this.waitSeconds * 1000 })
    return reached.jsonValue()
  }

  /** @param {import('playwright').Page} page @param {string} what @param {() => unknown} check */
  static async until(page, what, check) {
    const deadline = Date.now() + this.promptSeconds * 1000
    while (Date.now() < deadline) {
      if (await check()) return
      await sleep(this.pollMilliseconds)
    }
    assert.fail(`timed out waiting for ${what}; the probe shows:\n${(await this.lines(page)).join('\n')}`)
  }

  /** @param {import('playwright').Page} page @param {string} label @param {string} packageName */
  static async follow(page, label, packageName) {
    const heard = (text) => async () => (await this.lines(page)).some((line) => line.includes(text))
    if (this.after[label] === 'allow') await this.until(page, 'the Android notification question', async () => this.tapAny(this.allowButtons) || await heard('NotificationPermissionChanged')())
    if (this.after[label] !== 'back') return
    await this.until(page, 'the Android share sheet', async () => this.shareSheet.test(this.resumed()) || await heard('Share answered')())
    if (!this.shareSheet.test(this.resumed())) return
    this.adb(['shell', 'input', 'keyevent', 'KEYCODE_BACK'])
    await this.until(page, 'the app to come back from the share sheet', () => this.resumed().includes(packageName))
  }

  /** @param {string} name */
  static screenshot(name) {
    const result = spawnSync(this.adbPath, ['exec-out', 'screencap', '-p'], { maxBuffer: 64 * 1024 * 1024, timeout: 60000 })
    if (result.status === 0) writeFileSync(path.join(this.evidence, `${name}.png`), result.stdout)
  }

  /** @param {string} apk @param {{ application: string, id: string, gestures: string[] }} staged */
  static async drive(apk, staged) {
    const { chromium } = await import('playwright')
    const packageName = staged.id.split('.').map((segment) => segment.replace(/[^A-Za-z0-9]/g, '_')).join('.')
    const { passed, failed } = WasmFeaturesFixture
    let browser
    let port
    try {
      const pid = await this.launch(apk, packageName)
      port = this.adb(['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`])
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true })
      const page = browser.contexts()[0].pages()[0]
      assert.ok(page, 'the Android WebView exposes a live document')
      for (const label of staged.gestures) {
        const waiting = `Probe waiting for ${label}`
        if (await this.phase(page, [waiting, passed, failed]) !== waiting) break
        await page.getByRole('button', { name: label, exact: true }).click()
        await this.follow(page, label, packageName)
      }
      const phase = await this.phase(page, [passed, failed])
      const lines = await this.lines(page)
      this.screenshot(staged.application)
      assert.equal(phase, passed, `the in-app probe did not pass:\n${lines.join('\n')}`)
      const pageOrigin = await page.evaluate(() => location.origin)
      const loads = await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => entry.name).filter((name) => name.endsWith('.wasm')))
      assert.equal(pageOrigin, this.origin, 'the page runs on the app bundle origin')
      assert.ok(loads.length, 'the page loaded WebAssembly')
      for (const load of loads) assert.ok(load.startsWith(`${this.origin}/`), `WebAssembly loads from the app bundle: ${load}`)
      return [...lines, `REPORT ${loads.length} WebAssembly loads from ${this.origin} in process ${pid}`]
    } finally {
      if (browser) await browser.close().catch(() => undefined)
      if (port) this.attempt(['forward', '--remove', `tcp:${port}`])
      this.attempt(['shell', 'am', 'force-stop', packageName])
      this.attempt(['uninstall', packageName])
    }
  }

  /** @param {string} project @param {{ application: string, id: string, gestures: string[], needsYon: boolean }} staged */
  static async run(project, staged) {
    await WasmFeaturesAppTest.run(WasmFeaturesAppTest.binary, ['build', project, '--target', 'android'], { timeoutMilliseconds: 1800000 })
    const output = path.join(project, 'dist/android')
    const apk = path.join(output, staged.application, `${staged.application}.apk`)
    const unpacked = path.join(project, 'apk')
    await WasmFeaturesAppTest.run('unzip', ['-q', '-o', apk, 'assets/WebBundle/*', '-d', unpacked], { timeoutMilliseconds: 300000 })
    const modules = WasmFeaturesFixture.assertWebAssembly(output, path.join(unpacked, 'assets/WebBundle'))
    const yon = staged.needsYon ? new URL(JSON.parse(readFileSync(path.join(project, 'client/manifest.json'), 'utf8')).tac.yon.origin).port : ''
    if (yon) this.adb(['reverse', `tcp:${yon}`, `tcp:${yon}`])
    try {
      return { modules, lines: await this.drive(apk, staged) }
    } finally {
      if (yon) this.attempt(['reverse', '--remove', `tcp:${yon}`])
    }
  }
}

class WasmFeaturesAppTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'wasm-features-app-'))
  static hosts = { linux: LinuxWasmApp, macos: MacOsWasmApp, android: AndroidWasmApp, ios: IosWasmApp }
  static target = WasmFeaturesAppTest.option('target', 'linux')
  static checks = WasmFeaturesAppTest.option('check', Object.keys(WasmFeaturesFixture.checks).join(',')).split(',')
  static languages = (process.env.TAC_WASM_FEATURE_LANGUAGES ?? '').split(',').filter(Boolean)
  static gestures = process.env.TAC_WASM_FEATURE_GESTURES !== 'false'
  static receipt = process.env.TAC_WASM_FEATURES_RECEIPT

  /** @param {string} name @param {string} fallback */
  static option(name, fallback) {
    return process.argv.find((argument) => argument.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback
  }

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} options */
  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address()
        server.close(() => resolve(port))
      })
    })
  }

  /** @param {{ check: string, language: string }} item */
  static async check({ check, language }) {
    const host = this.hosts[this.target]
    const project = path.join(this.root, `${check}-${language}`)
    const port = await this.freePort()
    const staged = WasmFeaturesFixture.stage(project, check, language, { yonOrigin: `http://127.0.0.1:${port}`, gestures: this.gestures, recorder: host.recorder })
    const started = Date.now()
    const serve = ['serve', project, '--host', '127.0.0.1', '--port', String(port), '--out-dir', 'serve-dist', '--no-watch', '--no-self-restart']
    const answer = staged.needsYon
      ? await withServer(this.binary, serve, () => host.run(project, staged), { readinessTimeoutMilliseconds: 600000, timeoutMilliseconds: 3600000 })
      : await host.run(project, staged)
    return { ...answer, seconds: Math.round((Date.now() - started) / 1000) }
  }

  /** @param {Array<{ check: string, language: string }>} plan @param {object[]} results */
  static async checkAll(plan, results) {
    for (const item of plan) {
      const label = `${this.target} ${item.check} ${item.language}`
      try {
        const answer = await this.check(item)
        console.log(`==> PASS ${label} (${answer.seconds} s, ${answer.modules.length} WebAssembly modules)\n${answer.lines.join('\n')}`)
        results.push({ ...item, status: 'pass', seconds: answer.seconds, lines: answer.lines })
      } catch (error) {
        console.log(`==> FAIL ${label}\n${String(error?.message ?? error).slice(-6000)}`)
        results.push({ ...item, status: 'fail', error: String(error?.message ?? error).slice(-6000) })
      }
    }
  }

  static async main() {
    const host = this.hosts[this.target]
    assert.ok(host, `no in-app driver for ${this.target} yet; the drivers are ${Object.keys(this.hosts).join(', ')}`)
    const plan = WasmFeaturesFixture.plan(this.target, this.checks, this.languages)
    assert.ok(plan.length, 'nothing to run: no check has a WebAssembly language on this target')
    const results = []
    await host.prepare?.(this.root)
    try {
      await this.checkAll(plan, results)
    } finally {
      host.close?.()
    }
    if (this.receipt) {
      mkdirSync(path.dirname(path.resolve(this.receipt)), { recursive: true })
      writeFileSync(this.receipt, `${JSON.stringify({ target: this.target, results }, null, 2)}\n`)
    }
    console.table(results.map(({ check, language, status, seconds }) => ({ check, language, status, seconds })))
    if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    const failed = results.filter((result) => result.status !== 'pass')
    assert.deepEqual(failed.map(({ check, language }) => `${check} ${language}`), [], 'every in-app WebAssembly feature check passed')
    console.log(`PASS: ${results.length} WebAssembly feature checks ran inside the packaged ${this.target} app: ${results.map(({ check, language }) => `${check} ${language}`).join(', ')}`)
  }
}

await WasmFeaturesAppTest.main()

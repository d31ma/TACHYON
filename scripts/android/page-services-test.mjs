#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { PageServicesFixture } from '../native/page-services-fixture.mjs'

class AndroidPageServicesTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'android-page-services-'))
  static evidence = process.env.TAC_ANDROID_PAGE_SERVICES_EVIDENCE ?? path.join(AndroidPageServicesTest.repository, 'target/android-page-services-evidence')
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
  static adb = path.join(AndroidPageServicesTest.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  static appOrigin = 'https://appassets.tachyon.local'
  static languages = PageServicesFixture.list('TAC_ANDROID_SERVICE_LANGUAGES', 'go,cs')
  static windowCallers = PageServicesFixture.list('TAC_ANDROID_WINDOW_CALLERS', 'swift,cs')
  static aheadOfTime = process.env.TAC_ANDROID_DOTNET_AOT !== 'false'
  static waitMilliseconds = 300000
  static pollMilliseconds = 250

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 1800000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return `${result.stdout}${result.stderr}`.trim()
  }

  /** @param {string[]} args */
  static adbShell(...args) {
    return this.run(this.adb, ['shell', ...args], 120000)
  }

  /** @param {string} project @param {boolean} aheadOfTime */
  static setAheadOfTime(project, aheadOfTime) {
    const file = path.join(project, 'client/manifest.json')
    const manifest = JSON.parse(readFileSync(file, 'utf8'))
    manifest.tac = { ...manifest.tac, dotnet: { aot: aheadOfTime } }
    writeFileSync(file, JSON.stringify(manifest, null, 2))
  }

  /** @param {string} apk */
  static webAssembly(apk) {
    const listed = this.run('unzip', ['-l', apk], 120000).split('\n')
    const files = listed.map((line) => line.trim().split(/\s+/)).filter((fields) => fields.length >= 4 && fields.at(-1).startsWith('assets/WebBundle/') && fields.at(-1).endsWith('.wasm'))
    return Object.fromEntries(files.map((fields) => [fields.at(-1).slice('assets/WebBundle/'.length), Number(fields[0])]).sort())
  }

  /** @param {Record<string, number>} files */
  static largest(files) {
    return Math.max(0, ...Object.values(files))
  }

  /** @param {string} project @param {string} application */
  static build(project, application) {
    const started = Date.now()
    const output = this.run(this.binary, ['build', project, '--target', 'android'])
    const host = JSON.parse(readFileSync(path.join(project, 'dist/android/tachyon.host.json'), 'utf8'))
    const apk = path.join(project, 'dist/android', application, `${application}.apk`)
    return { apk, host, built: Math.round((Date.now() - started) / 1000), notes: output.split('\n').filter((line) => line.startsWith('Note:')) }
  }

  /** @param {string} apk @param {string} packageName */
  static async launch(apk, packageName) {
    spawnSync(this.adb, ['uninstall', packageName], { timeout: 60000, stdio: 'ignore' })
    this.run(this.adb, ['install', '-r', apk], 300000)
    this.adbShell('am', 'start', '-W', '-n', `${packageName}/.MainActivity`)
    let pid = ''
    for (let attempt = 0; attempt < 120 && !(pid && this.adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)); attempt++) {
      await sleep(this.pollMilliseconds * 2)
      pid = spawnSync(this.adb, ['shell', 'pidof', packageName], { encoding: 'utf8', timeout: 30000 }).stdout.trim()
    }
    assert.match(pid, /^\d+$/, `${packageName} must start`)
    return pid
  }

  /** @param {string} packageName @param {(page: import('playwright').Page, pid: string) => Promise<object>} body @param {string} apk */
  static async withApp(apk, packageName, body) {
    const { chromium } = await import('playwright')
    let browser
    let port
    try {
      const pid = await this.launch(apk, packageName)
      port = this.run(this.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`])
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true })
      const page = browser.contexts()[0].pages()[0]
      assert.ok(page, 'the Android WebView exposes a live document')
      page.on('pageerror', (error) => console.error(`${packageName} page error:`, error.message))
      return await body(page, pid)
    } finally {
      if (browser) await browser.close().catch(() => {})
      if (port) spawnSync(this.adb, ['forward', '--remove', `tcp:${port}`], { timeout: 30000 })
      spawnSync(this.adb, ['shell', 'am', 'force-stop', packageName], { timeout: 30000 })
      spawnSync(this.adb, ['uninstall', packageName], { timeout: 60000, stdio: 'ignore' })
    }
  }

  /** @param {string} name */
  static screenshot(name) {
    const file = path.join(this.evidence, `${name}.png`)
    const result = spawnSync(this.adb, ['exec-out', 'screencap', '-p'], { maxBuffer: 64 * 1024 * 1024, timeout: 60000 })
    if (result.status === 0) writeFileSync(file, result.stdout)
    return file
  }

  /** @param {string} language */
  static interpreterComparison(language) {
    if (language !== 'cs' || !this.aheadOfTime) return undefined
    const project = path.join(this.root, `tally-${language}-interpreter`)
    const application = PageServicesFixture.stageTally(project, language)
    this.setAheadOfTime(project, false)
    return this.largest(this.webAssembly(this.build(project, application).apk))
  }

  /** @param {string} language */
  static async tally(language) {
    const project = path.join(this.root, `tally-${language}`)
    const application = PageServicesFixture.stageTally(project, language)
    this.setAheadOfTime(project, this.aheadOfTime)
    const { apk, host, built, notes } = this.build(project, application)
    assert.deepEqual(host.companions, [], `${language}: a ${language} service runs in the page, not linked into the Android host`)
    const webAssembly = this.webAssembly(apk)
    assert.ok(Object.keys(webAssembly).length > 0, `${language}: the APK carries the service's WebAssembly`)
    const interpreter = this.interpreterComparison(language)
    if (interpreter !== undefined) assert.ok(this.largest(webAssembly) > interpreter, `${language}: the ahead-of-time build (${this.largest(webAssembly)} bytes) is larger than the trimmed interpreter (${interpreter} bytes)`)
    const packageName = `dev.tachyon.tally_${language}`
    return this.withApp(apk, packageName, async (page, pid) => {
      await page.getByText('0 in the basket', { exact: true }).waitFor({ timeout: this.waitMilliseconds })
      await page.getByRole('button', { name: 'Add two' }).click()
      await page.getByText('2 in the basket', { exact: true }).waitFor({ timeout: this.waitMilliseconds })
      await page.getByText('2 items', { exact: true }).waitFor({ timeout: this.waitMilliseconds })
      const loads = await page.evaluate(() => performance.getEntriesByType('resource').map((entry) => entry.name).filter((name) => name.endsWith('.wasm')))
      assert.ok(loads.length > 0, `${language}: the page loaded WebAssembly`)
      for (const load of loads) assert.ok(load.startsWith(`${this.appOrigin}/`), `${language}: WebAssembly loads from the app bundle origin: ${load}`)
      const screenshot = this.screenshot(`tally-${language}`)
      const result = { language, built, notes, pid, webAssembly, interpreterLargest: interpreter, aheadOfTime: language === 'cs' ? this.aheadOfTime : undefined, loads, screenshot }
      console.log(`==> ${language}: built in ${built} s, ${Object.keys(webAssembly).length} WebAssembly files (largest ${this.largest(webAssembly)} bytes${interpreter === undefined ? '' : `, interpreter build ${interpreter} bytes`}), the page service answered "2 in the basket" and "2 items" in process ${pid}`)
      return result
    })
  }

  static applicationWindowTitle() {
    const dump = spawnSync(this.adb, ['shell', 'uiautomator', 'dump', '--windows', '/sdcard/tachyon-windows.xml'], { encoding: 'utf8', timeout: 60000 })
    if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return ''
    return this.adbShell('cat', '/sdcard/tachyon-windows.xml').match(/<window [^>]*title="([^"]*)"[^>]*type="TYPE_APPLICATION"/)?.[1] ?? ''
  }

  /** @param {string} packageName */
  static resumed(packageName) {
    return (this.adbShell('dumpsys', 'activity', 'activities').split('\n').find((line) => line.includes('topResumedActivity=')) ?? '').includes(packageName)
  }

  /** @param {() => unknown} check @param {() => string | Promise<string>} message */
  static async waitFor(check, message) {
    for (let attempt = 0; attempt < 40; attempt++) {
      if (await check()) return
      await sleep(this.pollMilliseconds)
    }
    assert.fail(await message())
  }

  /** @param {string} language */
  static async windowCaller(language) {
    const project = path.join(this.root, `window-${language}`)
    const application = PageServicesFixture.stageWindowCaller(project, language)
    const { apk, host, built } = this.build(project, application)
    assert.deepEqual(host.companions, [], `${language}: the WindowControls caller runs in the page as WebAssembly`)
    const packageName = `dev.tachyon.window_${language}`
    return this.withApp(apk, packageName, async (page, pid) => {
      await page.getByRole('button', { name: 'Rename' }).waitFor({ timeout: this.waitMilliseconds })
      const wasm = await page.evaluate(() => performance.getEntriesByType('resource').some((entry) => entry.name.endsWith('.wasm')))
      assert.ok(wasm, `${language}: the caller loaded WebAssembly`)
      assert.notEqual(this.applicationWindowTitle(), 'Quarterly report')
      await page.getByRole('button', { name: 'Rename' }).click()
      await this.waitFor(() => this.applicationWindowTitle() === 'Quarterly report', () => `${language}: Rename sets the Android window title: ${this.applicationWindowTitle()}`)
      await page.getByRole('button', { name: 'Resize' }).click()
      const size = () => page.evaluate(() => [innerWidth, innerHeight])
      await this.waitFor(async () => JSON.stringify(await size()) === '[1280,800]', async () => `${language}: Resize makes the window 1280 by 800: ${await size()}`)
      assert.equal((await page.locator('main p').textContent()).trim(), '', `${language}: granted controls report no problem`)
      await page.getByRole('button', { name: 'Minimise' }).click()
      await this.waitFor(() => !this.resumed(packageName), () => `${language}: Minimise moves the app behind the launcher`)
      const result = { language, built, pid, title: 'Quarterly report', size: [1280, 800], minimized: true }
      console.log(`==> ${language}: a WebAssembly WindowControls caller set the title, resized the window to 1280 by 800 and minimised the app (process ${pid})`)
      return result
    })
  }

  static async main() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required')
    mkdirSync(this.evidence, { recursive: true })
    this.run(this.adb, ['wait-for-device'], 120000)
    const report = { release: this.adbShell('getprop', 'ro.build.version.release'), abi: this.adbShell('getprop', 'ro.product.cpu.abi'), results: [], failures: [] }
    const checks = [...this.languages.map((language) => ['tally', language]), ...this.windowCallers.map((language) => ['windowCaller', language])]
    try {
      for (const [check, language] of checks) {
        try {
          report.results.push({ check, ...(await this[check](language)) })
        } catch (error) {
          console.log(`==> ${check} ${language}: FAIL ${String(error.message).slice(0, 2000)}`)
          report.failures.push({ check, language, error: String(error.message).slice(0, 4000) })
        }
        writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      }
    } finally {
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    assert.deepEqual(report.failures.map(({ check, language }) => `${check} ${language}`), [], 'some page services failed; see the report')
    console.log(`PASS: Android ${report.release} (${report.abi}) page services (${this.languages.join(', ')}${this.aheadOfTime ? ', C# ahead of time' : ''}) answered in the app's web view, and WebAssembly WindowControls callers (${this.windowCallers.join(', ')}) set the title, resized and minimised the app. Report: ${path.join(this.evidence, 'report.json')}`)
  }
}

await AndroidPageServicesTest.main()

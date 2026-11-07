#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

class AndroidRustCrateTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'android-rust-crate-'))
  static project = path.join(AndroidRustCrateTest.root, 'project')
  static evidence = process.env.TAC_ANDROID_RUST_CRATE_EVIDENCE ?? path.join(AndroidRustCrateTest.repository, 'target/android-rust-crate-evidence')
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
  static adb = path.join(AndroidRustCrateTest.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  static application = 'Greeter'
  static applicationId = 'dev.tachyon.android-rust-crate'
  static packageName = 'dev.tachyon.android_rust_crate'
  static abis = ['arm64-v8a', 'x86_64']
  static waitMilliseconds = 120000
  static pollMilliseconds = 250

  static files = {
    'client/services/GreetingService.rs': `#[Service]
#[derive(Default)]
struct GreetingService {
    text: String,
}

impl GreetingService {
    pub fn greet(&mut self) {
        self.text = format!("{} from {}", tiny_greeting::hello(), tiny_platform::name());
    }

    pub fn process_id(&self) -> i64 {
        i64::from(std::process::id())
    }
}
`,
    'client/components/greeter/panel/tac.html': '<main>\n  <h1>Greeter</h1>\n  <p id="greeting">{greeting.text}</p>\n  <button id="greet" on:click="greeting.greet()">Greet</button>\n</main>\n',
    'client/pages/tac.html': '<greeter-panel />\n',
  }

  static crates = {
    'tiny-greeting': 'pub fn hello() -> String { String::from("hello") }\n',
    'tiny-platform': 'pub fn name() -> &\'static str { "android" }\n',
  }

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  /** @param {string[]} args */
  static adbShell(...args) {
    return this.run(this.adb, ['shell', ...args], 120000)
  }

  /** @param {string} name @param {string} text */
  static write(name, text) {
    const target = path.join(this.project, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static prepare() {
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    for (const [name, body] of Object.entries(this.crates)) {
      this.write(`vendor/rust/${name}/Cargo.toml`, `[package]\nname = "${name}"\nversion = "1.0.0"\nedition = "2021"\n`)
      this.write(`vendor/rust/${name}/src/lib.rs`, body)
      this.write(`vendor/rust/${name}/.cargo-checksum.json`, '{"files":{},"package":null}')
    }
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    this.write('client/manifest.json', JSON.stringify({
      name: this.application,
      start_url: '/',
      tac: {
        ...checks,
        application: {
          id: this.applicationId,
          version: '0.0.1',
          rust: { dependencies: { 'tiny-greeting': '=1.0.0' }, platformDependencies: { android: { 'tiny-platform': '=1.0.0' } } },
        },
      },
    }))
  }

  static checkBuild() {
    const output = path.join(this.project, 'dist/android')
    const host = JSON.parse(readFileSync(path.join(output, 'tachyon.host.json'), 'utf8'))
    assert.ok(host.companions.some((item) => item.service === 'greeting' && item.language === 'Rust'), `the Rust service is linked into the app: ${JSON.stringify(host.companions)}`)
    const apk = path.join(output, this.application, `${this.application}.apk`)
    const entries = this.run('unzip', ['-Z1', apk]).split('\n')
    const libraries = entries.filter((entry) => entry.startsWith('lib/') && entry.endsWith('.so'))
    for (const abi of this.abis) assert.ok(libraries.some((entry) => entry.startsWith(`lib/${abi}/`)), `the APK carries a Rust library for ${abi}: ${libraries.join(', ')}`)
    const crates = readFileSync(this.companionManifest(output), 'utf8')
    for (const name of Object.keys(this.crates)) assert.ok(crates.includes(`${name} = { version = "=1.0.0" }`), `${name} is not in the Rust companion crate:\n${crates}`)
    return { apk, libraries, crates }
  }

  /** @param {string} output */
  static companionManifest(output) {
    const found = spawnSync('find', [output, '-path', '*rust-companion/Cargo.toml'], { encoding: 'utf8' }).stdout.trim().split('\n').filter(Boolean)
    assert.ok(found.length > 0, `no Rust companion crate under ${output}`)
    return found[0]
  }

  /** @param {string} apk */
  static async launch(apk) {
    this.run(this.adb, ['wait-for-device'])
    spawnSync(this.adb, ['uninstall', this.packageName], { timeout: 60000, stdio: 'ignore' })
    this.run(this.adb, ['install', '-r', apk], 300000)
    this.adbShell('am', 'start', '-W', '-n', `${this.packageName}/.MainActivity`)
    let pid = ''
    for (let attempt = 0; attempt < 120 && !(pid && this.adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)); attempt++) {
      await sleep(this.pollMilliseconds * 2)
      pid = spawnSync(this.adb, ['shell', 'pidof', this.packageName], { encoding: 'utf8', timeout: 30000 }).stdout.trim()
    }
    assert.match(pid, /^\d+$/, 'the Greeter app must start')
    return pid
  }

  /** @param {string} name */
  static screenshot(name) {
    const file = path.join(this.evidence, `${name}.png`)
    const result = spawnSync(this.adb, ['exec-out', 'screencap', '-p'], { maxBuffer: 64 * 1024 * 1024, timeout: 60000 })
    if (result.status === 0) writeFileSync(file, result.stdout)
    return file
  }

  static async main() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required')
    mkdirSync(this.evidence, { recursive: true })
    const { chromium } = await import('playwright')
    const report = {}
    let browser
    let port
    try {
      this.prepare()
      this.run(this.binary, ['build', this.project, '--target', 'android'], 1800000)
      const { apk, libraries, crates } = this.checkBuild()
      Object.assign(report, { libraries, crates })
      const pid = await this.launch(apk)
      port = this.run(this.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`])
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true })
      const page = browser.contexts()[0].pages()[0]
      assert.ok(page, 'the Android WebView exposes a live document')
      await page.locator('#greet').waitFor({ timeout: this.waitMilliseconds })
      report.before = (await page.textContent('#greeting')).trim()
      await page.locator('#greet').click()
      await page.waitForFunction((before) => document.querySelector('#greeting').textContent.trim() !== before, report.before, { timeout: this.waitMilliseconds })
      report.after = (await page.textContent('#greeting')).trim()
      report.servicePid = await page.evaluate(() => __tachyonTac.service('greeting').processId())
      report.pid = Number(pid)
      report.screenshot = this.screenshot('greeted')
      assert.equal(report.after, 'hello from android', `the Rust service did not answer with its vendored crates: ${JSON.stringify(report)}`)
      assert.equal(Number(report.servicePid), report.pid, 'the Rust service ran inside the Android app process')
      report.release = this.adbShell('getprop', 'ro.build.version.release')
      report.abi = this.adbShell('getprop', 'ro.product.cpu.abi')
    } finally {
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      if (browser) await browser.close().catch(() => {})
      if (port) spawnSync(this.adb, ['forward', '--remove', `tcp:${port}`], { timeout: 30000 })
      spawnSync(this.adb, ['shell', 'am', 'force-stop', this.packageName], { timeout: 30000 })
      spawnSync(this.adb, ['uninstall', this.packageName], { timeout: 60000, stdio: 'ignore' })
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    console.log(`PASS: a Rust service using vendored crates (tiny-greeting, and tiny-platform for Android only) answered "${report.after}" in the Android ${report.release} emulator app (${report.abi}, process ${report.pid}).`)
  }
}

await AndroidRustCrateTest.main()

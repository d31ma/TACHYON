#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

class AsyncListenerApp {
  static proofFile = 'tachyon-async-listener.json'
  static listenerPause = 400
  static wanted = ['a1s1', 'a1s1a2s2']

  static models = {
    swift: 'struct Ping {\n    let step: Int\n}\n',
    kt: 'data class Ping(val step: Int)\n',
  }

  static services = {
    swift: `import Foundation

@Service
final class HeardService {
    var heard: String = ""
    var listenerProcess: Int = 0
    let applicationEventPublisher: ApplicationEventPublisher

    func send(_ step: Int) { applicationEventPublisher.publishEvent(Ping(step: step)) }

    @OnEvent
    func later(_ ping: Ping) async {
        try? await Task.sleep(nanoseconds: ${AsyncListenerApp.listenerPause}_000_000)
        heard += "a\\(ping.step)"
        listenerProcess = Int(ProcessInfo.processInfo.processIdentifier)
    }

    @OnEvent
    func now(_ ping: Ping) { heard += "s\\(ping.step)" }
}
`,
    kt: `import models.Ping

@Service
class HeardService {
    val applicationEventPublisher: ApplicationEventPublisher
    var heard: String = ""
    var listenerProcess: Int = 0

    fun send(step: Int) { applicationEventPublisher.publishEvent(Ping(step)) }

    @OnEvent
    suspend fun later(ping: Ping) {
        kotlin.coroutines.suspendCoroutine<Unit> { continuation ->
            Thread { Thread.sleep(${AsyncListenerApp.listenerPause}L); continuation.resumeWith(Result.success(Unit)) }.start()
        }
        heard += "a" + ping.step
        listenerProcess = android.os.Process.myPid()
    }

    @OnEvent
    fun now(ping: Ping) { heard += "s" + ping.step }
}
`,
  }

  static recorder = `import Foundation

@Service
final class AsyncProbeService {
    var phase: String = "starting"
    var proof: String = ""

    func record() -> Int {
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier), "phase": phase, "proof": proof]
        let destination = FileManager.default.temporaryDirectory.appendingPathComponent("${AsyncListenerApp.proofFile}")
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: destination, options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }
}
`

  static probe = `@Service
export default class AsyncProbeService {
  static WAIT = 120000;
  static PAUSE = 20;
  phase = 'starting';
  proof = '';

  @OnStart
  initialize() {
    setTimeout(() => { void this.probe(); }, 50);
  }

  async probe() {
    const seen = [];
    try {
      await this.until(() => this.heard() === '[]', seen);
      const timings = [];
      for (const [label, wanted] of [['Send one', '[a1s1]'], ['Send two', '[a1s1a2s2]']]) {
        const started = performance.now();
        [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === label).click();
        await this.until(() => this.heard() === wanted, seen);
        timings.push(Math.round(performance.now() - started));
      }
      await this.finish('done', { heard: this.heard(), seen, timings, listenerProcess: Number(document.querySelector('#listener-process').textContent) });
    } catch (error) {
      await this.finish('error', { error: String(error), seen, text: document.body.innerText.slice(0, 400) });
    }
  }

  heard() {
    return document.querySelector('#heard')?.textContent.trim();
  }

  /** @param {() => boolean} check @param {string[]} seen */
  async until(check, seen) {
    const deadline = Date.now() + AsyncProbeService.WAIT;
    while (Date.now() < deadline) {
      const heard = this.heard();
      if (heard !== undefined && seen.at(-1) !== heard) seen.push(heard);
      if (check()) return;
      await new Promise((resolve) => setTimeout(resolve, AsyncProbeService.PAUSE));
    }
    throw new Error('The page did not show the listener result');
  }

  /** @param {string} phase @param {object} facts */
  async finish(phase, facts) {
    this.phase = phase;
    this.proof = JSON.stringify(facts);
    if (typeof this.record === 'function') await this.record();
  }
}
`

  static view = `<main aria-label="Async listener probe">
<p id="heard">[{heard.heard}]</p>
<p id="listener-process">{heard.listenerProcess}</p>
<button on:click="heard.send(1)">Send one</button>
<button on:click="heard.send(2)">Send two</button>
<p>Probe {asyncProbe.phase}</p>
</main>
`

  /** @param {string} project @param {string} name @param {string} contents */
  static write(project, name, contents) {
    const file = path.join(project, name)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }

  /** @param {string} project @param {'swift' | 'kt'} language */
  static stage(project, language) {
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const applicationId = `dev.tachyon.async-listener-${language}`
    const name = `AsyncListener${language === 'kt' ? 'Kotlin' : 'Swift'}`
    this.write(project, 'client/manifest.json', JSON.stringify({ name, start_url: '/', tac: { ...checks, application: { id: applicationId, version: '0.0.1' } } }, null, 2))
    this.write(project, `client/models/Ping.${language}`, this.models[language])
    this.write(project, `client/services/HeardService.${language}`, this.services[language])
    this.write(project, 'client/services/AsyncProbeService.js', this.probe)
    if (language === 'swift') this.write(project, 'client/services/AsyncProbeService.swift', this.recorder)
    this.write(project, 'client/components/async/probe/tac.html', this.view)
    this.write(project, 'client/pages/tac.html', '<async-probe />\n')
    return { name, applicationId }
  }

  /** @param {{ phase: string, proof: string, pid?: number }} record @param {number} pid */
  static assertProof(record, pid) {
    const proof = JSON.parse(record.proof)
    assert.equal(record.phase, 'done', `the probe failed: ${record.proof}`)
    assert.equal(proof.heard, `[${this.wanted.at(-1)}]`, 'both listeners ran for both events')
    assert.deepEqual(proof.seen, ['[]', ...this.wanted.map((text) => `[${text}]`)], 'the page never showed the plain listener before the async one finished')
    assert.ok(proof.timings.every((milliseconds) => milliseconds >= this.listenerPause), `each send waited for the ${this.listenerPause} ms listener: ${proof.timings}`)
    assert.equal(proof.listenerProcess, pid, 'the async listener ran inside the app process')
    if (record.pid !== undefined) assert.equal(record.pid, pid, 'the proof came from the launched app')
    return proof
  }

  /** @param {string} output @param {string} language */
  static assertLinked(output, language) {
    const host = JSON.parse(readFileSync(path.join(output, 'tachyon.host.json'), 'utf8'))
    assert.ok(host.companions.some((item) => item.service === 'heard' && item.language === language), `the ${language} listener service is linked into the app: ${JSON.stringify(host.companions)}`)
    const modules = readdirSync(output, { recursive: true }).filter((name) => String(name).endsWith('.wasm'))
    assert.deepEqual(modules, [], 'the linked service is not also shipped as WebAssembly')
  }
}

class AsyncListenerAppTest {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'async-listener-app-'))
  static targets = (process.argv.find((argument) => argument.startsWith('--target='))?.slice('--target='.length) ?? 'macos,ios').split(',')
  static proofTimeout = 180000
  static pause = 250
  static deviceSet = ''
  static device = ''

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 900000) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  /** @param {string} file */
  static async record(file) {
    const deadline = Date.now() + this.proofTimeout
    while (Date.now() < deadline && !existsSync(file)) await sleep(this.pause)
    assert.ok(existsSync(file), `the app wrote no proof at ${file}`)
    return JSON.parse(readFileSync(file, 'utf8'))
  }

  /** @param {string} project @param {string} target */
  static build(project, target) {
    this.run(this.binary, ['build', project, '--target', target])
    const output = path.join(project, 'dist', target)
    AsyncListenerApp.assertLinked(output, 'Swift')
    const bundle = readdirSync(output).find((name) => name.endsWith('.app'))
    assert.ok(bundle, `no .app in ${output}`)
    return path.join(output, bundle)
  }

  static async macos() {
    const project = path.join(this.root, 'macos')
    AsyncListenerApp.stage(project, 'swift')
    const application = this.build(project, 'macos')
    const file = path.join(this.run('/usr/bin/getconf', ['DARWIN_USER_TEMP_DIR']), AsyncListenerApp.proofFile)
    rmSync(file, { force: true })
    this.run('/usr/bin/open', ['-n', application])
    try {
      const record = await this.record(file)
      const launched = spawnSync('/usr/bin/pgrep', ['-f', path.join(application, 'Contents/MacOS')], { encoding: 'utf8' }).stdout.trim()
      const proof = AsyncListenerApp.assertProof(record, Number(launched))
      console.log(`==> macos: ${JSON.stringify(proof)}`)
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(application, 'Contents/MacOS')])
    }
    console.log('PASS macOS: a linked Swift async @OnEvent listener finished inside the app before the next listener, for two events in a row.')
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], 240000)
  }

  static bootSimulator() {
    this.deviceSet = path.join(this.root, 'simulator-devices')
    mkdirSync(this.deviceSet)
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.name === 'iPhone 17 Pro')
      ?? (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `AsyncListener-${randomUUID()}`, phone.identifier, runtime.identifier)
    this.simctl('boot', this.device)
    this.simctl('bootstatus', this.device, '-b')
    return `${phone.name}, iOS ${runtime.version}`
  }

  static shutdownSimulator() {
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
  }

  static async ios() {
    const project = path.join(this.root, 'ios')
    const { applicationId } = AsyncListenerApp.stage(project, 'swift')
    const application = this.build(project, 'ios')
    const device = this.bootSimulator()
    try {
      this.simctl('install', this.device, application)
      const launched = this.simctl('launch', this.device, applicationId)
      const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1])
      const container = this.simctl('get_app_container', this.device, applicationId, 'data')
      const proof = AsyncListenerApp.assertProof(await this.record(path.join(container, 'tmp', AsyncListenerApp.proofFile)), pid)
      console.log(`==> ios (${device}): ${JSON.stringify(proof)}`)
      this.simctl('terminate', this.device, applicationId)
    } finally {
      this.shutdownSimulator()
    }
    console.log(`PASS iOS simulator (${device}): a linked Swift async @OnEvent listener finished inside the app before the next listener, for two events in a row.`)
  }

  static adb() {
    const sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
    assert.ok(sdk, 'set ANDROID_HOME for the Android run')
    return path.join(sdk, 'platform-tools', 'adb')
  }

  /** @param {string[]} args */
  static adbShell(...args) {
    return this.run(this.adb(), ['shell', ...args], 120000)
  }

  static async android() {
    const { chromium } = await import('playwright')
    const project = path.join(this.root, 'android')
    const { name, applicationId } = AsyncListenerApp.stage(project, 'kt')
    this.run(this.binary, ['bundle', project, '--target', 'android'])
    AsyncListenerApp.assertLinked(path.join(project, 'dist/android'), 'Kotlin')
    const apk = path.join(project, 'dist/android', name, `${name}.apk`)
    assert.ok(existsSync(apk), `no APK at ${apk}`)
    const packageName = applicationId.replaceAll('-', '_')
    this.run(this.adb(), ['install', '-r', apk])
    this.adbShell('am', 'force-stop', packageName)
    this.adbShell('am', 'start', '-n', `${packageName}/.MainActivity`)
    let pid = ''
    for (let attempt = 0; attempt < 120 && !(pid && this.adbShell('cat', '/proc/net/unix').includes(`webview_devtools_remote_${pid}`)); attempt++) {
      await sleep(this.pause * 2)
      pid = spawnSync(this.adb(), ['shell', 'pidof', packageName], { encoding: 'utf8' }).stdout.trim()
    }
    assert.match(pid, /^\d+$/, 'the Android app must start')
    const port = this.run(this.adb(), ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`])
    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { noDefaults: true })
    try {
      const page = browser.contexts()[0].pages()[0]
      assert.ok(page, 'the Android WebView exposes a live document')
      await page.waitForFunction(() => ['done', 'error'].includes(__tachyonTac.service('asyncProbe').phase), null, { timeout: this.proofTimeout })
      const record = await page.evaluate(() => ({ phase: __tachyonTac.service('asyncProbe').phase, proof: __tachyonTac.service('asyncProbe').proof }))
      const proof = AsyncListenerApp.assertProof(record, Number(pid))
      console.log(`==> android: ${JSON.stringify(proof)}`)
    } finally {
      await browser.close()
      this.run(this.adb(), ['forward', '--remove', `tcp:${port}`])
      this.adbShell('am', 'force-stop', packageName)
      spawnSync(this.adb(), ['uninstall', packageName], { stdio: 'ignore', timeout: 120000 })
    }
    console.log('PASS Android emulator: a linked Kotlin suspend @OnEvent listener finished inside the packaged app before the next listener, for two events in a row.')
  }

  static async main() {
    try {
      for (const target of this.targets) {
        assert.ok(['macos', 'ios', 'android'].includes(target), `unknown target ${target}; use macos, ios or android`)
        await this[target]()
      }
    } finally {
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
  }
}

await AsyncListenerAppTest.main()

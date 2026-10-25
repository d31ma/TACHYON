#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

class ProbeRecorder {
  /** @param {string} name @param {string} file */
  static swift(name, file) {
    return `import Foundation

@Service
final class ${name} {
    var phase: String = "starting"
    var proof: String = ""

    func record() -> Int {
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier), "phase": phase, "proof": proof]
        let destination = FileManager.default.temporaryDirectory.appendingPathComponent("${file}")
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: destination, options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }
}
`
  }

  /** @param {string} name @param {string} file */
  static rs(name, file) {
    return `#[derive(Default)]
#[Service]
pub struct ${name} {
    pub phase: String,
    pub proof: String,
}

impl ${name} {
    const PROOF_FILE: &'static str = "${file}";

    pub fn record(&mut self) -> i64 {
        let pid = std::process::id();
        let record = format!("{{\\"phase\\":{},\\"pid\\":{pid},\\"proof\\":{}}}", Self::quoted(&self.phase), Self::quoted(&self.proof));
        let _ = std::fs::write(std::env::temp_dir().join(Self::PROOF_FILE), record);
        i64::from(pid)
    }

    fn quoted(text: &str) -> String {
        format!("\\"{}\\"", text.replace('\\\\', "\\\\\\\\").replace('"', "\\\\\\""))
    }
}
`
  }
}

class AsyncListenerApp {
  static proofFile = 'tachyon-async-listener.json'
  static probeService = 'asyncProbe'
  static listenerPause = 400
  static wanted = ['a1s1', 'a1s1a2s2']

  static models = {
    swift: 'struct Ping {\n    let step: Int\n}\n',
    kt: 'data class Ping(val step: Int)\n',
    rs: 'pub struct Ping {\n    pub step: i64,\n}\n',
  }

  static names = { swift: 'Swift', kt: 'Kotlin', rs: 'Rust' }

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
    rs: `use crate::models::Ping;

#[derive(Default)]
#[Service]
struct HeardService {
    heard: String,
    listener_process: i64,
    application_event_publisher: ApplicationEventPublisher,
}

impl HeardService {
    pub fn send(&mut self, step: i64) {
        self.application_event_publisher.publish_event(Ping { step }, false);
    }

    #[OnEvent]
    async fn later(&mut self, value: Ping) {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut started = false;
        std::future::poll_fn(|context| {
            if done.load(std::sync::atomic::Ordering::SeqCst) {
                return std::task::Poll::Ready(());
            }
            if !started {
                started = true;
                let waker = context.waker().clone();
                let flag = done.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(${AsyncListenerApp.listenerPause}));
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    waker.wake();
                });
            }
            std::task::Poll::Pending
        })
        .await;
        self.heard.push_str(&format!("a{}", value.step));
        self.listener_process = i64::from(std::process::id());
    }

    #[OnEvent]
    fn now(&mut self, value: Ping) {
        self.heard.push_str(&format!("s{}", value.step));
    }
}
`,
  }

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

  /** @param {string} project @param {'swift' | 'kt' | 'rs'} language */
  static stage(project, language) {
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const applicationId = `dev.tachyon.async-listener-${language}`
    const name = `AsyncListener${this.names[language]}`
    this.write(project, 'client/manifest.json', JSON.stringify({ name, start_url: '/', tac: { ...checks, application: { id: applicationId, version: '0.0.1' } } }, null, 2))
    this.write(project, `client/models/Ping.${language}`, this.models[language])
    this.write(project, `client/services/HeardService.${language}`, this.services[language])
    this.write(project, 'client/services/AsyncProbeService.js', this.probe)
    if (ProbeRecorder[language]) this.write(project, `client/services/AsyncProbeService.${language}`, ProbeRecorder[language]('AsyncProbeService', this.proofFile))
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
    this.assertService(output, 'heard', language)
  }

  /** @param {string} language */
  static passed(language) {
    return `a linked ${language} async @OnEvent listener finished inside the app before the next listener, for two events in a row.`
  }

  /** @param {string} output @param {string} service @param {string} language */
  static assertService(output, service, language) {
    const host = JSON.parse(readFileSync(path.join(output, 'tachyon.host.json'), 'utf8'))
    assert.ok(host.companions.some((item) => item.service === service && item.language === language), `the ${language} ${service} service is linked into the app: ${JSON.stringify(host.companions)}`)
    const modules = readdirSync(output, { recursive: true }).filter((name) => String(name).endsWith('.wasm'))
    assert.deepEqual(modules, [], 'the linked service is not also shipped as WebAssembly')
  }
}

class SharedSetupApp {
  static proofFile = 'tachyon-shared-setup.json'
  static probeService = 'sharedProbe'
  static linkedServices = ['writer', 'reader']
  static classes = { writer: 'WriterService', reader: 'ReaderService' }
  static idleYon = { origin: 'http://127.0.0.1:9' }

  static files = {
    swift: {
      'client/models/ProgramState.swift': 'struct ProgramState: Codable {\n    let identity: String\n    let count: Int\n}\n',
      'client/remotes/PageProgramClient.swift': '@HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})\nprotocol PageProgramClient {\n    @GetRequest("/read")\n    func read() async throws -> ProgramState\n\n    @PostRequest("/increment")\n    func increment() async throws -> ProgramState\n}\n',
      'client/setups/PageProgramSetup.swift': `@Setup
final class PageProgramSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        let identity = String(UInt64.random(in: 1...UInt64.max), radix: 16)
        var count = 0
        var adapters = BehaviorAdapters()
        adapters.http["pageprogram"] = BehaviorHTTPAdapter { request, _ in
            if try request.get("method").text == "POST", try request.get("path").text.hasSuffix("/increment") {
                count += 1
            }
            return .object(["identity": .string(identity), "count": .number(Double(count))])
        }
        return adapters
    }
}
`,
    },
    kt: {
      'client/models/ProgramState.kt': 'data class ProgramState(val identity: String, val count: Int)\n',
      'client/remotes/PageProgramClient.kt': 'import models.ProgramState\n\n@HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})\ninterface PageProgramClient {\n    @GetRequest("/read")\n    suspend fun read(): ProgramState\n\n    @PostRequest("/increment")\n    suspend fun increment(): ProgramState\n}\n',
      'client/setups/PageProgramSetup.kt': `@Setup
class PageProgramSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {
            val identity = kotlin.random.Random.nextLong().toULong().toString(16)
            var count = 0
            val program = Behaviors.HttpAdapter { request, _ ->
                val path = request["path"] as? String ?: ""
                if (request["method"] == "POST" && path.endsWith("/increment")) count += 1
                mapOf("identity" to identity, "count" to count)
            }
            return Behaviors.Configuration(http = mapOf("pageprogram" to program))
        }
    }
}
`,
    },
  }

  static services = {
    swift: (name) => `@Service
final class ${name} {
    @Needs
    private let pageProgramClient: PageProgramClient
    var identity: String = ""
    var count: Int = 0

    func inspect() async throws {
        show(try await pageProgramClient.read())
    }

    func increment() async throws {
        show(try await pageProgramClient.increment())
    }

    private func show(_ state: ProgramState) {
        identity = state.identity
        count = state.count
    }
}
`,
    kt: (name) => `import models.ProgramState

@Service
class ${name} {
    @Needs
    private val pageProgramClient: PageProgramClient
    var identity: String = ""
    var count: Int = 0

    suspend fun inspect() {
        show(pageProgramClient.read())
    }

    suspend fun increment() {
        show(pageProgramClient.increment())
    }

    private fun show(state: ProgramState) {
        identity = state.identity
        count = state.count
    }
}
`,
  }

  static probe = `@Service
export default class SharedProbeService {
  static WAIT = 120000;
  static PAUSE = 20;
  phase = 'starting';
  proof = '';

  @OnStart
  initialize() {
    setTimeout(() => { void this.probe(); }, 50);
  }

  async probe() {
    try {
      await this.until(() => this.shown('writer-count') === '0' && this.shown('reader-count') === '0');
      this.press('Increment');
      await this.until(() => this.shown('writer-count') === '1' && this.shown('writer-identity') !== '');
      this.press('Inspect');
      await this.until(() => this.shown('reader-count') === '1' && this.shown('reader-identity') !== '');
      const facts = {};
      for (const service of ['writer', 'reader']) facts[service] = { identity: this.shown(service + '-identity'), count: this.shown(service + '-count') };
      await this.finish('done', facts);
    } catch (error) {
      await this.finish('error', { error: String(error), text: document.body.innerText.slice(0, 400) });
    }
  }

  /** @param {string} id */
  shown(id) {
    return document.querySelector('#' + id)?.textContent.trim();
  }

  /** @param {string} label */
  press(label) {
    [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === label).click();
  }

  /** @param {() => boolean} check */
  async until(check) {
    const deadline = Date.now() + SharedProbeService.WAIT;
    while (Date.now() < deadline) {
      if (check()) return;
      await new Promise((resolve) => setTimeout(resolve, SharedProbeService.PAUSE));
    }
    throw new Error('The page did not show both services');
  }

  /** @param {string} phase @param {object} facts */
  async finish(phase, facts) {
    this.phase = phase;
    this.proof = JSON.stringify(facts);
    if (typeof this.record === 'function') await this.record();
  }
}
`

  static view = `<main aria-label="Shared setup probe">
<p id="writer-identity">{writer.identity}</p>
<p id="writer-count">{writer.count}</p>
<p id="reader-identity">{reader.identity}</p>
<p id="reader-count">{reader.count}</p>
<button on:click="writer.increment()">Increment</button>
<button on:click="reader.inspect()">Inspect</button>
<p>Probe {sharedProbe.phase}</p>
</main>
`

  /** @param {string} project @param {'swift' | 'kt'} language */
  static stage(project, language) {
    assert.ok(this.files[language], `the shared Setup check runs linked Swift or Kotlin services, not ${language}`)
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const applicationId = `dev.tachyon.shared-setup-${language}`
    const name = `SharedSetup${AsyncListenerApp.names[language]}`
    const tac = { ...checks, application: { id: applicationId, version: '0.0.1' }, yon: this.idleYon }
    AsyncListenerApp.write(project, 'client/manifest.json', JSON.stringify({ name, start_url: '/', tac }, null, 2))
    for (const [file, contents] of Object.entries(this.files[language])) AsyncListenerApp.write(project, file, contents)
    for (const service of Object.values(this.classes)) AsyncListenerApp.write(project, `client/services/${service}.${language}`, this.services[language](service))
    AsyncListenerApp.write(project, 'client/services/SharedProbeService.js', this.probe)
    if (ProbeRecorder[language]) AsyncListenerApp.write(project, `client/services/SharedProbeService.${language}`, ProbeRecorder[language]('SharedProbeService', this.proofFile))
    AsyncListenerApp.write(project, 'client/components/shared/probe/tac.html', this.view)
    AsyncListenerApp.write(project, 'client/pages/tac.html', '<shared-probe />\n')
    return { name, applicationId }
  }

  /** @param {{ phase: string, proof: string, pid?: number }} record @param {number} pid */
  static assertProof(record, pid) {
    const proof = JSON.parse(record.proof)
    assert.equal(record.phase, 'done', `the probe failed: ${record.proof}`)
    assert.match(proof.writer.identity, /^[0-9a-f]+$/, 'the writer read an identity from the Setup')
    assert.equal(proof.reader.identity, proof.writer.identity, 'both linked services read one Setup configuration')
    assert.equal(proof.reader.count, '1', 'the reader saw the count the writer raised')
    if (record.pid !== undefined) assert.equal(record.pid, pid, 'the proof came from the launched app')
    return proof
  }

  /** @param {string} output @param {string} language */
  static assertLinked(output, language) {
    for (const service of this.linkedServices) AsyncListenerApp.assertService(output, service, language)
  }

  /** @param {string} language */
  static passed(language) {
    return `two linked ${language} services of one page read one Setup configuration inside the app: the reader saw the writer's identity and count.`
  }
}


class AsyncListenerAppTest {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'async-listener-app-'))
  static checks = { listener: AsyncListenerApp, 'shared-setup': SharedSetupApp }
  static check = process.argv.find((argument) => argument.startsWith('--check='))?.slice('--check='.length) ?? 'listener'
  static fixture = AsyncListenerAppTest.checks[AsyncListenerAppTest.check]
  static appleLanguage = process.env.TAC_LISTENER_LANGUAGE ?? 'swift'
  static androidLanguage = process.env.TAC_ANDROID_LISTENER_LANGUAGE ?? 'kt'
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
    this.fixture.assertLinked(output, AsyncListenerApp.names[this.appleLanguage])
    const bundle = readdirSync(output).find((name) => name.endsWith('.app'))
    assert.ok(bundle, `no .app in ${output}`)
    return path.join(output, bundle)
  }

  static async macos() {
    const project = path.join(this.root, 'macos')
    this.fixture.stage(project, this.appleLanguage)
    const application = this.build(project, 'macos')
    const file = path.join(this.run('/usr/bin/getconf', ['DARWIN_USER_TEMP_DIR']), this.fixture.proofFile)
    rmSync(file, { force: true })
    this.run('/usr/bin/open', ['-n', application])
    try {
      const record = await this.record(file)
      const launched = spawnSync('/usr/bin/pgrep', ['-f', path.join(application, 'Contents/MacOS')], { encoding: 'utf8' }).stdout.trim()
      const proof = this.fixture.assertProof(record, Number(launched))
      console.log(`==> macos: ${JSON.stringify(proof)}`)
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(application, 'Contents/MacOS')])
    }
    console.log(`PASS macOS: ${this.fixture.passed(AsyncListenerApp.names[this.appleLanguage])}`)
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
    const { applicationId } = this.fixture.stage(project, this.appleLanguage)
    const application = this.build(project, 'ios')
    const device = this.bootSimulator()
    try {
      this.simctl('install', this.device, application)
      const launched = this.simctl('launch', this.device, applicationId)
      const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1])
      const container = this.simctl('get_app_container', this.device, applicationId, 'data')
      const proof = this.fixture.assertProof(await this.record(path.join(container, 'tmp', this.fixture.proofFile)), pid)
      console.log(`==> ios (${device}): ${JSON.stringify(proof)}`)
      this.simctl('terminate', this.device, applicationId)
    } finally {
      this.shutdownSimulator()
    }
    console.log(`PASS iOS simulator (${device}): ${this.fixture.passed(AsyncListenerApp.names[this.appleLanguage])}`)
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
    const language = AsyncListenerApp.names[this.androidLanguage]
    const { name, applicationId } = this.fixture.stage(project, this.androidLanguage)
    this.run(this.binary, ['bundle', project, '--target', 'android'])
    this.fixture.assertLinked(path.join(project, 'dist/android'), language)
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
      await page.waitForFunction((probe) => ['done', 'error'].includes(globalThis.__tachyonTac?.service(probe)?.phase), this.fixture.probeService, { timeout: this.proofTimeout })
      const record = await page.evaluate((probe) => ({ phase: __tachyonTac.service(probe).phase, proof: __tachyonTac.service(probe).proof }), this.fixture.probeService)
      const proof = this.fixture.assertProof(record, Number(pid))
      console.log(`==> android: ${JSON.stringify(proof)}`)
    } finally {
      await browser.close()
      this.run(this.adb(), ['forward', '--remove', `tcp:${port}`])
      this.adbShell('am', 'force-stop', packageName)
      spawnSync(this.adb(), ['uninstall', packageName], { stdio: 'ignore', timeout: 120000 })
    }
    console.log(`PASS Android emulator: ${this.fixture.passed(language)}`)
  }

  static async main() {
    assert.ok(this.fixture, `unknown --check=${this.check}; use ${Object.keys(this.checks).join(' or ')}`)
    assert.ok(['swift', 'rs'].includes(this.appleLanguage), `unknown TAC_LISTENER_LANGUAGE ${this.appleLanguage}; use swift or rs`)
    assert.ok(['kt', 'rs'].includes(this.androidLanguage), `unknown TAC_ANDROID_LISTENER_LANGUAGE ${this.androidLanguage}; use kt or rs`)
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

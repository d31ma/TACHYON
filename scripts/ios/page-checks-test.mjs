#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer as createHttpServer } from 'node:http'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'

class IosPageChecksTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-page-checks-'))
  static deviceSet = path.join(IosPageChecksTest.fixture, 'simulator-devices')
  static evidence = process.env.TAC_IOS_PAGE_CHECKS_EVIDENCE ?? path.join(IosPageChecksTest.repository, 'target/ios-page-checks-evidence')
  static application = 'PageChecks'
  static applicationId = 'dev.tachyon.ios-page-checks'
  static pageOrigin = 'tachyon-app://bundle'
  static otherApplication = 'com.apple.Preferences'
  static waitMilliseconds = 90000
  static linkMilliseconds = 8000
  static pollMilliseconds = 250
  static device = ''
  static tapper = path.join(IosPageChecksTest.fixture, 'simulator-tap')
  static links = [
    { name: 'https', url: 'https://example.com/tachyon-rm2-https', opens: true },
    { name: 'http', url: 'http://example.com/tachyon-rm2-http', opens: true },
    { name: 'https-tapped', url: 'https://example.com/tachyon-rm2-tapped', tap: true, opens: true },
    { name: 'https-new-window-script', url: 'https://example.com/tachyon-rm2-blank-script', target: '_blank', opens: 'popup blocked without a tap' },
    { name: 'https-new-window-tapped', url: 'https://example.com/tachyon-rm2-blank-tapped', target: '_blank', tap: true, opens: true },
    { name: 'mailto', url: 'mailto:rm2@example.com?subject=tachyon-rm2', opens: 'mail' },
    { name: 'user-name', url: 'https://someone@example.com/tachyon-rm2-user', opens: false },
    { name: 'password', url: 'https://someone:secret@example.com/tachyon-rm2-password', opens: false },
    { name: 'other-scheme', url: 'ftp://example.com/tachyon-rm2-ftp', opens: false },
    { name: 'empty-mailto', url: 'mailto:', opens: false },
  ]

  static clientFiles(addresses) {
    return {
      'client/pages/tac.html': '<page-checks hydrate="load" />\n',
      'client/pages/other/tac.html': '<other-visit hydrate="load" />\n',
      'client/components/other/visit/tac.html': '<h1>The other page</h1>\n<p>Going back {otherVisit.back}</p>\n',
      'client/services/OtherVisitService.js': `@Service
export default class OtherVisitService {
  static BACK = '/?visited=other';
  static PAUSE = 300;
  back = 'soon';

  @OnStart
  open() {
    setTimeout(() => { location.assign(OtherVisitService.BACK); }, OtherVisitService.PAUSE);
  }
}
`,
      'client/components/page/checks/tac.html': '<main><a id="outside" :href="pageChecks.linkAddress" :target="pageChecks.linkTarget">Outside link</a><h1>Page checks</h1><p id="phase">{pageChecks.phase}</p><p id="heard">State {pageChecks.heard}</p><p id="report">{pageChecks.report}</p></main>\n',
      'client/components/page/checks/tac.css': '#outside { display: block; min-height: 50vh; font-size: 32px; }\n',
      'client/setups/ReachSetup.js': `@Setup
export default class ReachSetup {
  static _ADDRESSES = ${JSON.stringify(addresses)};
  static _REPORT_WAIT = 2000;

  /** @returns {object} */
  static configure() {
    return { http: { reach: { exchange: (request) => ReachSetup._exchange(request) } } };
  }

  /**
   * @param {{ path: string }} request
   * @returns {Promise<{ outcome: string }>}
   */
  static async _exchange(request) {
    const report = ReachSetup._policyReport();
    try {
      const response = await fetch(ReachSetup._ADDRESSES[request.path], { credentials: 'omit', cache: 'no-store' });
      return { outcome: \`reached \${response.status}\` };
    } catch {
      return { outcome: \`refused by \${await report}\` };
    }
  }

  /** @returns {Promise<string>} */
  static _policyReport() {
    return new Promise((resolve) => {
      document.addEventListener('securitypolicyviolation', (event) => resolve(event.effectiveDirective), { once: true });
      setTimeout(() => resolve('no policy report'), ReachSetup._REPORT_WAIT);
    });
  }
}
`,
      'client/remotes/NoteClient.js': `@HTTPClient
@CacheSettings({"name":"notes","ttlMs":1,"maxBytes":4096,"maxEntries":8})
export default class NoteClient {
  /** @returns {Promise<{ text: string }>} */
  @Cached
  @GetRequest("/api/notes")
  async note() {}

  /** @returns {Promise<{ text: string }>} */
  @Cached
  @GetRequest("/api/kept")
  async kept() {}

  /** @returns {Promise<{ text: string }>} */
  @GetRequest("/api/fresh")
  async fresh() {}
}
`,
      'client/services/PageChecksService.js': `@Service
export default class PageChecksService {
  static ADAPTER = 'reach';
  static KEY = 'theme';
  static DARK = 'dark';
  static READS = 'tachyon-yon-reads';
  static PAUSE = 200;
  static LINK_WAIT = 600000;
  static OTHER_PAGE = '/other/';
  static REMEMBERED = 'remembered';
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {PreferenceStrings} */
  @Needs
  preferenceStrings;
  /** @type {NoteClient} */
  @Needs
  noteClient;
  @Needs
  taskScheduler;
  /** @type {BehaviorContext | null} */
  behaviors = null;
  phase = 'starting';
  proof = '';
  report = 'Checking';
  heard = 'none';
  linkAddress = '/';
  linkTarget = '_self';
  @SessionState
  visit = 'none';

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(ReachSetup, { partition: 'reach', signal: this.lifecycle });
    setTimeout(() => { void this.check(); }, 50);
  }

  /** @param {ApplicationStateChanged} changed */
  @OnEvent({"replay":true})
  hear(changed) {
    this.heard = changed.state;
    void this.pageHeard(changed.state);
  }

  async check() {
    const facts = {};
    try {
      facts.themeBefore = this.preferenceStrings.read(PageChecksService.KEY);
      if (facts.themeBefore === PageChecksService.DARK) {
        facts.note = await this.answer(() => this.noteClient.note());
        facts.kept = await this.answer(() => this.noteClient.kept());
        facts.keptPaths = await this.keptPaths();
        facts.visitAfterRelaunch = this.visit;
        await this.finish('second', facts);
        return;
      }
      if (this.visit !== PageChecksService.REMEMBERED) {
        this.visit = PageChecksService.REMEMBERED;
        location.assign(PageChecksService.OTHER_PAGE);
        return;
      }
      facts.visitAfterNavigation = this.visit;
      facts.visitedFrom = new URLSearchParams(location.search).get('visited');
      this.preferenceStrings.write(PageChecksService.KEY, PageChecksService.DARK);
      facts.themeAfter = this.preferenceStrings.read(PageChecksService.KEY);
      facts.undeclared = await this.reach('/undeclared');
      facts.yon = await this.reach('/yon');
      facts.noteOnline = [await this.answer(() => this.noteClient.note()), await this.later(() => this.noteClient.note())];
      facts.keptOnline = [await this.answer(() => this.noteClient.kept()), await this.later(() => this.noteClient.kept())];
      facts.keptPathsOnline = await this.keptPaths();
      await this.ask('stop-yon', 'yon-stopped');
      facts.noteOffline = await this.answer(() => this.noteClient.note());
      facts.keptOffline = await this.answer(() => this.noteClient.kept());
      facts.freshOffline = await this.answer(() => this.noteClient.fresh());
      facts.keptPaths = await this.keptPaths();
      await this.finish('first', facts);
      await this.followLinks();
    } catch (error) {
      facts.error = String(error);
      await this.finish('error', facts);
    }
  }

  /**
   * @param {() => Promise<{ text: string }>} read
   * @returns {Promise<string>}
   */
  async answer(read) {
    return read().then((note) => note.text, (error) => \`failed \${error.code ?? error.message}\`);
  }

  /**
   * @param {() => Promise<{ text: string }>} read
   * @returns {Promise<string>}
   */
  async later(read) {
    await new Promise((resolve) => setTimeout(resolve, PageChecksService.PAUSE));
    return this.answer(read);
  }

  /** @returns {Promise<string[]>} */
  async keptPaths() {
    const cache = await caches.open(PageChecksService.READS);
    return (await cache.keys()).map((request) => new URL(request.url).pathname);
  }

  /**
   * @param {string} target
   * @returns {Promise<string>}
   */
  async reach(target) {
    return this.behaviors.restClient(PageChecksService.ADAPTER).get(target).retrieve().then((answer) => answer.outcome, (error) => \`failed: \${error.message}\`);
  }

  /**
   * @param {string} phase
   * @param {string} flag
   */
  async ask(phase, flag) {
    this.phase = phase;
    this.proof = '{}';
    await this.record();
    while (!(await this.flag(flag))) await new Promise((resolve) => setTimeout(resolve, PageChecksService.PAUSE));
  }

  /**
   * @param {string} phase
   * @param {object} facts
   */
  async finish(phase, facts) {
    this.phase = phase;
    this.proof = JSON.stringify(facts);
    this.report = this.proof;
    await this.record();
    await this.taskScheduler.render();
  }

  async followLinks() {
    const deadline = Date.now() + PageChecksService.LINK_WAIT;
    while (Date.now() < deadline) {
      const asked = await this.nextLink();
      if (asked === 'done') return;
      if (asked) await this.follow(JSON.parse(asked));
      await new Promise((resolve) => setTimeout(resolve, PageChecksService.PAUSE));
    }
  }

  /** @param {{ url: string, target?: string, tap?: boolean }} link */
  async follow(link) {
    this.linkAddress = link.url;
    this.linkTarget = link.target ?? '_self';
    await this.taskScheduler.render();
    if (!link.tap) document.querySelector('#outside').click();
  }
}
`,
      'client/services/PageChecksService.swift': `import Foundation
@Service
final class PageChecksService {
    var phase: String = "starting"
    var proof: String = ""

    func record() -> Int {
        let phases = ["first", "second", "stop-yon", "error"]
        let safePhase = phases.contains(phase) ? phase : "error"
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier), "proof": proof]
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: documents().appendingPathComponent("page-checks-" + safePhase + ".json"), options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }

    func flag(_ name: String) -> Bool {
        let flags = ["yon-stopped"]
        return flags.contains(name) && FileManager.default.fileExists(atPath: documents().appendingPathComponent(name).path)
    }

    func nextLink() -> String {
        let file = documents().appendingPathComponent("open-link.json")
        guard let text = try? String(contentsOf: file, encoding: .utf8) else { return "" }
        try? FileManager.default.removeItem(at: file)
        return text
    }

    func pageHeard(_ state: String) -> Int {
        append("page:" + state)
        return 0
    }

    @OnEvent
    func nativeHeard(_ changed: ApplicationStateChanged) {
        append("native:" + changed.state)
    }

    @OnStop
    func stopped() {
        append("native:stopped")
    }

    private func documents() -> URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }

    private func append(_ line: String) {
        let file = documents().appendingPathComponent("state.log")
        let data = Data((line + "\\n").utf8)
        guard let handle = try? FileHandle(forWritingTo: file) else {
            try? data.write(to: file)
            return
        }
        handle.seekToEndOfFile()
        handle.write(data)
        handle.closeFile()
    }
}
`,
    }
  }

  static serverFiles = {
    'server/controllers/NoteController.js': `@RequestRoute("/api/notes")
@Public
class NoteController {
  @GetRoute
  getNote() {
    return Response.json({ text: \`Note \${Date.now()}\` }, { headers: { 'cache-control': 'public, no-cache' } })
  }
}
`,
    'server/controllers/KeptController.js': `@RequestRoute("/api/kept")
@Public
class KeptController {
  @GetRoute
  getKept() {
    return Response.json({ text: \`Kept \${Date.now()}\` }, { headers: { 'cache-control': 'public, max-age=3600' } })
  }
}
`,
    'server/controllers/FreshController.js': `@RequestRoute("/api/fresh")
@Public
class FreshController {
  @GetRoute
  getFresh() {
    return { text: 'Fresh from Yon' }
  }
}
`,
  }

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args])
  }

  static write(name, text) {
    const target = path.join(this.fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
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

  static prepare(yonOrigin, undeclaredOrigin) {
    const addresses = { '/undeclared': `${undeclaredOrigin}/probe`, '/yon': `${yonOrigin}/api/fresh` }
    for (const [name, text] of Object.entries({ ...this.clientFiles(addresses), ...this.serverFiles })) this.write(name, text)
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    this.write('client/manifest.json', JSON.stringify({
      name: this.application,
      start_url: '/',
      tac: {
        application: { id: this.applicationId, version: '0.0.1' },
        ...checks,
        yon: { origin: yonOrigin },
        cache: [{ path: '/api/notes', policy: 'network-first' }, { path: '/api/kept', policy: 'cache-first' }],
        browserApis: ['preferenceStrings'],
      },
    }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [this.pageOrigin] } }))
    this.run(this.binary, ['build', this.fixture, '--target', 'ios'])
  }

  static sources(folder) {
    return readdirSync(folder, { recursive: true }).filter((name) => name.endsWith('.swift')).map((name) => readFileSync(path.join(folder, name), 'utf8'))
  }

  static checkBuild(yonOrigin, undeclaredOrigin) {
    const output = path.join(this.fixture, 'dist/ios')
    const policy = this.sources(output).join('\n').match(/connect-src [^;"]*/)?.[0] ?? ''
    assert.ok(policy.includes(yonOrigin), `connect-src lacks the Yon origin ${yonOrigin}: ${policy}`)
    assert.ok(!policy.includes(undeclaredOrigin), `connect-src lists the undeclared ${undeclaredOrigin}: ${policy}`)
    const bundle = path.join(output, `${this.application}.app/WebBundle`)
    assert.ok(existsSync(path.join(bundle, '.tachyon/tac-yon-reads.js')), 'the bundle has no .tachyon/tac-yon-reads.js')
    assert.match(readFileSync(path.join(bundle, 'index.html'), 'utf8'), /tachyon-browser-apis" content="preferenceStrings"/)
    return policy
  }

  static async undeclaredHost() {
    const port = await this.freePort()
    const host = { origin: `http://127.0.0.1:${port}`, requests: [] }
    host.server = createHttpServer((request, response) => {
      host.requests.push(`${request.method} ${request.url} from ${request.headers.origin ?? 'no origin'}`)
      response.writeHead(200, { 'content-type': 'application/json', 'access-control-allow-origin': '*' })
      response.end('{"reached":true}')
    })
    await new Promise((resolve) => host.server.listen(port, '127.0.0.1', resolve))
    return host
  }

  static bootSimulator() {
    mkdirSync(this.deviceSet)
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.name === 'iPhone 17 Pro')
      ?? (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `PageChecks-${randomUUID()}`, phone.identifier, runtime.identifier)
    console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    this.simctl('boot', this.device)
    this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'bootstatus', this.device, '-b'])
    return { runtime: runtime.version, device: phone.name }
  }

  static shutdownSimulator() {
    if (!this.device) return
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
  }

  static launch() {
    const launched = this.simctl('launch', this.device, this.applicationId)
    const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1])
    assert.ok(Number.isSafeInteger(pid) && pid > 0, `simctl gave no app PID: ${launched}`)
    return pid
  }

  static documents() {
    return path.join(this.simctl('get_app_container', this.device, this.applicationId, 'data'), 'Documents')
  }

  static async waitFor(description, check, milliseconds = this.waitMilliseconds) {
    const deadline = Date.now() + milliseconds
    while (Date.now() < deadline) {
      const failure = path.join(this.documents(), 'page-checks-error.json')
      assert.ok(!existsSync(failure), existsSync(failure) ? readFileSync(failure, 'utf8') : '')
      const value = check()
      if (value) return value
      await sleep(this.pollMilliseconds)
    }
    return null
  }

  static async proof(phase) {
    const file = path.join(this.documents(), `page-checks-${phase}.json`)
    const found = await this.waitFor(phase, () => existsSync(file))
    assert.ok(found, `the page wrote no ${phase} proof`)
    const record = JSON.parse(readFileSync(file, 'utf8'))
    return { pid: record.pid, facts: JSON.parse(record.proof) }
  }

  static stateLines() {
    const file = path.join(this.documents(), 'state.log')
    return existsSync(file) ? readFileSync(file, 'utf8').trim().split('\n').filter(Boolean) : []
  }

  static async heard(start, lines, milliseconds = this.waitMilliseconds) {
    const found = await this.waitFor(lines.join(' '), () => {
      const later = this.stateLines().slice(start)
      return lines.every((line) => later.includes(line)) ? later : null
    }, milliseconds)
    return found ?? this.stateLines().slice(start)
  }

  static screenshot(name) {
    const file = path.join(this.fixture, `${name}.png`)
    this.simctl('io', this.device, 'screenshot', file)
    copyFileSync(file, path.join(this.evidence, `${name}.png`))
    return path.join(this.evidence, `${name}.png`)
  }

  static openRequests(start) {
    const result = spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'spawn', this.device, 'log', 'show', '--style', 'compact', '--start', start,
      '--predicate', 'eventMessage CONTAINS[c] "OpenURL from" OR eventMessage CONTAINS[c] "open URL"'], { encoding: 'utf8', timeout: 120000, maxBuffer: 16 * 1024 * 1024 })
    return (result.stdout ?? '').split('\n').filter((line) => /open ?url/i.test(line) && !line.includes(' log[')).slice(0, 12)
  }

  static clock() {
    return this.run('/bin/date', ['+%Y-%m-%d %H:%M:%S'])
  }

  static async backgroundAndForeground() {
    const settled = await this.heard(0, ['page:foreground'], 5000)
    const beforeBackground = this.stateLines().length
    this.simctl('launch', this.device, this.otherApplication)
    const background = await this.heard(beforeBackground, ['native:background', 'page:background'])
    const backgroundShot = this.screenshot('state-background')
    const beforeForeground = this.stateLines().length
    this.launch()
    const foreground = await this.heard(beforeForeground, ['native:foreground', 'page:foreground'])
    const foregroundShot = this.screenshot('state-foreground')
    return { settled, background, foreground, screenshots: [backgroundShot, foregroundShot] }
  }

  static async followLink(link) {
    const start = this.clock()
    const before = this.stateLines().length
    writeFileSync(path.join(this.documents(), 'open-link.json'), JSON.stringify({ url: link.url, target: link.target, tap: link.tap }))
    if (link.tap) {
      await sleep(1500)
      this.run(this.tapper, [this.deviceSet, this.device, '0.5', '0.3'])
    }
    const left = await this.waitFor(`${link.name} link`, () => this.stateLines().slice(before).includes('native:background'), this.linkMilliseconds)
    const shot = this.screenshot(`link-${link.name}`)
    const log = this.openRequests(start)
    if (left) {
      const returning = this.stateLines().length
      this.launch()
      await this.heard(returning, ['native:foreground'])
    }
    return { name: link.name, url: link.url, target: link.target ?? null, tapped: Boolean(link.tap), expected: link.opens, leftTheApp: Boolean(left), screenshot: shot, log }
  }

  static async relaunch(firstPid) {
    const before = this.stateLines().length
    const telemetry = path.join(this.simctl('get_app_container', this.device, this.applicationId, 'data'), `Library/Logs/Tachyon/${this.applicationId}.jsonl`)
    writeFileSync(path.join(this.documents(), 'open-link.json'), 'done')
    await sleep(1000)
    this.simctl('terminate', this.device, this.applicationId)
    await sleep(2000)
    const stopped = this.stateLines().slice(before)
    const events = existsSync(telemetry) ? readFileSync(telemetry, 'utf8').trim().split('\n').map((line) => JSON.parse(line).event) : []
    for (const name of ['page-checks-first.json', 'page-checks-stop-yon.json']) rmSync(path.join(this.documents(), name), { force: true })
    const pid = this.launch()
    assert.notEqual(pid, firstPid, 'the relaunch must start a new process')
    const second = await this.proof('second')
    assert.equal(second.pid, pid, 'the second proof must come from the relaunched process')
    this.screenshot('relaunch')
    return { pid, second: second.facts, terminate: { stateLog: stopped, telemetry: events } }
  }

  static checkFirst(facts, undeclared) {
    assert.equal(facts.themeBefore, null, 'a fresh app has no preference string')
    assert.equal(facts.themeAfter, 'dark', 'the preference string reads back at once')
    assert.equal(facts.undeclared, 'refused by connect-src', `the undeclared host: ${facts.undeclared}`)
    assert.deepEqual(undeclared.requests, [], `the undeclared host saw requests: ${undeclared.requests.join(', ')}`)
    assert.equal(facts.yon, 'reached 200', `the Yon origin: ${facts.yon}`)
    assert.match(facts.noteOnline[0], /^Note \d+$/)
    assert.notEqual(facts.noteOnline[0], facts.noteOnline[1], 'a network-first read goes to Yon while it runs')
    assert.match(facts.keptOnline[0], /^Kept \d+$/)
    assert.equal(facts.keptOnline[1], facts.keptOnline[0], 'a cache-first read answers from the kept copy while Yon runs')
    assert.equal(facts.noteOffline, facts.noteOnline[1], 'a network-first read answers from the kept copy with Yon stopped')
    assert.equal(facts.keptOffline, facts.keptOnline[0], 'a cache-first read answers from the kept copy with Yon stopped')
    assert.match(facts.freshOffline, /^failed/, 'a read without a cache rule fails with Yon stopped')
    assert.deepEqual([...facts.keptPaths].sort(), ['/api/kept', '/api/notes'])
    assert.equal(facts.visitedFrom, 'other', 'the first page came back from the other page')
    assert.equal(facts.visitAfterNavigation, 'remembered', 'a @SessionState field is kept across a navigation to another page and back')
  }

  static async main() {
    const started = Date.now()
    mkdirSync(this.evidence, { recursive: true })
    const yonPort = await this.freePort()
    const yonOrigin = `http://127.0.0.1:${yonPort}`
    const undeclared = await this.undeclaredHost()
    const report = { yonOrigin, undeclaredOrigin: undeclared.origin }
    try {
      this.run('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation', '-framework', 'AppKit',
        path.join(this.repository, 'scripts/ios/simulator-tap.m'), '-o', this.tapper])
      this.prepare(yonOrigin, undeclared.origin)
      report.connectSrc = this.checkBuild(yonOrigin, undeclared.origin)
      Object.assign(report, this.bootSimulator())
      this.simctl('install', this.device, path.join(this.fixture, 'dist/ios', `${this.application}.app`))
      let firstPid = 0
      await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async () => {
        firstPid = this.launch()
        await this.proof('stop-yon')
        console.log(`==> ${undeclared.origin} saw ${undeclared.requests.length} requests; stopping Yon`)
      })
      writeFileSync(path.join(this.documents(), 'yon-stopped'), 'stopped\n')
      const first = await this.proof('first')
      assert.equal(first.pid, firstPid, 'the first proof must come from the launched app')
      report.first = first.facts
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      this.screenshot('first-launch')
      this.checkFirst(first.facts, undeclared)
      console.log('==> first launch passed; sending the app to the background and back')
      report.state = await this.backgroundAndForeground()
      report.links = []
      for (const link of this.links) {
        report.links.push(await this.followLink(link))
        writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      }
      report.relaunch = await this.relaunch(firstPid)
      assert.equal(report.relaunch.second.themeBefore, 'dark', 'the preference string written before the quit is read after the relaunch')
      assert.equal(report.relaunch.second.visitAfterRelaunch, 'none', 'a @SessionState field is gone after a terminate and relaunch')
      assert.equal(report.relaunch.second.note, report.first.noteOffline, 'a network-first read kept before the quit answers after the relaunch with Yon stopped')
      assert.equal(report.relaunch.second.kept, report.first.keptOffline, 'a cache-first read kept before the quit answers after the relaunch with Yon stopped')
      for (const link of report.links.filter(({ expected }) => typeof expected === 'boolean')) {
        assert.equal(link.leftTheApp, link.expected, `the ${link.name} link ${link.expected ? 'must open another app' : 'must open nothing'}`)
      }
    } finally {
      report.seconds = Math.round((Date.now() - started) / 1000)
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      this.shutdownSimulator()
      await new Promise((resolve) => undeclared.server.close(resolve))
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.fixture, { recursive: true, force: true })
    }
    console.log(JSON.stringify({ state: report.state, links: report.links.map(({ name, expected, leftTheApp }) => ({ name, expected, leftTheApp })), relaunch: report.relaunch }, null, 2))
    console.log(`PASS: on the iOS ${report.runtime} simulator (${report.device}), a @SessionState field was kept across a navigation to another page and back and was gone after a relaunch; connect-src refused ${undeclared.origin} while the Yon origin answered; network-first and cache-first Yon reads answered from the kept copy with Yon stopped; a preference string survived a terminate and relaunch (${report.seconds} s). Report: ${path.join(this.evidence, 'report.json')}`)
  }
}

await IosPageChecksTest.main()

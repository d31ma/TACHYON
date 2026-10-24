#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

class IosRustCrateTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-rust-crate-'))
  static project = path.join(IosRustCrateTest.root, 'project')
  static deviceSet = path.join(IosRustCrateTest.root, 'simulator-devices')
  static evidence = process.env.TAC_IOS_RUST_CRATE_EVIDENCE ?? path.join(IosRustCrateTest.repository, 'target/ios-rust-crate-evidence')
  static application = 'Greeter'
  static applicationId = 'dev.tachyon.ios-rust-crate'
  static waitMilliseconds = 120000
  static pollMilliseconds = 250
  static device = ''

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
}
`,
    'client/components/greeter/panel/tac.html': '<main>\n  <h1>Greeter</h1>\n  <p id="greeting">{greeting.text}</p>\n  <button id="greet" on:click="greeting.greet()">Greet</button>\n</main>\n',
    'client/pages/tac.html': '<greeter-panel />\n<greeter-probe />\n',
    'client/components/greeter/probe/tac.html': '<p id="probe">Probe {greeterProbe.phase}</p>\n',
    'client/services/GreeterProbeService.js': `@Service
export default class GreeterProbeService {
  static WAIT = 60000;
  static PAUSE = 100;
  phase = 'starting';
  proof = '';

  @OnStart
  initialize() {
    setTimeout(() => { void this.probe(); }, 50);
  }

  async probe() {
    try {
      await this.until(() => document.querySelector('#greet'));
      const before = document.querySelector('#greeting').textContent;
      document.querySelector('#greet').click();
      await this.until(() => document.querySelector('#greeting').textContent !== before);
      this.phase = 'greeted';
      this.proof = JSON.stringify({ before, after: document.querySelector('#greeting').textContent });
    } catch (error) {
      this.phase = 'error';
      this.proof = JSON.stringify({ error: String(error), text: document.body.innerText.slice(0, 400) });
    }
    await this.record();
  }

  /** @param {() => unknown} check */
  async until(check) {
    const deadline = Date.now() + GreeterProbeService.WAIT;
    while (Date.now() < deadline) {
      if (check()) return;
      await new Promise((resolve) => setTimeout(resolve, GreeterProbeService.PAUSE));
    }
    throw new Error('The page did not become ready');
  }
}
`,
    'client/services/GreeterProbeService.swift': `import Foundation
@Service
final class GreeterProbeService {
    var phase: String = "starting"
    var proof: String = ""

    func record() -> Int {
        let safePhase = ["greeted", "error"].contains(phase) ? phase : "error"
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier), "proof": proof]
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: documents.appendingPathComponent("greeter-" + safePhase + ".json"), options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }
}
`,
  }

  static crates = {
    'tiny-greeting': 'pub fn hello() -> String { String::from("hello") }\n',
    'tiny-platform': 'pub fn name() -> &\'static str { "ios" }\n',
  }

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args])
  }

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
          rust: { dependencies: { 'tiny-greeting': '=1.0.0' }, platformDependencies: { ios: { 'tiny-platform': '=1.0.0' } } },
        },
      },
    }))
  }

  static checkBuild() {
    const crate = readFileSync(path.join(this.project, 'dist/ios/project/rust-companion/Cargo.toml'), 'utf8')
    for (const name of Object.keys(this.crates)) assert.ok(crate.includes(`${name} = { version = "=1.0.0" }`), `${name} is not in the Rust companion crate:\n${crate}`)
    const executable = readFileSync(path.join(this.project, 'dist/ios', `${this.application}.app`, this.application))
    assert.ok(executable.includes('GreetingService'), 'the Rust service is not linked into the app')
    return crate
  }

  static bootSimulator() {
    mkdirSync(this.deviceSet)
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `RustCrate-${randomUUID()}`, phone.identifier, runtime.identifier)
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

  static async proof(pid) {
    const documents = path.join(this.simctl('get_app_container', this.device, this.applicationId, 'data'), 'Documents')
    const deadline = Date.now() + this.waitMilliseconds
    const files = ['greeted', 'error'].map((phase) => path.join(documents, `greeter-${phase}.json`))
    while (Date.now() < deadline && !files.some((file) => existsSync(file))) await sleep(this.pollMilliseconds)
    const file = files.find((candidate) => existsSync(candidate))
    assert.ok(file, 'the page wrote no proof')
    const record = JSON.parse(readFileSync(file, 'utf8'))
    assert.equal(record.pid, pid, 'the proof must come from the launched app')
    return { phase: path.basename(file, '.json'), ...JSON.parse(record.proof) }
  }

  static async main() {
    mkdirSync(this.evidence, { recursive: true })
    const report = {}
    try {
      this.prepare()
      this.run(this.binary, ['build', this.project, '--target', 'ios'], 900000)
      report.crate = this.checkBuild()
      Object.assign(report, this.bootSimulator())
      this.simctl('install', this.device, path.join(this.project, 'dist/ios', `${this.application}.app`))
      const launched = this.simctl('launch', this.device, this.applicationId)
      const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1])
      report.proof = await this.proof(pid)
      const screenshot = path.join(this.root, 'greeted.png')
      this.simctl('io', this.device, 'screenshot', screenshot)
      copyFileSync(screenshot, path.join(this.evidence, 'greeted.png'))
      assert.equal(report.proof.after, 'hello from ios', `the Rust service did not answer with its vendored crates: ${JSON.stringify(report.proof)}`)
    } finally {
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      this.shutdownSimulator()
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    console.log(`PASS: a Rust service using vendored crates (tiny-greeting, and tiny-platform for iOS only) answered "${report.proof.after}" in the iOS ${report.runtime} simulator app (${report.device}).`)
  }
}

await IosRustCrateTest.main()

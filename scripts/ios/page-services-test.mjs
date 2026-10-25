#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

class IosPageServicesTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-page-services-'))
  static deviceSet = path.join(IosPageServicesTest.root, 'simulator-devices')
  static evidence = process.env.TAC_IOS_PAGE_SERVICES_EVIDENCE ?? path.join(IosPageServicesTest.repository, 'target/ios-page-services-evidence')
  static feature = path.join(IosPageServicesTest.repository, 'website/content/features/web-assembly-services')
  static languages = (process.env.TAC_IOS_SERVICE_LANGUAGES ?? 'py,cs,kt,dart,java,go,rb,lua,hpp,fs,rs').split(',').map((item) => item.trim()).filter(Boolean)
  static laterFloors = { dart: '18.2', kt: '18.2', java: '18.4' }
  static aheadOfTime = process.env.TAC_IOS_DOTNET_AOT !== 'false'
  static linked = ['rs', 'swift']
  static scopedColor = 'rgb(10, 20, 30)'
  static waitMilliseconds = 300000
  static pollMilliseconds = 250
  static device = ''
  static runtimeVersion = process.env.TAC_IOS_RUNTIME ?? ''

  static probeFiles = {
    'client/components/services/probe/tac.html': '<p id="probe">Probe {servicesProbe.phase}</p>\n<a id="to-item" href="/items/7/">Open item seven</a>\n',
    'client/components/services/probe/tac.css': `p { color: ${IosPageServicesTest.scopedColor}; }\n`,
    'client/components/services/item/tac.html': '<h1 id="item">Item route</h1>\n<p>Probe {servicesProbe.phase}</p>\n',
    'client/pages/items/_id/tac.html': '<services-item />\n',
    'client/services/ServicesProbeService.js': `@Service
export default class ServicesProbeService {
  static WAIT = 240000;
  static PAUSE = 100;
  phase = 'starting';
  proof = '';

  @OnStart
  initialize() {
    setTimeout(() => { void this.probe(); }, 50);
  }

  async probe() {
    try {
      if (location.pathname.startsWith('/items/')) {
        await this.until(() => document.querySelector('#item')?.textContent === 'Item route');
        await this.finish('item', { path: location.pathname, heading: document.querySelector('#item').textContent });
        return;
      }
      await this.until(() => this.paragraph('0 in the basket'));
      const loaded = Math.round(performance.now());
      [...document.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Add two').click();
      await this.until(() => this.paragraph('2 in the basket') && this.paragraph('2 items'));
      await this.finish('root', {
        loaded,
        basket: this.paragraph('in the basket').textContent.trim(),
        summary: this.paragraph('items').textContent.trim(),
        scoped: getComputedStyle(document.querySelector('#probe')).color,
        outside: getComputedStyle(this.paragraph('in the basket')).color,
      });
      document.querySelector('#to-item').click();
    } catch (error) {
      await this.finish('error', { error: String(error), text: document.body.innerText.slice(0, 400) });
    }
  }

  /**
   * @param {string} text
   * @returns {HTMLElement | undefined}
   */
  paragraph(text) {
    return [...document.querySelectorAll('p')].find((element) => element.textContent.includes(text));
  }

  /** @param {() => unknown} check */
  async until(check) {
    const deadline = Date.now() + ServicesProbeService.WAIT;
    while (Date.now() < deadline) {
      if (check()) return;
      await new Promise((resolve) => setTimeout(resolve, ServicesProbeService.PAUSE));
    }
    throw new Error('The page did not become ready');
  }

  /**
   * @param {string} phase
   * @param {object} facts
   */
  async finish(phase, facts) {
    this.phase = phase;
    this.proof = JSON.stringify(facts);
    await this.record();
  }
}
`,
    'client/services/ServicesProbeService.swift': `import Foundation
@Service
final class ServicesProbeService {
    var phase: String = "starting"
    var proof: String = ""

    func record() -> Int {
        let safePhase = ["root", "item", "error"].contains(phase) ? phase : "error"
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier), "proof": proof]
        let documents = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: documents.appendingPathComponent("probe-" + safePhase + ".json"), options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }
}
`,
  }

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return `${result.stdout}${result.stderr}`.trim()
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args])
  }

  /** @param {string} language */
  static stage(language) {
    const project = path.join(this.root, `tally-${language}`)
    const source = path.join(this.feature, 'client')
    for (const entry of ['pages', 'components']) cpSync(path.join(source, entry), path.join(project, 'client', entry), { recursive: true })
    mkdirSync(path.join(project, 'client/services'), { recursive: true })
    copyFileSync(path.join(source, 'services', `TallyService.${language}`), path.join(project, 'client/services', `TallyService.${language}`))
    for (const [name, text] of Object.entries(this.probeFiles)) {
      mkdirSync(path.dirname(path.join(project, name)), { recursive: true })
      writeFileSync(path.join(project, name), text)
    }
    writeFileSync(path.join(project, 'client/pages/tac.html'), `${readFileSync(path.join(source, 'pages/tac.html'), 'utf8').trimEnd()}\n<services-probe />\n`)
    const manifest = JSON.parse(readFileSync(path.join(source, 'manifest.json'), 'utf8'))
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const application = `Tally${language[0].toUpperCase()}${language.slice(1)}`
    manifest.name = application
    manifest.start_url = '/'
    manifest.tac = { ...manifest.tac, ...checks, application: { id: `dev.tachyon.tally-${language}`, version: '0.0.1' }, dotnet: { aot: this.aheadOfTime } }
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest, null, 2))
    return { project, application, applicationId: manifest.tac.application.id }
  }

  /** @param {string} version */
  static chosenVersion(version) {
    return !this.runtimeVersion || version === this.runtimeVersion || version.startsWith(`${this.runtimeVersion}.`)
  }

  static bootSimulator() {
    mkdirSync(this.deviceSet)
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-') && this.chosenVersion(item.version))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.name === 'iPhone 17 Pro')
      ?? (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `PageServices-${randomUUID()}`, phone.identifier, runtime.identifier)
    console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    this.simctl('boot', this.device)
    this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'bootstatus', this.device, '-b'], 600000)
    return { runtime: runtime.version, device: phone.name }
  }

  static shutdownSimulator() {
    if (!this.device) return
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
  }

  /** @param {string} documents @param {string} phase @param {number} pid */
  static async proof(documents, phase, pid) {
    const file = path.join(documents, `probe-${phase}.json`)
    const failure = path.join(documents, 'probe-error.json')
    const deadline = Date.now() + this.waitMilliseconds
    while (Date.now() < deadline && !existsSync(file)) {
      assert.ok(!existsSync(failure), existsSync(failure) ? readFileSync(failure, 'utf8') : '')
      await sleep(this.pollMilliseconds)
    }
    assert.ok(existsSync(file), `the page wrote no ${phase} proof`)
    const record = JSON.parse(readFileSync(file, 'utf8'))
    assert.equal(record.pid, pid, `the ${phase} proof must come from the launched app`)
    return JSON.parse(record.proof)
  }

  /** @param {string} name */
  static screenshot(name) {
    const file = path.join(this.root, `${name}.png`)
    this.simctl('io', this.device, 'screenshot', file)
    copyFileSync(file, path.join(this.evidence, `${name}.png`))
    return path.join(this.evidence, `${name}.png`)
  }

  /** @param {string} folder */
  static webAssembly(folder) {
    const files = readdirSync(folder, { recursive: true }).filter((name) => name.endsWith('.wasm'))
    return Object.fromEntries(files.map((name) => [name, statSync(path.join(folder, name)).size]).sort())
  }

  /** @param {string} language */
  static async check(language) {
    const { project, application, applicationId } = this.stage(language)
    const started = Date.now()
    const output = this.run(this.binary, ['bundle', project, '--target', 'ios'], 1800000)
    const built = Math.round((Date.now() - started) / 1000)
    const bundle = path.join(project, 'dist/ios', `${application}.app`)
    const minimum = this.run('/usr/bin/plutil', ['-extract', 'MinimumOSVersion', 'raw', path.join(bundle, 'Info.plist')])
    assert.equal(minimum, this.laterFloors[language] ?? '17.4', `${language}: MinimumOSVersion`)
    const webAssembly = this.webAssembly(path.join(bundle, 'WebBundle'))
    const modules = Object.keys(webAssembly).length
    if (this.linked.includes(language)) assert.equal(modules, 0, `${language} is linked into the app, not WebAssembly`)
    else assert.ok(modules > 0, `${language}: the bundle has no WebAssembly`)
    const notes = output.split('\n').filter((line) => line.startsWith('Note:'))
    this.simctl('install', this.device, bundle)
    const launched = this.simctl('launch', this.device, applicationId)
    const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1])
    const documents = path.join(this.simctl('get_app_container', this.device, applicationId, 'data'), 'Documents')
    try {
      const root = await this.proof(documents, 'root', pid)
      const rootShot = this.screenshot(`${language}-root`)
      const item = await this.proof(documents, 'item', pid)
      const itemShot = this.screenshot(`${language}-item`)
      assert.equal(root.basket, '2 in the basket', `${language}: the page service answered the click`)
      assert.equal(root.summary, '2 items', `${language}: the page service answered a method with an argument`)
      assert.equal(root.scoped, this.scopedColor, `${language}: the component style applies inside the component`)
      assert.notEqual(root.outside, this.scopedColor, `${language}: the component style stays inside the component`)
      assert.equal(item.path, '/items/7/', `${language}: the dynamic route opened`)
      const result = { language, built, minimum, webAssembly, notes, pid, root, item, screenshots: [rootShot, itemShot] }
      console.log(`==> ${language}: built in ${built} s, MinimumOSVersion ${minimum}, ${modules ? `${modules} WebAssembly files, ${Math.round(Object.values(webAssembly).reduce((total, size) => total + size, 0) / 1024)} KB` : 'linked'}, ready ${root.loaded} ms after the page started, ${root.basket}, ${root.summary}, ${item.path}`)
      return result
    } finally {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'terminate', this.device, applicationId], { timeout: 30000, stdio: 'ignore' })
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'uninstall', this.device, applicationId], { timeout: 60000, stdio: 'ignore' })
    }
  }

  static async main() {
    mkdirSync(this.evidence, { recursive: true })
    const report = { languages: this.languages, results: [], failures: [] }
    try {
      Object.assign(report, this.bootSimulator())
      for (const language of this.languages) {
        try {
          report.results.push(await this.check(language))
        } catch (error) {
          console.log(`==> ${language}: FAIL ${String(error.message).slice(0, 2000)}`)
          report.failures.push({ language, error: String(error.message).slice(0, 4000) })
        }
        writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      }
    } finally {
      this.shutdownSimulator()
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    assert.deepEqual(report.failures.map(({ language }) => language), [], 'some page services failed; see the report')
    console.log(`PASS: page services (${this.languages.join(', ')}) answered a click and a method with an argument in the iOS ${report.runtime} simulator app (${report.device}), with a scoped component style and a dynamic route. Report: ${path.join(this.evidence, 'report.json')}`)
  }
}

await IosPageServicesTest.main()

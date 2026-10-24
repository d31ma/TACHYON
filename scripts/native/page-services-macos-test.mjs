#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawnSync } from 'node:child_process'
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

assert.equal(process.platform, 'darwin', 'The macOS page-service check needs macOS.')

class MacOsPageServicesTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'macos-page-services-'))
  static probe = path.join(MacOsPageServicesTest.root, 'page-checks-macos-probe')
  static features = path.join(MacOsPageServicesTest.repository, 'website/content/features')
  static serviceLanguages = (process.env.TAC_MACOS_SERVICE_LANGUAGES ?? 'py,dart,kt,java').split(',').map((item) => item.trim()).filter(Boolean)
  static garbageCollectedLanguages = ['dart', 'kt', 'java']
  static garbageCollectedFloor = '15.2'
  static defaultFloor = '13.0'
  static pyodideWasm = '.tachyon/services/python/314.0.7/pyodide.asm.wasm'
  static tallySteps = [
    ['wait', '0 in the basket', 'Add two'],
    ['press', 'Add two'],
    ['wait', '2 in the basket', '2 items'],
    ['quit'],
  ]

  static run(program, args) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout: 900_000, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')} failed:\n${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout
  }

  /** @param {string} project @param {string} id */
  static stage(project, id) {
    const source = path.join(this.features, 'web-assembly-services', 'client')
    for (const entry of ['pages', 'components']) cpSync(path.join(source, entry), path.join(project, 'client', entry), { recursive: true })
    const manifest = JSON.parse(readFileSync(path.join(source, 'manifest.json'), 'utf8'))
    manifest.tac = { ...manifest.tac, application: { ...manifest.tac?.application, id, version: '0.0.1' } }
    manifest.name ??= 'Tally'
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest, null, 2))
    return manifest.name.replace(/[^A-Za-z0-9]/g, '')
  }

  /** @param {string} project @param {string} language */
  static copyService(project, language) {
    const target = path.join(project, 'client/services', `TallyService.${language}`)
    mkdirSync(path.dirname(target), { recursive: true })
    cpSync(path.join(this.features, 'web-assembly-services', `client/services/TallyService.${language}`), target)
  }

  /** @param {string} app */
  static minimumSystem(app) {
    return this.run('/usr/libexec/PlistBuddy', ['-c', 'Print :LSMinimumSystemVersion', path.join(app, 'Contents/Info.plist')]).trim()
  }

  /** @param {string} id @param {Array<string[]>} steps */
  static async drive(id, steps) {
    for (const [action, ...values] of steps) {
      await promisify(execFile)(this.probe, [id, action, ...values], { timeout: 180_000 })
    }
  }

  /** @param {string} language */
  static async checkService(language) {
    const project = path.join(this.root, `tally-${language}`)
    const id = `dev.tachyon.tally-macos-${language}`
    const application = this.stage(project, id)
    this.copyService(project, language)
    const started = Date.now()
    this.run(this.binary, ['bundle', project, '--target', 'macos'])
    const built = ((Date.now() - started) / 1000).toFixed(0)
    const app = path.join(project, 'dist/macos', `${application}.app`)
    const floor = this.garbageCollectedLanguages.includes(language) ? this.garbageCollectedFloor : this.defaultFloor
    assert.equal(this.minimumSystem(app), floor, `${language}: LSMinimumSystemVersion`)
    if (language === 'py') assert.ok(this.run('/usr/bin/find', [app, '-path', `*/${this.pyodideWasm}`]).trim(), 'the app holds no Pyodide runtime')
    const opened = Date.now()
    this.run('/usr/bin/open', ['-n', app])
    try {
      await this.drive(id, this.tallySteps)
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(app, 'Contents/MacOS')])
    }
    const answered = ((Date.now() - opened) / 1000).toFixed(1)
    console.log(`ok - ${language} page service answered a click in the macOS app (built in ${built} s, LSMinimumSystemVersion ${floor}, opened and answered in ${answered} s)`)
  }

  static async main() {
    try {
      this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
        path.join(this.repository, 'scripts/native/page-checks-macos-probe.swift'), '-o', this.probe])
      for (const language of this.serviceLanguages) await this.checkService(language)
    } finally {
      rmSync(this.root, { recursive: true, force: true })
    }
    console.log(`PASS: WebAssembly page services (${this.serviceLanguages.join(', ')}) answered in the packaged macOS app.`)
  }
}

await MacOsPageServicesTest.main()

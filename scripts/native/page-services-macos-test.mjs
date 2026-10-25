#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawnSync } from 'node:child_process'
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { PageServicesFixture } from './page-services-fixture.mjs'

assert.equal(process.platform, 'darwin', 'The macOS page-service check needs macOS.')

class MacOsPageServicesTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'macos-page-services-'))
  static probe = path.join(MacOsPageServicesTest.root, 'page-checks-macos-probe')
  static bounds = path.join(MacOsPageServicesTest.root, 'window-grant-bounds')
  static panelKeys = path.join(MacOsPageServicesTest.repository, 'scripts/native/MacOsOpenPanelKeys.applescript')
  static features = path.join(MacOsPageServicesTest.repository, 'website/content/features')
  static serviceLanguages = PageServicesFixture.list('TAC_MACOS_SERVICE_LANGUAGES', 'py,dart,kt,java,go')
  static windowCallers = PageServicesFixture.list('TAC_MACOS_WINDOW_CALLERS', 'cs,kt')
  static pickerCallers = PageServicesFixture.list('TAC_MACOS_PICKER_CALLERS', 'rs')
  static windowRefusal = process.env.TAC_MACOS_WINDOW_REFUSAL !== '0'
  static resizedWidth = 1280
  static attempts = 60
  static pause = 250
  static garbageCollectedLanguages = ['dart', 'kt']
  static garbageCollectedFloor = '15.2'
  static exceptionReferenceLanguages = ['java']
  static exceptionReferenceFloor = '15.4'
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

  /** @param {string} id */
  static forgetWebsiteData(id) {
    const library = path.join(homedir(), 'Library')
    for (const folder of ['WebKit', 'Caches', 'HTTPStorages']) rmSync(path.join(library, folder, id), { recursive: true, force: true })
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
    const floor = this.exceptionReferenceLanguages.includes(language)
      ? this.exceptionReferenceFloor
      : this.garbageCollectedLanguages.includes(language) ? this.garbageCollectedFloor : this.defaultFloor
    assert.equal(this.minimumSystem(app), floor, `${language}: LSMinimumSystemVersion`)
    if (language === 'py') assert.ok(this.run('/usr/bin/find', [app, '-path', `*/${this.pyodideWasm}`]).trim(), 'the app holds no Pyodide runtime')
    const opened = Date.now()
    this.run('/usr/bin/open', ['-n', app])
    try {
      await this.drive(id, this.tallySteps)
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(app, 'Contents/MacOS')])
      this.forgetWebsiteData(id)
    }
    const answered = ((Date.now() - opened) / 1000).toFixed(1)
    console.log(`ok - ${language} page service answered a click in the macOS app (built in ${built} s, LSMinimumSystemVersion ${floor}, opened and answered in ${answered} s)`)
  }

  /** @param {string} project @param {string} application */
  static open(project, application) {
    const app = path.join(project, 'dist/macos', `${application}.app`)
    this.run(this.binary, ['bundle', project, '--target', 'macos'])
    this.run('/usr/bin/open', ['-n', app])
    return app
  }

  /** @param {string} app @param {string} id */
  static close(app, id) {
    spawnSync('/usr/bin/pkill', ['-f', path.join(app, 'Contents/MacOS')])
    this.forgetWebsiteData(id)
  }

  /** @param {string} owner */
  static async settledWidth(owner) {
    let width = 0
    for (let attempt = 0; attempt < this.attempts && width !== this.resizedWidth; attempt += 1) {
      await new Promise((done) => setTimeout(done, this.pause))
      width = Number(spawnSync(this.bounds, [owner], { encoding: 'utf8' }).stdout.trim())
    }
    return width
  }

  /** @param {string} language */
  static async checkWindowCaller(language) {
    const project = path.join(this.root, `window-${language}`)
    const application = PageServicesFixture.stageWindowCaller(project, language)
    const id = `dev.tachyon.window-${language}`
    const app = this.open(project, application)
    try {
      await this.drive(id, [['wait', 'Report', 'Rename'], ['press', 'Rename'], ['wait', 'Quarterly report'], ['press', 'Resize']])
      const width = await this.settledWidth(application)
      assert.equal(width, this.resizedWidth, `${language}: the resized window is ${width} wide`)
    } finally {
      this.close(app, id)
    }
    console.log(`ok - ${language} WindowControls caller renamed the macOS window to Quarterly report and resized it to ${this.resizedWidth} wide`)
  }

  static async checkWindowRefusal() {
    const project = path.join(this.root, 'window-refusal')
    const id = 'dev.tachyon.window-refusal'
    const application = PageServicesFixture.stageWindowRefusal(project, id)
    const app = this.open(project, application)
    try {
      await this.drive(id, [['wait', 'finished', 'Heard 1', PageServicesFixture.refusedTitles.oversized, 'nothing']])
    } finally {
      this.close(app, id)
    }
    console.log('ok - a JavaScript WindowControls caller heard BrowserCallFailed windowControls setTitle "invalid or oversized host call" after the macOS host refused an oversized title, and nothing for the accepted titles')
  }

  /** @param {string} app @param {string} id @param {string} chosen */
  static async answerPanel(app, id, chosen) {
    const pid = this.run('/usr/bin/pgrep', ['-n', '-f', path.join(app, 'Contents/MacOS')]).trim()
    const pressed = promisify(execFile)(this.probe, [id, 'press', 'Choose file'], { timeout: 180_000 }).catch((error) => error)
    this.run('/usr/bin/osascript', [this.panelKeys, pid, chosen])
    await pressed
  }

  /** @param {string} language */
  static async checkPickerCaller(language) {
    const project = path.join(this.root, `picker-${language}`)
    const { application, notes } = PageServicesFixture.stagePicker(project, language)
    const id = `dev.tachyon.picker-${language}`
    const [read, cancelled] = PageServicesFixture.pickerStatuses[language]
    const app = this.open(project, application)
    try {
      await this.drive(id, [['wait', 'Choose a text file', 'Ready']])
      await this.answerPanel(app, id, notes)
      await this.drive(id, [['wait', read]])
      await this.answerPanel(app, id, '')
      await this.drive(id, [['wait', cancelled]])
    } finally {
      this.close(app, id)
    }
    console.log(`ok - ${language} FilePicker caller chose and read ${path.basename(notes)} in NSOpenPanel and answered ${cancelled} for an Escaped panel`)
  }

  static async main() {
    try {
      this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
        path.join(this.repository, 'scripts/native/page-checks-macos-probe.swift'), '-o', this.probe])
      this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'CoreGraphics',
        path.join(this.repository, 'scripts/window-grant-macos-bounds.swift'), '-o', this.bounds])
      for (const language of this.serviceLanguages) await this.checkService(language)
      for (const language of this.windowCallers) await this.checkWindowCaller(language)
      for (const language of this.pickerCallers) await this.checkPickerCaller(language)
      if (this.windowRefusal) await this.checkWindowRefusal()
    } finally {
      rmSync(this.root, { recursive: true, force: true })
    }
    console.log(`PASS: page services (${this.serviceLanguages.join(', ')}) answered in the packaged macOS app, WindowControls callers (${this.windowCallers.join(', ')}) renamed and resized its window, FilePicker callers (${this.pickerCallers.join(', ')}) chose, read and cancelled through NSOpenPanel${this.windowRefusal ? ', and a host refusal reached a JavaScript WindowControls caller as BrowserCallFailed' : ''}.`)
  }
}

await MacOsPageServicesTest.main()

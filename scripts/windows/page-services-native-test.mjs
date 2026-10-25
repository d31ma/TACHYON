#!/usr/bin/env node
import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { PageServicesFixture } from '../native/page-services-fixture.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsPageServicesTest {
  static root = WindowsGate.fixture('windows-page-services-')
  static serviceLanguages = PageServicesFixture.list('TAC_WINDOWS_SERVICE_LANGUAGES', 'go,cs,fs,swift,kt,java,dart,rb,lua,hpp,py')
  static windowCallers = PageServicesFixture.list('TAC_WINDOWS_WINDOW_CALLERS', 'swift,cs,kt,rs')
  static pickerCallers = PageServicesFixture.list('TAC_WINDOWS_PICKER_CALLERS', process.env.TAC_SWIFT_WASM_SDK ? 'js,kt,cs,rs,swift' : 'js,kt,cs,rs')
  static storedField = process.env.TAC_WINDOWS_STORED_FIELD !== 'false'
  static nativeLanguages = ['rs', 'cs']
  static pageLanguages = ['js', 'ts']
  static callerFolders = { cs: 'native' }

  /** @param {string} language */
  static kind(language) {
    if (this.pageLanguages.includes(language)) return 'page'
    return this.nativeLanguages.includes(language) ? 'natively linked' : 'WebAssembly'
  }

  /** @param {string} project @param {string} application @param {Array<[string, unknown]>} steps */
  static async drive(project, application, steps) {
    const executable = WindowsGate.executable(project, application)
    const app = WindowsGate.launch(executable)
    try {
      const output = await WindowsGate.probeSteps(app.pid, steps, path.join(project, 'handshake'))
      const memory = await WindowsGate.memory(app)
      return [...output.split(/\r?\n/).filter(line => /^(OK|LOADED|NOTE|PASS)/.test(line)), memory].join('\n')
    } finally {
      await app.stop()
    }
  }

  /** @param {string} ty @param {string} language */
  static async checkService(ty, language) {
    const project = path.join(this.root, `tally-${language}`)
    const application = PageServicesFixture.stageTally(project, language)
    const started = Date.now()
    await WindowsGate.build(ty, project, 900_000)
    const built = ((Date.now() - started) / 1000).toFixed(0)
    const output = await this.drive(project, application, PageServicesFixture.tallySteps)
    console.log(`==> ${language} ${this.kind(language)} page service (built in ${built} s)\n${output}`)
  }

  /** @param {string} ty */
  static async checkStoredField(ty) {
    const project = path.join(this.root, 'stored-field')
    const application = PageServicesFixture.stageStoredField(project)
    await WindowsGate.build(ty, project, 900_000)
    const companion = path.join(path.dirname(WindowsGate.executable(project, application)), 'TachyonRustCompanion.dll')
    assert.ok(existsSync(companion), `the Rust service is not linked into the Windows app: no ${companion}`)
    const output = await this.drive(project, application, PageServicesFixture.storedFieldSteps)
    console.log(`==> a linked Rust service's @LocalState field, set by its own method, after a page reload\n${output}`)
  }

  /** @param {string} ty @param {string} language */
  static async checkWindowCaller(ty, language) {
    const project = path.join(this.root, `window-${language}`)
    const application = PageServicesFixture.stageWindowCaller(project, language, this.callerFolders[language] ?? 'client')
    await WindowsGate.build(ty, project, 900_000)
    const output = await this.drive(project, application, PageServicesFixture.reportSteps)
    console.log(`==> ${language} ${this.kind(language)} WindowControls caller\n${output}`)
  }

  /** @param {string} ty @param {string} language */
  static async checkPickerCaller(ty, language) {
    const project = path.join(this.root, `picker-${language}`)
    const { application, notes } = PageServicesFixture.stagePicker(project, language)
    await WindowsGate.build(ty, project, 900_000)
    const [read, cancelled] = PageServicesFixture.pickerStatuses[language]
    const steps = [
      ['wait', ['Choose a text file', 'Ready']],
      ['pick', { control: 'Choose file', file: notes }],
      ['wait', [read]],
      ['pick', { control: 'Choose file', cancel: true }],
      ['wait', [cancelled]],
    ]
    const output = await this.drive(project, application, steps)
    console.log(`==> ${language} ${this.kind(language)} FilePicker caller\n${output}`)
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    for (const language of this.serviceLanguages) await this.checkService(ty, language)
    for (const language of this.windowCallers) await this.checkWindowCaller(ty, language)
    for (const language of this.pickerCallers) await this.checkPickerCaller(ty, language)
    if (this.storedField) await this.checkStoredField(ty)
    console.log(`PASS: page services (${this.serviceLanguages.join(', ')}) answered in the built Windows app (WebView2; C# is linked natively, the others run as WebAssembly), WindowControls callers (${this.windowCallers.join(', ')}) renamed and resized its window, FilePicker callers (${this.pickerCallers.join(', ')}) chose, read and cancelled through the file dialog, and ${this.storedField ? 'a linked Rust service\'s @LocalState field survived a page reload' : 'the stored field check was skipped'}.`)
  }
}

await WindowsPageServicesTest.main()

#!/usr/bin/env node
import path from 'node:path'
import { PageServicesFixture } from '../native/page-services-fixture.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsPageServicesTest {
  static root = WindowsGate.fixture('windows-page-services-')
  static serviceLanguages = PageServicesFixture.list('TAC_WINDOWS_SERVICE_LANGUAGES', 'go,cs,fs,swift,kt,java,dart,rb,lua,hpp,py')
  static windowCallers = PageServicesFixture.list('TAC_WINDOWS_WINDOW_CALLERS', 'swift,cs,kt')
  static nativeLanguages = ['rs', 'cs']
  static callerFolders = { cs: 'native' }

  /** @param {string} language */
  static kind(language) {
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

  /** @param {string} ty @param {string} language */
  static async checkWindowCaller(ty, language) {
    const project = path.join(this.root, `window-${language}`)
    const application = PageServicesFixture.stageWindowCaller(project, language, this.callerFolders[language] ?? 'client')
    await WindowsGate.build(ty, project, 900_000)
    const output = await this.drive(project, application, PageServicesFixture.reportSteps)
    console.log(`==> ${language} ${this.kind(language)} WindowControls caller\n${output}`)
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    for (const language of this.serviceLanguages) await this.checkService(ty, language)
    for (const language of this.windowCallers) await this.checkWindowCaller(ty, language)
    console.log(`PASS: page services (${this.serviceLanguages.join(', ')}) answered in the built Windows app (WebView2; C# is linked natively, the others run as WebAssembly), and WindowControls callers (${this.windowCallers.join(', ')}) renamed and resized its window.`)
  }
}

await WindowsPageServicesTest.main()

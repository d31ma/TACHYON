#!/usr/bin/env node
import path from 'node:path'
import { MobileAccessibilityFixture } from '../native/mobile-accessibility-fixture.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsSystemBackTest {
  static fixture = WindowsGate.fixture('windows-system-back-')
  static handshake = path.join(WindowsSystemBackTest.fixture, 'handshake')
  static applicationId = 'dev.tachyon.system-back'

  static steps() {
    const { HEADING, LINK, ABOUT_HEADING } = MobileAccessibilityFixture
    return [
      ['wait', HEADING], ['click', LINK], ['wait', ABOUT_HEADING],
      ['historyKeys', 'back'], ['wait', HEADING], ['gone', ABOUT_HEADING],
      ['historyKeys', 'forward'], ['wait', ABOUT_HEADING],
      ['historyButton', 'back'], ['wait', HEADING],
      ['historyButton', 'forward'], ['wait', ABOUT_HEADING],
      ['quit', 'Exit'],
    ]
  }

  static async main() {
    WindowsGate.requireDesktop()
    const started = Date.now()
    try {
      const ty = await WindowsGate.ty()
      const project = path.join(this.fixture, 'app')
      MobileAccessibilityFixture.write(project, this.applicationId)
      await WindowsGate.build(ty, project)
      const executable = WindowsGate.executable(project, MobileAccessibilityFixture.NAME)
      const app = WindowsGate.launch(executable)
      try {
        await WindowsGate.probeSteps(app.pid, this.steps(), this.handshake)
        await WindowsGate.waitForQuit(app, executable)
      } finally {
        await app.stop()
      }
    } finally {
      WindowsGate.remove(this.fixture)
    }
    const seconds = Math.round((Date.now() - started) / 1000)
    console.log(`PASS: in the built Windows app (WebView2), Alt+Left and the mouse back button went back to the previous route, and Alt+Right and the mouse forward button went forward again, with no host binding (${seconds} s).`)
  }
}

await WindowsSystemBackTest.main()

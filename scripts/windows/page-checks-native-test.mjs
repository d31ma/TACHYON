#!/usr/bin/env node
import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { PageChecksFixture } from '../native/page-checks-fixture.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsPageChecksTest {
  static fixture = WindowsGate.fixture('windows-page-checks-')
  static handshake = path.join(WindowsPageChecksTest.fixture, 'handshake')
  static application = PageChecksFixture.application
  static stopRequest = PageChecksFixture.stopRequest
  static roleChecks = {
    'Saved theme none': [['Edit', 'Typing box'], ['Button', 'Reach the undeclared host'], ['Hyperlink', 'Open the other page'], ['Text', 'Saved theme none']],
    'The other page': [['Heading', 'The other page'], ['Hyperlink', 'Back to the first page']],
  }

  static firstLaunchSteps() {
    return PageChecksFixture.firstLaunchSteps('Exit').flatMap(step => {
      const anchor = step[0] === 'wait' ? Object.keys(this.roleChecks).find(text => step[1].includes(text)) : undefined
      return anchor ? [step, ['roles', this.roleChecks[anchor]]] : [step]
    })
  }

  static checkBuild(yonOrigin, undeclaredOrigin) {
    PageChecksFixture.checkBuild(WindowsGate.hostSource(this.fixture), WindowsGate.bundle(this.fixture, this.application), yonOrigin, undeclaredOrigin)
    const executable = WindowsGate.executable(this.fixture, this.application)
    assert.ok(!existsSync(WindowsGate.webViewData(executable)), 'the fresh build already has WebView2 data')
    return executable
  }

  static async firstLaunch(executable, yon, undeclared) {
    const app = WindowsGate.launch(executable)
    try {
      const probe = WindowsGate.startProbe(app.pid, this.firstLaunchSteps(), this.handshake)
      await WindowsGate.answerAll(probe, this.handshake, {
        [this.stopRequest]: async () => {
          assert.deepEqual(undeclared.requests, [], `The undeclared host saw requests: ${undeclared.requests.join(', ')}`)
          console.log(`==> ${undeclared.origin} saw no request; stopping Yon`)
          await yon.stop()
        },
      })
      await WindowsGate.waitForQuit(app, executable)
    } finally {
      await app.stop()
    }
    assert.ok(existsSync(WindowsGate.webViewData(executable)), `WebView2 kept no data in ${WindowsGate.webViewData(executable)}`)
    console.log(`==> the app quit; WebView2 data is in ${WindowsGate.webViewData(executable)}; starting it again`)
  }

  static async secondLaunch(executable) {
    const app = WindowsGate.launch(executable)
    try {
      await WindowsGate.probeSteps(app.pid, PageChecksFixture.secondLaunchSteps, this.handshake)
    } finally {
      await app.stop()
    }
  }

  static async main() {
    WindowsGate.requireDesktop()
    const started = Date.now()
    const ty = await WindowsGate.ty()
    const yonPort = await PageChecksFixture.freePort()
    const yonOrigin = `http://127.0.0.1:${yonPort}`
    const undeclared = await PageChecksFixture.undeclaredHost()
    let yon = null
    try {
      PageChecksFixture.writeProject(this.fixture, { yonOrigin, undeclaredOrigin: undeclared.origin, pageOrigin: WindowsGate.pageOrigin })
      await WindowsGate.build(ty, this.fixture)
      const executable = this.checkBuild(yonOrigin, undeclared.origin)
      const served = await WindowsGate.serve(ty, this.fixture, yonPort, '/api/notes', WindowsGate.pageOrigin)
      yon = served.yon
      await PageChecksFixture.checkYonAnswers(served.request, WindowsGate.pageOrigin)
      await this.firstLaunch(executable, yon, undeclared)
      await this.secondLaunch(executable)
    } finally {
      await yon?.stop()
      await new Promise((resolve) => undeclared.server.close(resolve))
    }
    const seconds = Math.round((Date.now() - started) / 1000)
    console.log(`PASS: in the built Windows app (WebView2), typing kept focus and caret through reactive writes; connect-src refused ${undeclared.origin} (it saw no request) while the Yon origin answered; a network-first Yon read answered from the cache with Yon stopped; a @SessionState field survived a navigation; a preference string survived a quit and a relaunch; UI Automation exposed Edit, Button, Hyperlink, Text and Heading roles (${seconds} s).`)
  }
}

await WindowsPageChecksTest.main()

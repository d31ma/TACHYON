#!/usr/bin/env node
import assert from 'node:assert/strict'
import path from 'node:path'
import { PeersNativeProbe } from '../native/peers-native-probe.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsPeersTest {
  static fixture = WindowsGate.fixture('windows-peers-')
  static handshake = path.join(WindowsPeersTest.fixture, 'handshake')
  static application = 'PeersGate'
  static read = ['read', { prefix: PeersNativeProbe.PREFIX, not: [PeersNativeProbe.CHECKING] }]

  static files() {
    return PeersNativeProbe.files({ id: 'dev.tachyon.peers-gate', name: 'Peers Gate', iceServers: [], relayOnly: false })
  }

  /** @param {string} output */
  static result(output) {
    const line = output.split(/\r?\n/).find(text => text.startsWith('READ: '))
    assert.ok(line, 'the probe read no peers result')
    return line.slice('READ: '.length).trim()
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    for (const [name, text] of Object.entries(this.files())) WindowsGate.write(this.fixture, name, text)
    await WindowsGate.build(ty, this.fixture)
    const executable = WindowsGate.executable(this.fixture, this.application)
    const app = WindowsGate.launch(executable)
    try {
      const output = await WindowsGate.probeSteps(app.pid, [this.read], this.handshake)
      const result = this.result(output)
      assert.equal(result, PeersNativeProbe.PASSED, `two Peers connections in the WebView2 page must connect and carry ${PeersNativeProbe.TOTAL} bytes, got ${result}`)
      console.log(`PASS: in the WebView2 app (https://tachyon.local), Peers opened two connections, carried the offer, answer and candidates between them, opened an ordered channel and delivered ${PeersNativeProbe.TOTAL} bytes (launch ${app.pid})`)
    } finally {
      await app.stop()
    }
  }
}

await WindowsPeersTest.main()

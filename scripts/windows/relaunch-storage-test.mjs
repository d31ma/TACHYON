#!/usr/bin/env node
import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import path from 'node:path'
import { RelaunchStorageProbe } from '../native/relaunch-storage-probe.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsRelaunchStorageTest {
  static fixture = WindowsGate.fixture('windows-relaunch-storage-')
  static handshake = path.join(WindowsRelaunchStorageTest.fixture, 'handshake')
  static application = 'StorageGate'
  static prefix = 'Relaunch storage: '
  static read = ['read', { prefix: WindowsRelaunchStorageTest.prefix, not: [RelaunchStorageProbe.CHECKING] }]

  static files() {
    return {
      'client/manifest.json': JSON.stringify({
        name: 'Storage Gate',
        start_url: '/',
        tac: { application: { id: 'dev.tachyon.storage-gate', version: '0.0.1' } },
      }),
      'client/pages/tac.html': '<storage-check hydrate="load" />\n',
      'client/shared/relaunch-storage.js': RelaunchStorageProbe.pageModule(),
      'client/services/StorageCheckService.js': `import { RelaunchStorage } from '/shared/relaunch-storage.js';
@Service()
export default class StorageCheckService {
  static _PREFIX = ${JSON.stringify(this.prefix)};
  @Needs
  taskScheduler;
  report = StorageCheckService._PREFIX + '${RelaunchStorageProbe.CHECKING}';
  @OnStart()
  initialize() {
    void this._check();
  }
  async _check() {
    this.report = StorageCheckService._PREFIX + await RelaunchStorage.check();
    await this.taskScheduler.render();
  }
}
`,
      'client/components/storage/check/tac.html': '<main aria-label="Storage"><p>{storageCheck.report}</p></main>\n',
    }
  }

  /** @param {string} output */
  static result(output) {
    const line = output.split(/\r?\n/).find(text => text.startsWith('READ: '))
    assert.ok(line, 'the probe read no storage result')
    return line.slice('READ: '.length).trim()
  }

  /** @param {string} executable @param {Array<[string, unknown]>} steps */
  static async launch(executable, steps) {
    const app = WindowsGate.launch(executable)
    try {
      const output = await WindowsGate.probeSteps(app.pid, steps, this.handshake)
      return { pid: app.pid, result: this.result(output), app }
    } catch (error) {
      await app.stop()
      throw error
    }
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    for (const [name, text] of Object.entries(this.files())) WindowsGate.write(this.fixture, name, text)
    await WindowsGate.build(ty, this.fixture)
    const executable = WindowsGate.executable(this.fixture, this.application)
    const data = WindowsGate.webViewData(executable)
    assert.ok(!existsSync(data), `the fresh build already has WebView2 data in ${data}`)
    const first = await this.launch(executable, [this.read, ['quit', 'Exit']])
    try {
      assert.equal(first.result, RelaunchStorageProbe.WRITTEN, `the first launch must write web storage, got ${first.result}`)
      await WindowsGate.waitForQuit(first.app, executable)
    } finally {
      await first.app.stop()
    }
    assert.ok(existsSync(data), `WebView2 kept no data in ${data}`)
    console.log(`==> launch ${first.pid} wrote ${first.result}; WebView2 data is in ${data}; starting the app again`)
    const second = await this.launch(executable, [this.read])
    try {
      assert.notEqual(second.pid, first.pid, 'the relaunch must be a new process')
      assert.equal(second.result, RelaunchStorageProbe.KEPT, `web storage was not kept after File > Exit, got ${second.result}`)
    } finally {
      await second.app.stop()
    }
    console.log(`PASS: the unpackaged Windows app kept localStorage, IndexedDB and the Cache API in ${path.basename(data)} across File > Exit and a relaunch (launch ${first.pid} ${first.result}, relaunch ${second.pid} read ${second.result})`)
  }
}

await WindowsRelaunchStorageTest.main()

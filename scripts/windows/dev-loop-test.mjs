#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawn } from 'node:child_process'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { DevLoopFixture } from '../native/dev-loop-fixture.mjs'
import { WindowsCommand } from './windows-command.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsDevLoopTest {
  static fixture = WindowsGate.fixture('windows-dev-loop-')
  static handshake = path.join(WindowsDevLoopTest.fixture, 'handshake')
  static log = process.env.TAC_DEV_LOOP_LOG ?? path.join(WindowsDevLoopTest.fixture, 'preview.log')
  static launcher = fileURLToPath(new URL('./process-group.py', import.meta.url))
  static python = process.env.TAC_PYTHON ?? 'python'
  static opened = 'Opened the windows app'
  static clickAttempts = 6
  static clickWaitSeconds = 5
  static openMilliseconds = 900_000
  static stopMilliseconds = 10_000
  static pollMilliseconds = 250
  static output = ''
  /** @type {import('node:child_process').ChildProcess | null} */
  static preview = null
  static previewStatus = null

  static executable() {
    const application = DevLoopFixture.application
    return path.join(this.fixture, 'dist', 'preview', 'windows', application, 'bin', `${application}.exe`)
  }

  /** @param {import('./windows-command.mjs').WindowsProcess} entry */
  static isApp(entry) {
    const executable = this.executable().toLowerCase()
    return entry.name.toLowerCase() === `${DevLoopFixture.application.toLowerCase()}.exe` && entry.commandLine.toLowerCase().includes(executable)
  }

  static async appProcesses() {
    return (await WindowsCommand.processes()).filter(entry => this.isApp(entry))
  }

  static record(chunk) {
    this.output += chunk.toString()
    writeFileSync(this.log, this.output)
  }

  /** @param {number} port */
  static startPreview(ty, port) {
    const args = [this.launcher, ty, 'preview', this.fixture, '--target', 'windows', '--port', String(port)]
    const preview = spawn(this.python, args, { cwd: WindowsGate.repository, env: process.env, stdio: ['pipe', 'pipe', 'pipe'] })
    preview.stdout.on('data', chunk => this.record(chunk))
    preview.stderr.on('data', chunk => this.record(chunk))
    preview.once('error', error => { this.record(`\nthe preview launcher failed: ${error.message}\n`) })
    preview.once('exit', status => { this.previewStatus = status })
    this.preview = preview
  }

  static previewRunning() {
    return this.preview !== null && this.previewStatus === null && this.preview.exitCode === null
  }

  /** @param {Array<[string, unknown]>} steps @param {number} waitSeconds */
  static async probe(steps, waitSeconds = 30) {
    const file = path.join(this.handshake, `steps-${Date.now()}.json`)
    writeFileSync(file, JSON.stringify(steps))
    const args = ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', WindowsGate.probe, '-Executable', this.executable(),
      '-StepsFile', file, '-WaitSeconds', String(waitSeconds)]
    const result = await WindowsCommand.run('powershell', args, {
      cwd: WindowsGate.repository, env: { ...process.env, TAC_PROBE_HANDSHAKE: this.handshake }, timeoutMilliseconds: 300_000,
    })
    return { passed: result.status === 0 && !result.error, output: `${result.stdout}${result.stderr}` }
  }

  /** @param {Array<[string, unknown]>} steps */
  static async probeOrFail(steps) {
    const { passed, output } = await this.probe(steps)
    assert.ok(passed, `the UI Automation probe failed on ${JSON.stringify(steps)}\n${output}\npreview log:\n${this.output}`)
  }

  /** @param {[string, string]} value */
  static async clickUntil([name, text]) {
    let output = ''
    for (let attempt = 0; attempt < this.clickAttempts; attempt += 1) {
      const result = await this.probe([['click', name], ['wait', [text]]], this.clickWaitSeconds)
      if (result.passed) return
      output = result.output
    }
    assert.fail(`pressing ${name} never showed ${text}\n${output}\npreview log:\n${this.output}`)
  }

  /** @param {[string, string, string]} value */
  static edit([relative, old, replacement]) {
    const file = path.join(this.fixture, relative)
    const source = readFileSync(file, 'utf8')
    assert.ok(source.includes(old), `${relative} does not hold ${old}`)
    writeFileSync(file, source.replace(old, replacement))
    console.log(`EDIT: ${relative}: ${old} -> ${replacement}`)
  }

  static launches() {
    return this.output.split(this.opened).length - 1
  }

  /** @param {number} count */
  static async openedTimes(count) {
    const deadline = Date.now() + this.openMilliseconds
    while (this.launches() < count) {
      assert.ok(this.previewRunning(), `ty preview stopped early (${this.previewStatus})\n${this.output}`)
      assert.ok(Date.now() < deadline, `ty preview did not open the app ${count} time(s)\n${this.output}`)
      await sleep(this.pollMilliseconds)
    }
    const running = await this.appProcesses()
    assert.equal(running.length, 1, `one ${DevLoopFixture.application}.exe runs after opening ${count} time(s), not ${running.length}: ${running.map(entry => entry.pid).join(', ')}`)
    console.log(`OK: ty preview opened the app ${count} time(s) (process ${running[0].pid})`)
  }

  static async drive() {
    const actions = {
      wait: value => this.probeOrFail([['wait', value]]),
      click: value => this.probeOrFail([['click', value]]),
      clickUntil: value => this.clickUntil(value),
      edit: value => this.edit(value),
      opened: value => this.openedTimes(value),
    }
    await this.openedTimes(1)
    for (const [kind, value] of DevLoopFixture.steps) {
      await actions[kind](value)
      console.log(`ok - ${kind} ${JSON.stringify(value)}`)
    }
  }

  static async stopPreview() {
    assert.equal((await this.appProcesses()).length, 1, 'the app runs before ty preview stops')
    this.preview.stdin.write('break\n')
    await WindowsCommand.waitUntilGone(entry => this.isApp(entry), this.stopMilliseconds, `${DevLoopFixture.application}.exe after ty preview stopped`)
    console.log('OK: stopping ty preview (CTRL_BREAK) closed the app')
    const deadline = Date.now() + this.stopMilliseconds
    while (this.previewRunning() && Date.now() < deadline) await sleep(this.pollMilliseconds)
    assert.ok(!this.previewRunning(), `ty preview still runs ${this.stopMilliseconds / 1000} s after CTRL_BREAK\n${this.output}`)
    console.log(`OK: ty preview exited with code ${this.previewStatus}`)
    await WindowsGate.waitForWebViewExit(this.executable())
  }

  static async forceStop() {
    if (this.previewRunning()) {
      await new Promise(resolve => execFile('taskkill', ['/PID', String(this.preview.pid), '/T', '/F'], { windowsHide: true }, () => resolve(undefined)))
    }
    for (const entry of await this.appProcesses().catch(() => [])) {
      await new Promise(resolve => execFile('taskkill', ['/PID', String(entry.pid), '/T', '/F'], { windowsHide: true }, () => resolve(undefined)))
    }
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    DevLoopFixture.write(this.fixture)
    WindowsGate.write(this.handshake, '.keep', '')
    const port = await DevLoopFixture.freePort()
    this.startPreview(ty, port)
    try {
      await this.drive()
      assert.ok(existsSync(this.executable()), `ty preview left no ${this.executable()}`)
      await this.stopPreview()
    } finally {
      await this.forceStop()
      console.log(`preview log: ${this.log}`)
    }
    if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') WindowsGate.remove(this.fixture)
    console.log(`PASS: ty preview --target windows opened the app (WebView2) against http://127.0.0.1:${port}; its Yon calls reached the development server, a style edit arrived without a reload, a page edit reached the app, a manifest edit reopened it, and stopping ty preview closed it.`)
  }
}

await WindowsDevLoopTest.main()

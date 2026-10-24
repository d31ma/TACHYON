#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { DevLoopFixture } from '../native/dev-loop-fixture.mjs'

class IosDevLoopTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-dev-loop-'))
  static reader = path.join(IosDevLoopTest.fixture, 'screenshot-text')
  static tapper = path.join(IosDevLoopTest.fixture, 'simulator-tap')
  static log = process.env.TAC_DEV_LOOP_LOG ?? path.join(IosDevLoopTest.fixture, 'preview.log')
  static deviceSet = path.join(homedir(), 'Library/Developer/CoreSimulator/Devices')
  static executable = `${DevLoopFixture.application}.app/${DevLoopFixture.application}`
  static opened = 'Opened the ios app'
  static waitMilliseconds = 60_000
  static retryMilliseconds = 6_000
  static openSeconds = 900
  static pollMilliseconds = 500
  static device = ''
  static created = false
  static output = ''
  static preview = null

  static sleep(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds))
  }

  static run(program, args, timeout = 120_000) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')} failed\n${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', ...args], 600_000)
  }

  static compileTools() {
    this.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(this.repository, 'scripts/ios/screenshot-text.swift'), '-o', this.reader], 600_000)
    this.run('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/ios/simulator-tap.m'), '-o', this.tapper], 600_000)
  }

  static bootSimulator() {
    this.device = process.env.TAC_IOS_SIMULATOR ?? ''
    if (this.device === '') {
      const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
      const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
        .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
      assert.ok(runtime, 'an installed iOS simulator runtime is required')
      const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
      assert.ok(phone, 'the iOS runtime needs an iPhone device type')
      this.device = this.simctl('create', `DevLoop-${path.basename(this.fixture)}`, phone.identifier, runtime.identifier)
      this.created = true
      console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    }
    spawnSync('/usr/bin/xcrun', ['simctl', 'boot', this.device], { stdio: 'ignore', timeout: 600_000 })
    this.simctl('bootstatus', this.device, '-b')
  }

  static shutdownSimulator() {
    if (!this.created) return
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', ...args], { timeout: 60_000, stdio: 'ignore' })
    }
  }

  static screenLines() {
    const file = path.join(this.fixture, 'screen.png')
    this.simctl('io', this.device, 'screenshot', file)
    return JSON.parse(this.run(this.reader, [file]))
  }

  static async find(texts, milliseconds) {
    const deadline = Date.now() + milliseconds
    let lines = []
    while (Date.now() < deadline) {
      lines = this.screenLines()
      const found = texts.map((text) => lines.find((line) => line.text.includes(text)))
      if (found.every(Boolean)) return { found, lines }
      await this.sleep(this.pollMilliseconds)
    }
    return { found: null, lines }
  }

  static async wait(texts) {
    const { found, lines } = await this.find(texts, this.waitMilliseconds)
    assert.ok(found, `the app never showed ${texts.join(' | ')}; it showed ${lines.map((line) => line.text).join(' | ')}\n${this.output}`)
  }

  static async click(name) {
    const { found, lines } = await this.find([name], this.waitMilliseconds)
    assert.ok(found, `the app shows no ${name}; it showed ${lines.map((line) => line.text).join(' | ')}`)
    this.run(this.tapper, [this.deviceSet, this.device, String(found[0].x), String(found[0].y)])
  }

  static async clickUntil([name, text]) {
    const deadline = Date.now() + this.waitMilliseconds
    while (Date.now() < deadline) {
      await this.click(name)
      if ((await this.find([text], this.retryMilliseconds)).found) return
    }
    assert.fail(`tapping ${name} never showed ${text}\n${this.output}`)
  }

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

  static async openedTimes(count) {
    const deadline = Date.now() + this.openSeconds * 1000
    while (this.launches() < count) {
      assert.ok(this.preview.exited !== true && this.preview.error === undefined, `ty preview stopped early: ${this.preview.error?.message ?? this.preview.status}\n${this.output}`)
      assert.ok(Date.now() < deadline, `ty preview did not open the app ${count} time(s)\n${this.output}`)
      await this.sleep(this.pollMilliseconds)
    }
    console.log(`OK: ty preview opened the app ${count} time(s)`)
  }

  static running() {
    return spawnSync('/usr/bin/pgrep', ['-f', this.executable]).status === 0
  }

  static async drive() {
    const actions = {
      wait: (value) => this.wait(value),
      click: (value) => this.click(value),
      clickUntil: (value) => this.clickUntil(value),
      edit: (value) => this.edit(value),
      opened: (value) => this.openedTimes(value),
    }
    await this.openedTimes(1)
    for (const [kind, value] of DevLoopFixture.steps) {
      await actions[kind](value)
      console.log(`ok - ${kind} ${JSON.stringify(value)}`)
    }
  }

  static async stopPreview() {
    assert.ok(this.running(), `no process runs ${this.executable} before ty preview stops`)
    process.kill(this.preview.child.pid, 'SIGINT')
    const deadline = Date.now() + 20_000
    while (this.running() && Date.now() < deadline) await this.sleep(this.pollMilliseconds)
    assert.ok(!this.running(), `the app ${this.executable} still runs after ty preview stopped\n${this.output}`)
    console.log('OK: stopping ty preview closed the app')
  }

  static async main() {
    DevLoopFixture.write(this.fixture)
    this.compileTools()
    this.bootSimulator()
    spawnSync('/usr/bin/xcrun', ['simctl', 'uninstall', this.device, DevLoopFixture.identifier], { stdio: 'ignore', timeout: 120_000 })
    const port = await DevLoopFixture.freePort()
    const record = (chunk) => {
      this.output += chunk.toString()
      writeFileSync(this.log, this.output)
    }
    this.preview = await SupervisedCommand.start(this.binary, ['preview', this.fixture, '--target', 'ios', '--port', String(port)], {
      cwd: this.repository, env: { ...process.env, TAC_IOS_SIMULATOR: this.device }, timeoutMilliseconds: 1_800_000, onStdout: record, onStderr: record,
    })
    try {
      await this.drive()
      await this.stopPreview()
      await this.preview.completion
    } finally {
      if (this.preview.exited !== true) await this.preview.stop()
      spawnSync('/usr/bin/xcrun', ['simctl', 'uninstall', this.device, DevLoopFixture.identifier], { stdio: 'ignore', timeout: 120_000 })
      this.shutdownSimulator()
      console.log(`preview log: ${this.log}`)
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1' && existsSync(this.fixture)) rmSync(this.fixture, { recursive: true, force: true })
    }
    console.log(`PASS: ty preview --target ios installed the app on simulator ${this.device} against http://127.0.0.1:${port}; its Yon calls reached the development server, a style edit arrived without a reload, a page edit reached the app, a manifest edit reinstalled it, and stopping ty preview closed it.`)
  }
}

await IosDevLoopTest.main()

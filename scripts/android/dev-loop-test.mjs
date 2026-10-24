#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { DevLoopFixture } from '../native/dev-loop-fixture.mjs'

class AndroidDevLoopTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'android-dev-loop-'))
  static reader = path.join(AndroidDevLoopTest.fixture, 'screenshot-text')
  static log = process.env.TAC_DEV_LOOP_LOG ?? path.join(AndroidDevLoopTest.fixture, 'preview.log')
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? ''
  static adb = AndroidDevLoopTest.sdk === '' ? 'adb' : path.join(AndroidDevLoopTest.sdk, 'platform-tools/adb')
  static packageName = DevLoopFixture.identifier.replaceAll('-', '_')
  static opened = 'Opened the android app'
  static waitMilliseconds = 60_000
  static retryMilliseconds = 6_000
  static openSeconds = 900
  static pollMilliseconds = 500
  static screen = { width: 0, height: 0 }
  static output = ''
  static preview = null

  static sleep(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds))
  }

  static run(program, args, options = {}) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout: 120_000, maxBuffer: 32 * 1024 * 1024, ...options })
    assert.equal(result.status, 0, `${program} ${args.join(' ')} failed\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static adbShell(...args) {
    return this.run(this.adb, ['shell', ...args]).trim()
  }

  static compileReader() {
    this.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(this.repository, 'scripts/ios/screenshot-text.swift'), '-o', this.reader], { timeout: 600_000 })
  }

  static measureScreen() {
    const size = this.adbShell('wm', 'size').match(/(\d+)x(\d+)\s*$/)
    assert.ok(size, 'adb shell wm size printed no size')
    this.screen = { width: Number(size[1]), height: Number(size[2]) }
  }

  static screenLines() {
    const file = path.join(this.fixture, 'screen.png')
    writeFileSync(file, this.run(this.adb, ['exec-out', 'screencap', '-p'], { encoding: 'buffer' }))
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
    const x = Math.round(found[0].x * this.screen.width)
    const y = Math.round(found[0].y * this.screen.height)
    this.adbShell('input', 'tap', String(x), String(y))
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
    return spawnSync(this.adb, ['shell', 'pidof', this.packageName], { encoding: 'utf8', timeout: 60_000 }).stdout.trim() !== ''
  }

  static reversed(port) {
    return this.run(this.adb, ['reverse', '--list']).includes(`tcp:${port}`)
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

  static async stopPreview(port) {
    assert.ok(this.running(), `${this.packageName} does not run before ty preview stops`)
    assert.ok(this.reversed(port), `adb reverse holds no tcp:${port} before ty preview stops`)
    process.kill(this.preview.child.pid, 'SIGINT')
    const deadline = Date.now() + 30_000
    while ((this.running() || this.reversed(port)) && Date.now() < deadline) await this.sleep(this.pollMilliseconds)
    assert.ok(!this.running(), `${this.packageName} still runs after ty preview stopped\n${this.output}`)
    assert.ok(!this.reversed(port), `adb reverse still holds tcp:${port} after ty preview stopped`)
    console.log('OK: stopping ty preview closed the app and removed its adb reverse')
  }

  static async main() {
    DevLoopFixture.write(this.fixture)
    this.compileReader()
    this.measureScreen()
    spawnSync(this.adb, ['uninstall', this.packageName], { stdio: 'ignore', timeout: 120_000 })
    const port = await DevLoopFixture.freePort()
    const record = (chunk) => {
      this.output += chunk.toString()
      writeFileSync(this.log, this.output)
    }
    this.preview = await SupervisedCommand.start(this.binary, ['preview', this.fixture, '--target', 'android', '--port', String(port)], {
      cwd: this.repository, env: process.env, timeoutMilliseconds: 1_800_000, onStdout: record, onStderr: record,
    })
    try {
      await this.drive()
      await this.stopPreview(port)
      await this.preview.completion
    } finally {
      if (this.preview.exited !== true) await this.preview.stop()
      spawnSync(this.adb, ['uninstall', this.packageName], { stdio: 'ignore', timeout: 120_000 })
      console.log(`preview log: ${this.log}`)
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1' && existsSync(this.fixture)) rmSync(this.fixture, { recursive: true, force: true })
    }
    console.log(`PASS: ty preview --target android installed the app on ${process.env.ANDROID_SERIAL ?? 'the emulator'} against port ${port} through adb reverse; its Yon calls reached the development server, a style edit arrived without a reload, a page edit reached the app, a manifest edit reinstalled it, and stopping ty preview closed it.`)
  }
}

await AndroidDevLoopTest.main()

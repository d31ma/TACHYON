#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawnSync } from 'node:child_process'
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { DevLoopFixture } from './dev-loop-fixture.mjs'

class MacOsDevLoopTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'macos-dev-loop-'))
  static probe = path.join(MacOsDevLoopTest.fixture, 'page-checks-macos-probe')
  static log = process.env.TAC_DEV_LOOP_LOG ?? path.join(MacOsDevLoopTest.fixture, 'preview.log')
  static opened = 'Opened the macos app'
  static waitAttempts = 2
  static clickAttempts = 6
  static openSeconds = 900
  static pollMilliseconds = 250
  static output = ''
  static preview = null

  static sleep(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds))
  }

  static compileProbe() {
    const source = path.join(this.repository, 'scripts/native/page-checks-macos-probe.swift')
    const result = spawnSync('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit', source, '-o', this.probe], { encoding: 'utf8', timeout: 600_000 })
    assert.equal(result.status, 0, `the probe did not compile\n${result.stdout}${result.stderr}`)
  }

  static async probeOnce(action, values) {
    try {
      await promisify(execFile)(this.probe, [DevLoopFixture.identifier, action, ...values], { timeout: 120_000 })
      return ''
    } catch (error) {
      return String(error.stderr || error.message)
    }
  }

  static async attempt(times, action, values) {
    let failure = ''
    for (let index = 0; index < times; index += 1) {
      failure = await this.probeOnce(action, values)
      if (failure === '') return
    }
    assert.fail(`${action} ${values.join(' | ')} failed:\n${failure}\n${this.output}`)
  }

  static async wait(texts) {
    await this.attempt(this.waitAttempts, 'wait', texts)
  }

  static async click(name) {
    await this.attempt(1, 'press', [name])
  }

  static async clickUntil([name, text]) {
    for (let index = 0; index < this.clickAttempts; index += 1) {
      await this.click(name)
      if (await this.probeOnce('wait', [text]) === '') return
    }
    assert.fail(`pressing ${name} never showed ${text}\n${this.output}`)
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

  static application() {
    const folder = path.join(this.fixture, 'dist/preview/macos')
    const bundle = readdirSync(folder).find((name) => name.endsWith('.app'))
    assert.ok(bundle, `ty preview left no .app in ${folder}`)
    return path.join(folder, bundle, 'Contents/MacOS')
  }

  static running(executables) {
    return spawnSync('/usr/bin/pgrep', ['-f', executables]).status === 0
  }

  static websiteData() {
    const library = path.join(homedir(), 'Library')
    const identifier = DevLoopFixture.identifier
    return [path.join(library, 'WebKit', identifier), path.join(library, 'Caches', identifier),
      path.join(library, 'HTTPStorages', identifier), path.join(library, 'Saved Application State', `${identifier}.savedState`)]
  }

  static forgetWebsiteData() {
    for (const folder of this.websiteData()) rmSync(folder, { recursive: true, force: true })
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

  static async stopPreview(preview, executables) {
    process.kill(preview.child.pid, 'SIGINT')
    const deadline = Date.now() + 10_000
    while (this.running(executables) && Date.now() < deadline) await this.sleep(this.pollMilliseconds)
    assert.ok(!this.running(executables), `the app ${executables} still runs after ty preview stopped\n${this.output}`)
    console.log('OK: stopping ty preview closed the app')
  }

  static async main() {
    DevLoopFixture.write(this.fixture)
    this.compileProbe()
    this.forgetWebsiteData()
    const port = await DevLoopFixture.freePort()
    const record = (chunk) => {
      this.output += chunk.toString()
      writeFileSync(this.log, this.output)
    }
    const preview = this.preview = await SupervisedCommand.start(this.binary, ['preview', this.fixture, '--target', 'macos', '--port', String(port)], {
      cwd: this.repository, env: process.env, timeoutMilliseconds: 1_800_000, onStdout: record, onStderr: record,
    })
    let executables = ''
    try {
      await this.drive()
      executables = this.application()
      await this.stopPreview(preview, executables)
      await preview.completion
    } finally {
      if (preview.exited !== true) await preview.stop()
      if (executables !== '') spawnSync('/usr/bin/pkill', ['-f', executables])
      this.forgetWebsiteData()
      console.log(`preview log: ${this.log}`)
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1' && existsSync(this.fixture)) rmSync(this.fixture, { recursive: true, force: true })
    }
    console.log(`PASS: ty preview --target macos opened the app against http://127.0.0.1:${port}; its Yon calls reached the development server, a style edit arrived without a reload, a page edit reached the app, a manifest edit reopened it, and stopping ty preview closed it.`)
  }
}

await MacOsDevLoopTest.main()

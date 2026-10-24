#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { PageServicesFixture } from '../native/page-services-fixture.mjs'

class IosFilePickerTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-file-picker-'))
  static deviceSet = path.join(IosFilePickerTest.root, 'simulator-devices')
  static evidence = process.env.TAC_IOS_FILE_PICKER_EVIDENCE ?? path.join(IosFilePickerTest.repository, 'target/ios-file-picker-evidence')
  static tapper = path.join(IosFilePickerTest.root, 'simulator-tap')
  static reader = path.join(IosFilePickerTest.root, 'screenshot-text')
  static languages = PageServicesFixture.list('TAC_IOS_PICKER_CALLERS', 'js,swift,rs,cs,kt')
  static linked = ['swift', 'rs']
  static fileName = 'tachyon-fp4-notes'
  static localStorage = 'group.com.apple.FileProvider.LocalStorage'
  static waitMilliseconds = 60000
  static pollMilliseconds = 500
  static device = ''

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args])
  }

  static prepareTools() {
    this.run('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/ios/simulator-tap.m'), '-o', this.tapper])
    this.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(this.repository, 'scripts/ios/screenshot-text.swift'), '-o', this.reader], 600000)
  }

  static bootSimulator() {
    mkdirSync(this.deviceSet)
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.name === 'iPhone 17 Pro')
      ?? (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `FilePicker-${randomUUID()}`, phone.identifier, runtime.identifier)
    console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    this.simctl('boot', this.device)
    this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'bootstatus', this.device, '-b'], 600000)
    return { runtime: runtime.version, device: phone.name }
  }

  static shutdownSimulator() {
    if (!this.device) return
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
  }

  static async seedFile() {
    let groups = ''
    for (let attempt = 0; attempt < 2 && !groups.includes(this.localStorage); attempt++) {
      groups = spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'get_app_container', this.device, 'com.apple.DocumentsApp', 'groups'], { encoding: 'utf8', timeout: 60000 }).stdout ?? ''
      if (groups.includes(this.localStorage)) break
      this.simctl('launch', this.device, 'com.apple.DocumentsApp')
      await sleep(5000)
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'terminate', this.device, 'com.apple.DocumentsApp'], { timeout: 30000, stdio: 'ignore' })
    }
    const line = groups.split('\n').find((item) => item.startsWith(this.localStorage))
    assert.ok(line, `the simulator has no ${this.localStorage} container: ${groups}`)
    const folder = path.join(line.slice(this.localStorage.length).trim(), 'File Provider Storage')
    mkdirSync(folder, { recursive: true })
    const file = path.join(folder, `${this.fileName}.txt`)
    writeFileSync(file, 'Quarterly notes\n')
    return file
  }

  /** @param {string} name */
  static read(name) {
    const file = path.join(this.root, `${name}.png`)
    this.simctl('io', this.device, 'screenshot', file)
    return { file, lines: JSON.parse(this.run(this.reader, [file])) }
  }

  /** @param {string} name @param {(texts: string[]) => boolean} check */
  static async screenWith(name, check) {
    const deadline = Date.now() + this.waitMilliseconds
    let shot = this.read(name)
    const texts = () => shot.lines.map(({ text }) => text.trim())
    while (!check(texts()) && Date.now() < deadline) {
      await sleep(this.pollMilliseconds)
      shot = this.read(name)
    }
    copyFileSync(shot.file, path.join(this.evidence, `${name}.png`))
    return { screenshot: path.join(this.evidence, `${name}.png`), text: texts(), found: check(texts()), lines: shot.lines }
  }

  /** @param {{ x: number, y: number, text: string }} target */
  static tapLine(target) {
    this.run(this.tapper, [this.deviceSet, this.device, String(target.x), String(target.y)])
    return target.text
  }

  /** @param {string} label */
  static tap(label) {
    const shot = this.read('before-tap')
    const target = shot.lines.find(({ text }) => text.trim() === label) ?? shot.lines.find(({ text }) => text.includes(label))
    assert.ok(target, `no "${label}" on screen: ${shot.lines.map(({ text }) => text).join(' | ')}`)
    return this.tapLine(target)
  }

  /** @param {string[]} texts */
  static pickerShown(texts) {
    return texts.includes('Cancel') && !texts.includes('Choose file')
  }

  static async choose() {
    const taps = []
    const deadline = Date.now() + this.waitMilliseconds
    while (Date.now() < deadline) {
      const shot = this.read('picker')
      const find = (check) => shot.lines.find(({ text }) => check(text.trim()))
      const file = find((text) => text.startsWith(this.fileName))
      const next = file ?? find((text) => text === 'On My iPhone') ?? find((text) => text === 'Browse')
      if (next) taps.push(this.tapLine(next))
      if (file) return taps
      await sleep(next ? 1500 : this.pollMilliseconds)
    }
    assert.fail(`the picker never listed ${this.fileName}: ${taps.join(' > ')}`)
  }

  /** @param {string} folder */
  static webAssembly(folder) {
    return readdirSync(folder, { recursive: true }).filter((name) => String(name).endsWith('.wasm')).length
  }

  /** @param {string} language */
  static async check(language) {
    const project = path.join(this.root, `picker-${language}`)
    const { application } = PageServicesFixture.stagePicker(project, language)
    const [read, cancelled] = PageServicesFixture.pickerStatuses[language]
    const applicationId = `dev.tachyon.picker-${language}`
    this.run(this.binary, ['bundle', project, '--target', 'ios'], 1800000)
    const bundle = path.join(project, 'dist/ios', `${application}.app`)
    const modules = this.webAssembly(path.join(bundle, 'WebBundle'))
    if (this.linked.includes(language) || language === 'js') assert.equal(modules, 0, `${language} runs without WebAssembly`)
    else assert.ok(modules > 0, `${language}: the bundle has no WebAssembly`)
    this.simctl('install', this.device, bundle)
    this.simctl('launch', this.device, applicationId)
    try {
      const started = await this.screenWith(`${language}-ready`, (texts) => texts.includes('Choose file') && texts.includes('Ready'))
      assert.ok(started.found, `${language}: the page did not show Ready: ${started.text.join(' | ')}`)
      this.tap('Choose file')
      const picker = await this.screenWith(`${language}-picker`, (texts) => this.pickerShown(texts))
      assert.ok(picker.found, `${language}: the document picker did not open: ${picker.text.join(' | ')}`)
      const taps = await this.choose()
      const chosen = await this.screenWith(`${language}-read`, (texts) => texts.includes(read))
      assert.ok(chosen.found, `${language}: the page did not show "${read}": ${chosen.text.join(' | ')}`)
      this.tap('Choose file')
      const again = await this.screenWith(`${language}-picker-again`, (texts) => this.pickerShown(texts))
      assert.ok(again.found, `${language}: the document picker did not open again: ${again.text.join(' | ')}`)
      this.tap('Cancel')
      const closed = await this.screenWith(`${language}-cancelled`, (texts) => texts.includes(cancelled))
      assert.ok(closed.found, `${language}: the page did not show "${cancelled}": ${closed.text.join(' | ')}`)
      const result = { language, linked: this.linked.includes(language), webAssembly: modules, taps, read, cancelled, screenshots: [picker.screenshot, chosen.screenshot, closed.screenshot] }
      console.log(`==> ${language}: ${result.linked ? 'linked' : modules ? 'WebAssembly' : 'page'} FilePicker chose ${this.fileName}.txt (${taps.join(' > ')}), read it (${read}), and Cancel answered ${cancelled}`)
      return result
    } finally {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'terminate', this.device, applicationId], { timeout: 30000, stdio: 'ignore' })
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'uninstall', this.device, applicationId], { timeout: 60000, stdio: 'ignore' })
    }
  }

  static async main() {
    mkdirSync(this.evidence, { recursive: true })
    const report = { languages: this.languages, results: [], failures: [] }
    try {
      this.prepareTools()
      Object.assign(report, this.bootSimulator())
      report.seeded = await this.seedFile()
      for (const language of this.languages) {
        try {
          report.results.push(await this.check(language))
        } catch (error) {
          console.log(`==> ${language}: FAIL ${String(error.message).slice(0, 2000)}`)
          report.failures.push({ language, error: String(error.message).slice(0, 4000) })
        }
        writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      }
    } finally {
      this.shutdownSimulator()
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    assert.deepEqual(report.failures.map(({ language }) => language), [], 'some FilePicker callers failed; see the report')
    console.log(`PASS: on the iOS ${report.runtime} simulator (${report.device}) FilePicker callers (${this.languages.join(', ')}) chose and read a file through the document picker and answered cancelled after Cancel. Report: ${path.join(this.evidence, 'report.json')}`)
  }
}

await IosFilePickerTest.main()

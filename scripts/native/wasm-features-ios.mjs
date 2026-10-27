import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { WasmFeaturesFixture } from './wasm-features-fixture.mjs'

/** Runs one in-app WebAssembly feature check in the iOS simulator app: a linked Swift recorder writes the probe's verdict into the app's tmp folder, and screen text drives the click steps. */
export class IosWasmApp {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static proofFile = 'tachyon-wasm-probe.json'
  static recorderService = 'Swift wasmRecorder'
  static recorder = {
    'client/services/WasmRecorderService.swift': `import Foundation

@Service
final class WasmRecorderService {
    var state: String = "ready"

    func record(_ proof: String) -> Int {
        let pid = Int(ProcessInfo.processInfo.processIdentifier)
        let record: [String: Any] = ["pid": pid, "proof": proof]
        let destination = FileManager.default.temporaryDirectory.appendingPathComponent("${IosWasmApp.proofFile}")
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: destination, options: .atomic)
        }
        state = "recorded"
        return pid
    }
}
`,
  }

  static chosenDevice = process.env.TAC_IOS_DEVICE ?? ''
  static runtimeVersion = process.env.TAC_IOS_RUNTIME ?? ''
  static evidence = process.env.TAC_IOS_WASM_FEATURES_EVIDENCE ?? path.join(IosWasmApp.repository, 'target/ios-wasm-features-evidence')
  static waitMilliseconds = 360000
  static installMilliseconds = 900000
  static retapMilliseconds = 8000
  static pollMilliseconds = 500
  static notificationPrompt = /Would Like to Send You Notifications/i
  static shareSheet = ['AirDrop', 'Messages', 'Mail', 'Copy', 'Save to Files']
  static pasteAnswers = ['Paste', 'Allow Paste']
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = ''
  static deviceSet = ''
  static device = ''
  static bootedVersion = ''
  static tapper = ''
  static reader = ''

  /** @param {string} program @param {string[]} args @param {number} [timeout] */
  static tool(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return `${result.stdout}${result.stderr}`.trim()
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.tool('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args])
  }

  /** @param {string[]} args */
  static slowSimctl(...args) {
    return this.tool('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], this.installMilliseconds)
  }

  /** @param {string} version */
  static chosenVersion(version) {
    return !this.runtimeVersion || version === this.runtimeVersion || version.startsWith(`${this.runtimeVersion}.`)
  }

  /** Compiles the tap and screen-text tools and boots one simulator for every check. @param {string} root */
  static prepare(root) {
    this.root = root
    mkdirSync(this.evidence, { recursive: true })
    this.tapper = path.join(root, 'simulator-tap')
    this.reader = path.join(root, 'screenshot-text')
    this.tool('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation', '-framework', 'AppKit', path.join(this.repository, 'scripts/ios/simulator-tap.m'), '-o', this.tapper])
    this.tool('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(this.repository, 'scripts/ios/screenshot-text.swift'), '-o', this.reader], 600000)
    this.deviceSet = process.env.TAC_IOS_DEVICE_SET ?? (this.chosenDevice ? path.join(homedir(), 'Library/Developer/CoreSimulator/Devices') : path.join(root, 'simulator-devices'))
    return this.chosenDevice ? this.useBootedSimulator() : this.bootSimulator()
  }

  static useBootedSimulator() {
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const devices = JSON.parse(this.simctl('list', 'devices', '--json')).devices
    const [identifier, found] = Object.entries(devices).map(([key, list]) => [key, list.find((item) => item.udid === this.chosenDevice)]).find(([, item]) => item) ?? []
    assert.ok(found, `TAC_IOS_DEVICE ${this.chosenDevice} is not in the device set ${this.deviceSet}`)
    assert.equal(found.state, 'Booted', `TAC_IOS_DEVICE ${this.chosenDevice} must already be booted`)
    const runtime = runtimes.find((item) => item.identifier === identifier)
    assert.ok(runtime && this.chosenVersion(runtime.version), `TAC_IOS_DEVICE ${this.chosenDevice} runs ${identifier}, not iOS ${this.runtimeVersion}`)
    this.device = this.chosenDevice
    this.bootedVersion = runtime.version
    console.log(`==> using booted ${found.name} with iOS ${runtime.version}`)
  }

  static bootSimulator() {
    mkdirSync(this.deviceSet, { recursive: true })
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-') && this.chosenVersion(item.version))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, `an installed iOS simulator runtime${this.runtimeVersion ? ` ${this.runtimeVersion}` : ''} is required`)
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.name === 'iPhone 17 Pro')
      ?? (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `WasmFeatures-${randomUUID()}`, phone.identifier, runtime.identifier)
    this.bootedVersion = runtime.version
    console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    this.simctl('boot', this.device)
    this.tool('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'bootstatus', this.device, '-b'], 600000)
  }

  static close() {
    if (!this.device || this.chosenDevice) return
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
  }

  /** A loaded simulator sometimes times out on a screenshot ("Timeout waiting for screen surfaces"), so it gets three tries. @param {string} name */
  static screen(name) {
    const file = path.join(this.root, `${name}.png`)
    for (let attempt = 1; ; attempt += 1) {
      try {
        this.simctl('io', this.device, 'screenshot', file)
        break
      } catch (error) {
        if (attempt === 3) throw error
      }
    }
    return { file, lines: /** @type {Array<{ text: string, x: number, y: number }>} */ (JSON.parse(this.tool(this.reader, [file]))) }
  }

  /** @param {number} x @param {number} y */
  static tapAt(x, y) {
    this.tool(this.tapper, [this.deviceSet, this.device, String(x), String(y)])
  }

  /** @param {Array<{ text: string, x: number, y: number }>} lines @param {string} label */
  static tapLabel(lines, label) {
    const target = lines.find(({ text }) => text.trim() === label)
    if (!target) return false
    this.tapAt(target.x, target.y)
    return true
  }

  /**
   * Answers what the screen asks for until the probe's verdict lands: taps the button the probe waits for, allows the notification prompt, pastes when iOS asks, and closes the share sheet.
   * @param {string} application @param {string} proof @param {string[]} gestures
   */
  static async drive(application, proof, gestures) {
    const deadline = Date.now() + this.waitMilliseconds
    const taps = []
    let tapped = { label: '', at: 0 }
    let shot = { file: '', lines: [] }
    while (Date.now() < deadline && !existsSync(proof)) {
      if (gestures.length) {
        shot = this.screen(application)
        const text = shot.lines.map((line) => line.text).join(' ').replace(/\s+/g, ' ')
        const waiting = gestures.find((label) => text.includes(`waiting for ${label}`))
        if (this.notificationPrompt.test(text) && this.tapLabel(shot.lines, 'Allow')) taps.push('Allow (notification prompt)')
        else if (this.pasteAnswers.some((label) => this.tapLabel(shot.lines, label))) taps.push('Paste (iOS paste prompt)')
        else if (shot.lines.filter(({ text }) => this.shareSheet.includes(text.trim())).length >= 2) {
          this.tapAt(0.5, 0.08)
          taps.push('closed the share sheet')
        } else if (waiting && (tapped.label !== waiting || Date.now() - tapped.at > this.retapMilliseconds)) {
          assert.ok(this.tapLabel(shot.lines, waiting), `the probe waits for "${waiting}" but the button is not on screen: ${shot.lines.map(({ text }) => text).join(' | ')}`)
          tapped = { label: waiting, at: Date.now() }
          taps.push(waiting)
        }
      }
      await sleep(this.pollMilliseconds)
    }
    shot = this.screen(application)
    copyFileSync(shot.file, path.join(this.evidence, `${application}.png`))
    assert.ok(existsSync(proof), `the app wrote no probe verdict in ${this.waitMilliseconds / 1000} s; taps ${JSON.stringify(taps)}; the screen shows: ${shot.lines.map(({ text }) => text).join(' | ')}`)
    return taps
  }

  /** @param {string} project @param {{ application: string, id: string, gestures: string[] }} staged */
  static async run(project, staged) {
    const built = await SupervisedCommand.run(this.binary, ['build', project, '--target', 'ios'], { cwd: this.repository, timeoutMilliseconds: 1800000 })
    assert.ok(built.status === 0 && built.cleanupConfirmed, `ty build --target ios\n${built.error?.message ?? ''}\n${built.stdout}${built.stderr}`)
    const output = path.join(project, 'dist/ios')
    const bundle = path.join(output, `${staged.application}.app`)
    const modules = WasmFeaturesFixture.assertWebAssembly(output, path.join(bundle, 'WebBundle'), [this.recorderService])
    const minimum = this.tool('/usr/bin/plutil', ['-extract', 'MinimumOSVersion', 'raw', path.join(bundle, 'Info.plist')])
    assert.ok(this.bootedVersion.localeCompare(minimum, undefined, { numeric: true }) >= 0, `the app's MinimumOSVersion is ${minimum}, above the booted iOS ${this.bootedVersion}; pick a runtime with TAC_IOS_RUNTIME`)
    this.slowSimctl('install', this.device, bundle)
    try {
      const launched = this.slowSimctl('launch', this.device, staged.id)
      const pid = Number(launched.match(/:\s*(\d+)\s*$/)?.[1])
      const proof = path.join(this.simctl('get_app_container', this.device, staged.id, 'data'), 'tmp', this.proofFile)
      const taps = await this.drive(staged.application, proof, staged.gestures)
      const record = JSON.parse(readFileSync(proof, 'utf8'))
      assert.equal(record.pid, pid, 'the verdict came from the launched app')
      const verdict = WasmFeaturesFixture.assertProof(record.proof)
      const lines = [`iOS ${this.bootedVersion}, MinimumOSVersion ${minimum}, pid ${pid}${taps.length ? `, taps: ${taps.join(', ')}` : ''}`, ...verdict.lines]
      return { modules, lines }
    } finally {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'terminate', this.device, staged.id], { timeout: 30000, stdio: 'ignore' })
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'uninstall', this.device, staged.id], { timeout: 60000, stdio: 'ignore' })
    }
  }
}

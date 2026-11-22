#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { FoldPosture } from './fold-posture.mjs'
import { FoldableProbe } from './foldable-probe.mjs'
import { ScreenshotPixels } from './screenshot-pixels.mjs'
import { SharedSimulator } from './shared-simulator.mjs'

assert.equal(process.platform, 'darwin', 'The iOS foldable check needs macOS and Xcode')

class IosFoldableGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
  static root = mkdtempSync(path.join(tmpdir(), 'tachyon-ios-fold-'))
  static deviceSet = SharedSimulator.deviceSet(path.join(IosFoldableGate.root, 'simulator-devices'))
  static appsFolder = process.env.TAC_IOS_FOLD_APPS ?? IosFoldableGate.root
  static buildOnly = process.env.TAC_IOS_FOLD_BUILD_ONLY === '1'
  static evidence = process.env.TAC_IOS_FOLD_EVIDENCE ?? path.join(IosFoldableGate.repo, 'target/ios-foldable-evidence')
  static family = 'iPhoneDuo'
  static deviceTypeName = 'iPhone Duo'
  static installMilliseconds = 900000
  static settleMilliseconds = 30000
  static pollMilliseconds = 250
  static freshReadings = 3
  static edgeShare = 0.9
  static tolerance = 0.5
  static postures = ['closed', 'half_open', 'open']
  static apps = {
    free: { appId: 'dev.tachyon.fold-free', color: '#ffaa00', orientation: 'any' },
    portrait: { appId: 'dev.tachyon.fold-portrait', color: '#00aa66', orientation: 'portrait' },
  }
  static createdDevices = []

  /** @param {string} program @param {string[]} args @param {number} [timeout] @returns {string} */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { cwd: IosFoldableGate.repo, encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`)
    return result.stdout.trim()
  }

  /** @param {...string} args @returns {string} */
  static simctl(...args) {
    return IosFoldableGate.run('/usr/bin/xcrun', ['simctl', '--set', IosFoldableGate.deviceSet, ...args], IosFoldableGate.installMilliseconds)
  }

  /** @param {number} milliseconds */
  static pause(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds))
  }

  /** @returns {string} */
  static prebuiltBinary() {
    if (IosFoldableGate.appsFolder === IosFoldableGate.root || IosFoldableGate.buildOnly) return IosFoldableGate.binary()
    return ''
  }

  /** @returns {string} */
  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN
    const { run } = IosFoldableGate
    run('cargo', ['build', '--locked', '--bin', 'ty'], 1200000)
    return path.join(JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty')
  }

  /** @param {string} binary @param {string} name @returns {string} */
  static build(binary, name) {
    const project = path.join(IosFoldableGate.appsFolder, name)
    const bundle = path.join(project, 'dist/ios/FoldProbe.app')
    if (IosFoldableGate.prebuilt(bundle)) return IosFoldableGate.describe(IosFoldableGate.localCopy(bundle, name), name)
    FoldableProbe.stage(project, IosFoldableGate.apps[name])
    console.log(`==> building the ${name} iOS app`)
    IosFoldableGate.run(binary, ['build', project, '--target', 'ios'], 600000)
    return IosFoldableGate.describe(bundle, name)
  }

  /** @param {string} bundle @returns {boolean} */
  static prebuilt(bundle) {
    const kept = IosFoldableGate.appsFolder !== IosFoldableGate.root && !IosFoldableGate.buildOnly && existsSync(bundle)
    if (kept) console.log(`==> using the app built before: ${bundle}`)
    return kept
  }

  /** @param {string} bundle @param {string} name @returns {string} */
  static localCopy(bundle, name) {
    const copy = path.join(IosFoldableGate.root, 'apps', name, 'FoldProbe.app')
    mkdirSync(path.dirname(copy), { recursive: true })
    IosFoldableGate.run('/usr/bin/ditto', [bundle, copy])
    return copy
  }

  /** @param {string} bundle @param {string} name @returns {string} */
  static describe(bundle, name) {
    const fullScreen = spawnSync('/usr/bin/plutil', ['-extract', 'UIRequiresFullScreen', 'raw', path.join(bundle, 'Info.plist')], { encoding: 'utf8' })
    console.log(`==> ${name}: UIRequiresFullScreen ${fullScreen.status === 0 ? fullScreen.stdout.trim() : 'not set'}`)
    return bundle
  }

  /** @returns {string} */
  static device() {
    if (SharedSimulator.udid(IosFoldableGate.family)) return SharedSimulator.use(IosFoldableGate.deviceSet, IosFoldableGate.family).device
    return IosFoldableGate.createDevice()
  }

  /** @returns {string} */
  static createDevice() {
    const { simctl, deviceSet, deviceTypeName } = IosFoldableGate
    mkdirSync(deviceSet, { recursive: true })
    const runtime = SharedSimulator.list(deviceSet, ['runtimes']).runtimes
      .filter((item) => item.isAvailable && item.identifier.includes('.iOS-') && SharedSimulator.wantedRuntime(item))
      .find((item) => item.supportedDeviceTypes.some((type) => type.name === deviceTypeName))
    assert.ok(runtime, `an installed iOS runtime with the ${deviceTypeName} device type is required`)
    const type = runtime.supportedDeviceTypes.find((item) => item.name === deviceTypeName)
    const device = simctl('create', `TachyonFold-${randomUUID()}`, type.identifier, runtime.identifier)
    IosFoldableGate.createdDevices.push(device)
    console.log(`==> booting a new ${deviceTypeName} with iOS ${runtime.version}`)
    simctl('boot', device)
    simctl('bootstatus', device, '-b')
    return device
  }

  /** @param {string} device @param {string} appId @returns {string} */
  static probeFile(device, appId) {
    return path.join(IosFoldableGate.simctl('get_app_container', device, appId, 'data'), 'Documents', FoldableProbe.PROBE_FILE)
  }

  /** @param {string} file @returns {any} */
  static reading(file) {
    if (!existsSync(file)) return null
    try {
      return JSON.parse(readFileSync(file, 'utf8'))
    } catch {
      return null
    }
  }

  /** @param {number[]} first @param {number[]} second @returns {boolean} */
  static same(first, second) {
    return Array.isArray(first) && Array.isArray(second) && first.every((value, index) => Math.abs(value - second[index]) <= IosFoldableGate.tolerance)
  }

  /** @param {any} record @returns {boolean} */
  static settled(record) {
    const { native, page } = record
    const screen = [0, 0, native.screenBounds[2], native.screenBounds[3]]
    const webSize = native.webFrame ?? [0, 0, -1, -1]
    return IosFoldableGate.same(native.windowOnScreen, screen) && Math.round(webSize[2]) === page.width && Math.round(webSize[3]) === page.height
  }

  /** @param {string} file @param {number} after @returns {Promise<any>} */
  static async settle(file, after) {
    const { pause, reading, settled, settleMilliseconds, pollMilliseconds, freshReadings } = IosFoldableGate
    const started = Date.now()
    let record = reading(file)
    while (Date.now() - started < settleMilliseconds) {
      record = reading(file)
      const fresh = record && record.page.sequence >= after + freshReadings
      if (fresh && settled(record)) return record
      await pause(pollMilliseconds)
    }
    assert.ok(record, `the app never wrote ${file}`)
    return record
  }

  /** @param {string} file @param {string} name */
  static keep(file, name) {
    mkdirSync(IosFoldableGate.evidence, { recursive: true })
    copyFileSync(file, path.join(IosFoldableGate.evidence, name))
  }

  /** @param {string} device @param {string} display @param {string} name @returns {import('./screenshot-pixels.mjs').ScreenshotImage} */
  static screenshot(device, display, name) {
    const file = path.join(IosFoldableGate.root, name)
    IosFoldableGate.simctl('io', device, 'screenshot', `--display=${display}`, '--mask=ignored', file)
    IosFoldableGate.keep(file, name)
    return ScreenshotPixels.read(file)
  }

  /** @param {string} label @param {any} record @param {import('./screenshot-pixels.mjs').ScreenshotImage} image @param {string} color */
  static checkFills(label, record, image, color) {
    const { native } = record
    const [, , width, height] = native.screenBounds
    assert.ok(IosFoldableGate.same(native.windowOnScreen, [0, 0, width, height]), `${label}: the window ${native.windowOnScreen} fills the screen ${width}x${height}`)
    assert.ok(IosFoldableGate.same(native.sceneBounds, [0, 0, width, height]), `${label}: the scene ${native.sceneBounds} fills the screen ${width}x${height}`)
    assert.equal(image.width, Math.round(width * native.scale), `${label}: the lit screen is ${image.width} px wide, the app's screen ${width * native.scale}`)
    assert.equal(image.height, Math.round(height * native.scale), `${label}: the lit screen is ${image.height} px high, the app's screen ${height * native.scale}`)
    const edges = {
      left: ScreenshotPixels.columnShare(image, 1, color),
      right: ScreenshotPixels.columnShare(image, image.width - 2, color),
      top: ScreenshotPixels.rowShare(image, 1, color),
      bottom: ScreenshotPixels.rowShare(image, image.height - 2, color),
    }
    const bars = Object.entries(edges).filter(([, share]) => share < IosFoldableGate.edgeShare)
    assert.deepEqual(bars, [], `${label}: no black bars; each screen edge shows the app's ${color}: ${JSON.stringify(edges)}`)
  }

  /** @param {string} label @param {any} record */
  static checkLayout(label, record) {
    const { native, page } = record
    const [webLeft, webTop, webWidth, webHeight] = native.webFrame
    const [, , windowWidth, windowHeight] = native.windowBounds
    assert.equal(page.width, Math.round(webWidth), `${label}: the page is as wide as its web view`)
    assert.equal(page.height, Math.round(webHeight), `${label}: the page is as high as its web view`)
    assert.equal(webLeft, 0, `${label}: the web view starts at the left edge`)
    assert.ok(Math.abs(webWidth - windowWidth) <= IosFoldableGate.tolerance, `${label}: the web view is as wide as the window`)
    assert.ok(Math.abs(webTop + webHeight - windowHeight) <= IosFoldableGate.tolerance, `${label}: the web view reaches the bottom of the window`)
  }

  /** @param {string} label @param {any} record */
  static checkSafeAreas(label, record) {
    const { native, page } = record
    const { safe, webSafe } = native
    const [, webTop] = native.webFrame
    const [, , width, height] = native.windowBounds
    assert.ok(Object.values(safe).every((inset) => inset >= 0), `${label}: no safe area inset is negative: ${JSON.stringify(safe)}`)
    assert.ok(safe.top > 0, `${label}: the status bar gives a top safe area: ${JSON.stringify(safe)}`)
    assert.ok(safe.left + safe.right < width && safe.top + safe.bottom < height, `${label}: the safe area leaves room inside the window: ${JSON.stringify(safe)}`)
    assert.equal(webTop, safe.top, `${label}: the standard page starts below the status bar`)
    const expected = { top: Math.round(webSafe.top), right: Math.round(webSafe.right), bottom: Math.round(webSafe.bottom), left: Math.round(webSafe.left) }
    assert.deepEqual(page.safe, expected, `${label}: the page's env(safe-area-inset-*) match the web view's safe area`)
    assert.equal(Math.round(webSafe.bottom), Math.round(safe.bottom), `${label}: the bottom safe area reaches the page`)
  }

  /** @param {string} label @param {any} before @param {any} after */
  static checkRelayout(label, before, after) {
    if (IosFoldableGate.same(before.native.screenBounds, after.native.screenBounds)) return
    assert.ok(after.layouts > before.layouts, `${label}: the app laid out again after the screen changed (${before.layouts} -> ${after.layouts})`)
    assert.ok(!IosFoldableGate.same(before.native.windowBounds, after.native.windowBounds), `${label}: the window took the new screen size`)
  }

  /** @param {string} device @param {string} bundle @param {string} name @returns {Promise<string[]>} */
  static async checkApp(device, bundle, name) {
    const { simctl, apps } = IosFoldableGate
    const { appId, color } = apps[name]
    await FoldPosture.set(IosFoldableGate.deviceSet, device, 'closed')
    spawnSync('/usr/bin/xcrun', ['simctl', '--set', IosFoldableGate.deviceSet, 'uninstall', device, appId], { stdio: 'ignore', timeout: 60000 })
    simctl('install', device, bundle)
    simctl('launch', device, appId)
    const file = IosFoldableGate.probeFile(device, appId)
    const failures = []
    let previous = null
    try {
      for (const posture of IosFoldableGate.postures) {
        const after = IosFoldableGate.reading(file)?.page.sequence ?? 0
        const lit = await FoldPosture.set(IosFoldableGate.deviceSet, device, posture)
        const record = await IosFoldableGate.settle(file, after)
        const label = `${name} ${posture} (${lit.display})`
        const image = IosFoldableGate.screenshot(device, lit.display, `${name}-${posture}.png`)
        console.log(`${label}: ${JSON.stringify(record)}`)
        IosFoldableGate.attempt(failures, () => IosFoldableGate.checkFills(label, record, image, color))
        IosFoldableGate.attempt(failures, () => IosFoldableGate.checkLayout(label, record))
        IosFoldableGate.attempt(failures, () => IosFoldableGate.checkSafeAreas(label, record))
        if (previous) IosFoldableGate.attempt(failures, () => IosFoldableGate.checkRelayout(label, previous, record))
        previous = record
      }
    } finally {
      SharedSimulator.release(IosFoldableGate.deviceSet, device, [appId])
    }
    return failures
  }

  /** @param {string[]} failures @param {() => void} check */
  static attempt(failures, check) {
    try {
      check()
    } catch (error) {
      console.log(`FAIL ${error.message}`)
      failures.push(error.message)
    }
  }

  static cleanup() {
    for (const device of IosFoldableGate.createdDevices) SharedSimulator.remove(IosFoldableGate.deviceSet, device)
    rmSync(IosFoldableGate.root, { recursive: true, force: true })
  }

  static async main() {
    SharedSimulator.families()
    if (!SharedSimulator.wants(IosFoldableGate.family)) {
      console.log(`skipped: ${SharedSimulator.FAMILIES_VARIABLE} does not name ${IosFoldableGate.family}`)
      return IosFoldableGate.cleanup()
    }
    let device = ''
    try {
      const binary = IosFoldableGate.prebuiltBinary()
      const bundles = Object.fromEntries(Object.keys(IosFoldableGate.apps).map((name) => [name, IosFoldableGate.build(binary, name)]))
      if (IosFoldableGate.buildOnly) return console.log(`==> built the apps in ${IosFoldableGate.appsFolder}; no simulator used`)
      device = IosFoldableGate.device()
      const failures = []
      for (const name of Object.keys(bundles)) failures.push(...await IosFoldableGate.checkApp(device, bundles[name], name))
      console.log(`==> screenshots kept in ${IosFoldableGate.evidence}`)
      assert.deepEqual(failures, [], `the foldable checks failed:\n${failures.join('\n')}`)
      console.log('PASS: iOS foldable: on the iPhone Duo, closed, half open and open flat, both a free and a portrait-locked app fill the lit screen with no black bars, lay out again after each fold, and keep their safe areas')
    } finally {
      if (device) await FoldPosture.set(IosFoldableGate.deviceSet, device, 'closed').catch(() => {})
      IosFoldableGate.cleanup()
    }
  }
}

process.once('SIGINT', () => { IosFoldableGate.cleanup(); process.exit(130) })
await IosFoldableGate.main()

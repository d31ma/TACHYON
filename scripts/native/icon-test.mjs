#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { deflateSync, inflateSync } from 'node:zlib'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'

/** @typedef {{ width: number, height: number, pixel: (x: number, y: number) => number[] }} IconPicture */
/** @typedef {{ project: string, bundle: string, stdout: string }} IconBuild */

class IconPng {
  static signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
  static channels = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }
  static crcTable = Array.from({ length: 256 }, (_, index) => {
    let value = index
    for (let bit = 0; bit < 8; bit += 1) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
    return value >>> 0
  })

  /** @param {Buffer} bytes */
  static crc(bytes) {
    let value = 0xffffffff
    for (const byte of bytes) value = this.crcTable[(value ^ byte) & 0xff] ^ (value >>> 8)
    return (value ^ 0xffffffff) >>> 0
  }

  /** @param {string} kind @param {Buffer} data */
  static chunk(kind, data) {
    const length = Buffer.alloc(4)
    length.writeUInt32BE(data.length)
    const body = Buffer.concat([Buffer.from(kind, 'ascii'), data])
    const crc = Buffer.alloc(4)
    crc.writeUInt32BE(this.crc(body))
    return Buffer.concat([length, body, crc])
  }

  /** @param {number} edge @param {number[]} color */
  static solid(edge, color) {
    const header = Buffer.alloc(13)
    header.writeUInt32BE(edge, 0)
    header.writeUInt32BE(edge, 4)
    header.set([8, 2, 0, 0, 0], 8)
    const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: edge }, () => color).flat())])
    const pixels = deflateSync(Buffer.concat(Array.from({ length: edge }, () => row)))
    return Buffer.concat([this.signature, this.chunk('IHDR', header), this.chunk('IDAT', pixels), this.chunk('IEND', Buffer.alloc(0))])
  }

  /** @param {Buffer} bytes @returns {IconPicture} */
  static decode(bytes) {
    assert.ok(bytes.subarray(0, 8).equals(this.signature), 'not a PNG file')
    let offset = 8
    let header
    let palette = Buffer.alloc(0)
    const data = []
    while (offset < bytes.length) {
      const length = bytes.readUInt32BE(offset)
      const kind = bytes.toString('ascii', offset + 4, offset + 8)
      const body = bytes.subarray(offset + 8, offset + 8 + length)
      if (kind === 'IHDR') header = { width: body.readUInt32BE(0), height: body.readUInt32BE(4), depth: body[8], type: body[9], interlace: body[12] }
      if (kind === 'PLTE') palette = body
      if (kind === 'IDAT') data.push(body)
      offset += length + 12
    }
    assert.ok(header && header.interlace === 0 && [8, 16].includes(header.depth), `unsupported PNG: ${JSON.stringify(header)}`)
    const samples = this.channels[header.type]
    const step = samples * (header.depth / 8)
    const stride = header.width * step
    const raw = inflateSync(Buffer.concat(data))
    const rows = Buffer.alloc(stride * header.height)
    for (let y = 0; y < header.height; y += 1) this.unfilter(raw, rows, y, stride, step)
    return {
      width: header.width,
      height: header.height,
      pixel: (x, y) => this.color(rows, (y * header.width + x) * step, header, palette),
    }
  }

  /** @param {Buffer} raw @param {Buffer} rows @param {number} y @param {number} stride @param {number} step */
  static unfilter(raw, rows, y, stride, step) {
    const filter = raw[y * (stride + 1)]
    const source = y * (stride + 1) + 1
    const target = y * stride
    for (let x = 0; x < stride; x += 1) {
      const left = x >= step ? rows[target + x - step] : 0
      const up = y > 0 ? rows[target - stride + x] : 0
      const corner = x >= step && y > 0 ? rows[target - stride + x - step] : 0
      const predictors = [0, left, up, (left + up) >> 1, this.paeth(left, up, corner)]
      rows[target + x] = (raw[source + x] + predictors[filter]) & 0xff
    }
  }

  /** @param {number} left @param {number} up @param {number} corner */
  static paeth(left, up, corner) {
    const estimate = left + up - corner
    const distances = [Math.abs(estimate - left), Math.abs(estimate - up), Math.abs(estimate - corner)]
    const smallest = Math.min(...distances)
    return [left, up, corner][distances.indexOf(smallest)]
  }

  /** @param {Buffer} rows @param {number} at @param {{ depth: number, type: number }} header @param {Buffer} palette */
  static color(rows, at, header, palette) {
    const wide = header.depth / 8
    const sample = (index) => rows[at + index * wide]
    const readers = {
      0: () => [sample(0), sample(0), sample(0)],
      2: () => [sample(0), sample(1), sample(2)],
      3: () => [...palette.subarray(sample(0) * 3, sample(0) * 3 + 3)],
      4: () => [sample(0), sample(0), sample(0)],
      6: () => [sample(0), sample(1), sample(2)],
    }
    return readers[header.type]()
  }

  /** @param {IconPicture} picture */
  static center(picture) {
    return picture.pixel(Math.floor(picture.width / 2), Math.floor(picture.height / 2))
  }

  /** @param {number[]} found @param {number[]} wanted @param {number} tolerance */
  static near(found, wanted, tolerance = IconGate.tolerance) {
    return found.every((channel, index) => Math.abs(channel - wanted[index]) <= tolerance)
  }

  /** @param {IconPicture} picture @param {number[]} wanted @param {{ top?: number, bottom?: number }} rows */
  static count(picture, wanted, rows = {}) {
    let found = 0
    const top = rows.top ?? 0
    const bottom = rows.bottom ?? picture.height
    for (let y = top; y < bottom; y += 1) {
      for (let x = 0; x < picture.width; x += 1) found += Number(this.near(picture.pixel(x, y), wanted))
    }
    return found
  }
}

class IconFixture {
  static name = 'A Icon Gate'
  static executable = 'AIconGate'
  static id = 'dev.tachyon.icon-gate'
  static themedName = 'dev.tachyon.icon_gate'
  static large = { file: '/shared/icons/gate-1024.png', edge: 1024, color: [255, 0, 170] }
  static small = { file: '/shared/icons/gate-256.png', edge: 256, color: [0, 200, 120] }
  static tray = { file: '/shared/icons/gate-tray.png', edge: 64, color: [255, 140, 0] }
  static desktop = ['macos', 'windows', 'linux']

  /** @param {string} project @param {string} target */
  static stage(project, target) {
    const tray = this.desktop.includes(target) ? { tray: { icon: this.tray.file, template: false, tooltip: 'Icon gate tray' } } : {}
    const manifest = {
      name: this.name,
      start_url: '/',
      icons: [
        { src: '/shared/icons/gate.svg', sizes: 'any', type: 'image/svg+xml' },
        { src: this.small.file, sizes: '256x256', type: 'image/png' },
        { src: this.large.file, sizes: '1024x1024', type: 'image/png' },
      ],
      tac: {
        application: { id: this.id, version: '1.0.0', build: 1, ...tray },
        names: { check: false },
        constants: { check: false },
        visibility: { check: false },
        duplication: { check: false },
        cohesion: { check: false },
      },
    }
    this.write(project, 'client/manifest.json', JSON.stringify(manifest, null, 2))
    this.write(project, 'client/pages/tac.html', '<gate-icon />\n')
    this.write(project, 'client/components/gate/icon/tac.html', '<main><h1>Icon gate</h1><p>The app icon comes from client/manifest.json.</p></main>\n')
    this.write(project, 'client/shared/icons/gate.svg', '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="8" height="8" fill="#000"/></svg>\n')
    for (const icon of [this.large, this.small, this.tray]) this.write(project, `client${icon.file}`, IconPng.solid(icon.edge, icon.color))
  }

  /** @param {string} project @param {string} relative @param {string | Buffer} contents */
  static write(project, relative, contents) {
    const file = path.join(project, relative)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }
}

class IconGate {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'icon-gate-'))
  static tolerance = Number(process.env.TAC_ICON_TOLERANCE ?? 40)
  static screen = process.argv.includes('--screen')
  static release = process.argv.includes('--release')
  static keep = process.argv.includes('--keep')
  static defaults = { darwin: 'macos,ios,android', win32: 'windows', linux: 'linux' }
  static built = /^Built \w+ app hosting \d+ routes to (.+) \([0-9a-f]+\) compiled=/m
  static results = []

  /** @param {string} name @param {string} fallback */
  static option(name, fallback) {
    return process.argv.find((argument) => argument.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback
  }

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} [options] */
  static async run(command, args, options = {}) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, timeoutMilliseconds: 600000, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} [options] */
  static async bytes(command, args, options = {}) {
    const chunks = []
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, timeoutMilliseconds: 300000, onStdout: (chunk) => chunks.push(chunk), ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stderr}`)
    return Buffer.concat(chunks)
  }

  /** @param {string} target @param {string[]} [extra] @returns {Promise<IconBuild>} */
  static async bundle(target, extra = []) {
    const project = path.join(this.root, target)
    IconFixture.stage(project, target)
    const stdout = await this.run(this.binary, ['bundle', project, '--target', target, '--no-incremental', ...extra], { timeoutMilliseconds: 3600000 })
    const bundle = stdout.match(this.built)?.[1]
    assert.ok(bundle, `ty bundle printed no app path:\n${stdout}`)
    assert.ok(!/has no icon/.test(stdout), `the ${target} build says the app has no icon:\n${stdout}`)
    return { project, bundle: path.resolve(project, bundle), stdout }
  }

  /** @param {IconBuild} built @param {string} target */
  static output(built, target) {
    let folder = built.bundle
    while (path.basename(folder) !== target && path.dirname(folder) !== folder) folder = path.dirname(folder)
    return path.basename(folder) === target ? folder : path.join(built.project, 'dist', target)
  }

  /** @param {string} folder @param {(file: string) => boolean} wanted @returns {string[]} */
  static find(folder, wanted) {
    if (!existsSync(folder)) return []
    return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
      const file = path.join(folder, entry.name)
      if (entry.isDirectory() && !entry.isSymbolicLink()) return [...(wanted(file) ? [file] : []), ...this.find(file, wanted)]
      return wanted(file) ? [file] : []
    })
  }

  /** @param {string} label @param {number[]} found @param {number[]} wanted */
  static expectColor(label, found, wanted) {
    assert.ok(IconPng.near(found, wanted), `${label}: found rgb(${found.join(', ')}), wanted rgb(${wanted.join(', ')}) within ${this.tolerance}`)
    return `${label} is rgb(${found.join(', ')})`
  }

  /** @param {string} label @param {string} file @param {{ color: number[], edge?: number }} icon */
  static expectFile(label, file, icon) {
    const picture = IconPng.decode(readFileSync(file))
    if (icon.edge) assert.deepEqual([picture.width, picture.height], [icon.edge, icon.edge], `${label}: ${file} size`)
    return this.expectColor(label, IconPng.center(picture), icon.color)
  }

  /** @param {string} file @param {string[]} events @param {number} seconds */
  static async waitForEvents(file, events, seconds = 60) {
    const deadline = Date.now() + seconds * 1000
    let recorded = []
    while (Date.now() < deadline) {
      recorded = existsSync(file) ? readFileSync(file, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line).event) : []
      if (events.every((event) => recorded.includes(event))) return recorded
      await sleep(500)
    }
    assert.fail(`${file} did not record ${events.join(', ')}; it holds ${recorded.join(', ') || 'nothing'}`)
  }

  /** @param {string} target @param {string} check @param {() => Promise<string | string[]>} body */
  static async step(target, check, body) {
    try {
      const lines = [await body()].flat()
      console.log(`PASS ${target} ${check}${lines.map((line) => `\n  ${line}`).join('')}`)
      this.results.push({ target, check, status: 'pass', lines })
    } catch (error) {
      const message = String(error?.message ?? error)
      console.log(`FAIL ${target} ${check}\n  ${message.slice(-4000).replaceAll('\n', '\n  ')}`)
      this.results.push({ target, check, status: 'fail', error: message.slice(-4000) })
    }
  }

  static async main() {
    const targets = this.option('targets', this.defaults[process.platform] ?? 'linux').split(',').filter(Boolean)
    const hosts = { macos: MacOsIcons, ios: IosIcons, android: AndroidIcons, windows: WindowsIcons, linux: LinuxIcons }
    console.log(`icon gate: ${targets.join(', ')} with ${this.binary}${this.screen ? ', on screen' : ''}${this.release ? ', release packages' : ''}; work folder ${this.root}`)
    try {
      for (const target of targets) {
        assert.ok(hosts[target], `unknown target ${target}`)
        let built
        await this.step(target, 'build names the manifest icon', async () => {
          built = await hosts[target].build()
          return `built ${built.bundle} with no icon note`
        })
        if (!built) continue
        await this.step(target, 'artifact holds the manifest icon', () => hosts[target].artifact(built))
        if (this.screen) await this.step(target, 'system shows the manifest icon', () => hosts[target].screen(built))
      }
    } finally {
      if (!this.keep) rmSync(this.root, { recursive: true, force: true })
    }
    const receipt = process.env.TAC_ICON_GATE_RECEIPT
    if (receipt) writeFileSync(receipt, JSON.stringify({ targets, screen: this.screen, release: this.release, results: this.results }, null, 2))
    const failed = this.results.filter((result) => result.status === 'fail')
    console.log(`icon gate: ${this.results.length - failed.length} passed, ${failed.length} failed`)
    process.exitCode = failed.length ? 1 : 0
  }
}

class MacOsIcons {
  static probe = ''

  static build() {
    return IconGate.bundle('macos')
  }

  /** @param {IconBuild} built */
  static async artifact(built) {
    const plist = path.join(built.bundle, 'Contents/Info.plist')
    assert.equal((await IconGate.run('/usr/bin/plutil', ['-extract', 'CFBundleIconFile', 'raw', plist])).trim(), 'AppIcon')
    const iconset = path.join(built.project, 'AppIcon.iconset')
    await IconGate.run('/usr/bin/iconutil', ['-c', 'iconset', path.join(built.bundle, 'Contents/Resources/AppIcon.icns'), '-o', iconset])
    return [
      'Info.plist CFBundleIconFile is AppIcon',
      IconGate.expectFile('AppIcon.icns 1024 px', path.join(iconset, 'icon_512x512@2x.png'), { ...IconFixture.large, edge: 1024 }),
      IconGate.expectFile('AppIcon.icns 32 px', path.join(iconset, 'icon_16x16@2x.png'), { ...IconFixture.large, edge: 32 }),
    ]
  }

  /** @param {IconBuild} built */
  static async screen(built) {
    if (!this.probe) {
      this.probe = path.join(IconGate.root, 'MacOsIconCheck')
      await IconGate.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-warnings-as-errors', '-O', '-framework', 'AppKit', path.join(IconGate.repository, 'scripts/native/MacOsIconCheck.swift'), '-o', this.probe])
    }
    const log = path.join(homedir(), 'Library/Logs/Tachyon', `${IconFixture.id}.jsonl`)
    rmSync(log, { force: true })
    await IconGate.run('/usr/bin/open', ['-n', built.bundle])
    try {
      await IconGate.waitForEvents(log, ['controller.created', 'tray.icon.loaded'])
      await sleep(3000)
      const report = JSON.parse(await IconGate.run(this.probe, [IconFixture.id, built.bundle]))
      const shot = path.join(built.project, 'screen.png')
      await IconGate.run('/usr/sbin/screencapture', ['-x', '-t', 'png', shot])
      const picture = IconPng.decode(readFileSync(shot))
      const menuBar = Math.round(picture.height * 0.04)
      const dock = IconPng.count(picture, IconFixture.large.color, { top: menuBar })
      const tray = IconPng.count(picture, IconFixture.tray.color, { bottom: menuBar })
      assert.ok(dock >= 400, `the screen shows ${dock} pixels in the app icon's color outside the menu bar; is the Dock hidden?`)
      assert.ok(tray >= 40, `the menu bar shows ${tray} pixels in the tray icon's color`)
      return [
        IconGate.expectColor('the Dock icon of the running app (NSRunningApplication.icon)', report.running, IconFixture.large.color),
        IconGate.expectColor('the Finder icon of the app (NSWorkspace.icon)', report.file, IconFixture.large.color),
        `the screen shows ${dock} pixels of the app icon and the menu bar ${tray} pixels of the tray icon`,
        'the host recorded tray.icon.loaded',
      ]
    } finally {
      await SupervisedCommand.run('/usr/bin/osascript', ['-e', `tell application id "${IconFixture.id}" to quit`], { timeoutMilliseconds: 30000 })
    }
  }
}

class IosIcons {
  static deviceSet = process.env.TAC_IOS_DEVICE_SET ?? ''
  static device = process.env.TAC_IOS_DEVICE ?? ''

  static build() {
    return IconGate.bundle('ios')
  }

  /** @param {IconBuild} built */
  static async artifact(built) {
    const plist = path.join(built.bundle, 'Info.plist')
    const first = await IconGate.run('/usr/bin/plutil', ['-extract', 'CFBundleIcons.CFBundlePrimaryIcon.CFBundleIconFiles.0', 'raw', plist])
    assert.equal(first.trim(), 'AppIcon60x60@2x.png')
    return [
      'Info.plist names AppIcon60x60@2x.png first',
      IconGate.expectFile('AppIcon60x60@2x.png', path.join(built.bundle, 'AppIcon60x60@2x.png'), { ...IconFixture.large, edge: 120 }),
      IconGate.expectFile('AppIcon60x60@3x.png', path.join(built.bundle, 'AppIcon60x60@3x.png'), { ...IconFixture.large, edge: 180 }),
    ]
  }

  /** @param {string[]} args */
  static simctl(...args) {
    const set = this.deviceSet ? ['--set', this.deviceSet] : []
    return IconGate.run('/usr/bin/xcrun', ['simctl', ...set, ...args], { timeoutMilliseconds: 900000 })
  }

  static async boot() {
    if (this.device) return { created: false }
    this.deviceSet = this.deviceSet || path.join(IconGate.root, 'simulator-devices')
    mkdirSync(this.deviceSet, { recursive: true })
    const runtimes = JSON.parse(await this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, `iOS ${runtime.version} has no iPhone device type`)
    this.device = (await this.simctl('create', 'TachyonIconGate', phone.identifier, runtime.identifier)).trim()
    await this.simctl('boot', this.device)
    await this.simctl('bootstatus', this.device, '-b')
    return { created: true }
  }

  /** @param {IconBuild} built */
  static async screen(built) {
    const { created } = await this.boot()
    try {
      await this.simctl('install', this.device, built.bundle)
      await sleep(5000)
      const shot = path.join(built.project, 'home.png')
      await this.simctl('io', this.device, 'screenshot', shot)
      const found = IconPng.count(IconPng.decode(readFileSync(shot)), IconFixture.large.color)
      assert.ok(found >= 400, `the home screen shows ${found} pixels in the app icon's color; the icon may be on another home screen page`)
      return `the simulator home screen shows ${found} pixels of the app icon`
    } finally {
      await SupervisedCommand.run('/usr/bin/xcrun', ['simctl', ...(this.deviceSet ? ['--set', this.deviceSet] : []), 'uninstall', this.device, IconFixture.id], { timeoutMilliseconds: 60000 })
      if (created) {
        for (const action of ['shutdown', 'delete']) await SupervisedCommand.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, action, this.device], { timeoutMilliseconds: 120000 })
        this.device = ''
      }
    }
  }
}

class AndroidIcons {
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? ''
  static adb = path.join(AndroidIcons.sdk, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')

  static build() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required')
    return IconGate.bundle('android')
  }

  static aapt() {
    const tools = path.join(this.sdk, 'build-tools')
    const version = readdirSync(tools).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }))[0]
    return path.join(tools, version, process.platform === 'win32' ? 'aapt2.exe' : 'aapt2')
  }

  /** @param {IconBuild} built */
  static async badging(built) {
    const text = await IconGate.run(this.aapt(), ['dump', 'badging', built.bundle])
    const packageName = text.match(/^package: name='([^']+)'/m)?.[1]
    const icons = [...text.matchAll(/^application-icon-(\d+):'([^']+)'/gm)].map((match) => ({ density: Number(match[1]), file: match[2] }))
    assert.ok(packageName && icons.length, `aapt2 badging names no package or icon:\n${text}`)
    return { packageName, icon: icons.sort((a, b) => b.density - a.density)[0].file }
  }

  /** @param {IconBuild} built */
  static async artifact(built) {
    const { icon } = await this.badging(built)
    const file = path.join(built.project, 'launcher.png')
    writeFileSync(file, await IconGate.bytes('unzip', ['-p', built.bundle, icon]))
    return [`aapt2 badging names ${icon}`, IconGate.expectFile(`${icon} in the APK`, file, IconFixture.large)]
  }

  /** @param {string[]} args */
  static shell(...args) {
    return IconGate.run(this.adb, ['shell', ...args], { timeoutMilliseconds: 120000 })
  }

  /** @param {IconBuild} built */
  static async screen(built) {
    const { packageName } = await this.badging(built)
    await IconGate.run(this.adb, ['wait-for-device'], { timeoutMilliseconds: 300000 })
    await SupervisedCommand.run(this.adb, ['uninstall', packageName], { timeoutMilliseconds: 60000 })
    await IconGate.run(this.adb, ['install', '-r', built.bundle], { timeoutMilliseconds: 300000 })
    try {
      const [width, height] = (await this.shell('wm', 'size')).match(/(\d+)x(\d+)\s*$/).slice(1).map(Number)
      await this.shell('am', 'start', '-a', 'android.intent.action.MAIN', '-c', 'android.intent.category.HOME')
      await sleep(2000)
      await this.shell('input', 'swipe', String(width / 2), String(Math.round(height * 0.85)), String(width / 2), String(Math.round(height * 0.2)), '400')
      await sleep(2500)
      const picture = IconPng.decode(await IconGate.bytes(this.adb, ['exec-out', 'screencap', '-p']))
      const found = IconPng.count(picture, IconFixture.large.color)
      assert.ok(found >= 400, `the app drawer shows ${found} pixels in the launcher icon's color`)
      return `the launcher's app drawer shows ${found} pixels of the app icon`
    } finally {
      await this.shell('am', 'start', '-a', 'android.intent.action.MAIN', '-c', 'android.intent.category.HOME')
      await SupervisedCommand.run(this.adb, ['uninstall', packageName], { timeoutMilliseconds: 60000 })
    }
  }
}

class WindowsIcons {
  static probe = String.raw`
param([string]$Mode, [string]$Target)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class TachyonIconProbe {
  [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr window, int message, IntPtr wide, IntPtr low);
  [DllImport("user32.dll", EntryPoint = "GetClassLongPtrW")] public static extern IntPtr GetClassLongPtr(IntPtr window, int index);
}
"@
function Center([System.Drawing.Bitmap]$bitmap) {
  $color = $bitmap.GetPixel([int]($bitmap.Width / 2), [int]($bitmap.Height / 2))
  return @($color.R, $color.G, $color.B)
}
if ($Mode -eq 'file') {
  $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($Target)
  ConvertTo-Json -Compress @{ file = (Center $icon.ToBitmap()) }
  exit 0
}
$process = Get-Process -Id ([int]$Target)
$window = $process.MainWindowHandle
$handle = [TachyonIconProbe]::GetClassLongPtr($window, -14)
$small = [TachyonIconProbe]::GetClassLongPtr($window, -34)
if ($handle -eq [IntPtr]::Zero) { $handle = [TachyonIconProbe]::SendMessageW($window, 0x7F, [IntPtr]1, [IntPtr]::Zero) }
if ($handle -eq [IntPtr]::Zero) { throw 'the window has no icon' }
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$working = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$top = [Math]::Max($working.Bottom, $bounds.Bottom - 120)
$shot = New-Object System.Drawing.Bitmap $bounds.Width, ($bounds.Bottom - $top)
$graphics = [System.Drawing.Graphics]::FromImage($shot)
$graphics.CopyFromScreen($bounds.Left, $top, 0, 0, $shot.Size)
$taskbar = 0
for ($y = 0; $y -lt $shot.Height; $y++) { for ($x = 0; $x -lt $shot.Width; $x++) {
  $pixel = $shot.GetPixel($x, $y)
  if ([Math]::Abs($pixel.R - 0) -le 40 -and [Math]::Abs($pixel.G - 200) -le 40 -and [Math]::Abs($pixel.B - 120) -le 40) { $taskbar++ }
} }
ConvertTo-Json -Compress @{ window = (Center ([System.Drawing.Icon]::FromHandle($handle)).ToBitmap()); small = ($small -ne [IntPtr]::Zero); taskbar = $taskbar }
`

  static build() {
    return IconGate.bundle('windows', IconGate.release ? ['--release'] : [])
  }

  /** @param {IconBuild} built */
  static executable(built) {
    const [exe] = IconGate.find(IconGate.output(built, 'windows'), (file) => path.basename(file) === `${IconFixture.executable}.exe`)
    assert.ok(exe, `no ${IconFixture.executable}.exe under ${IconGate.output(built, 'windows')}`)
    return exe
  }

  /** @param {string} mode @param {string} target */
  static async run(mode, target) {
    const script = path.join(IconGate.root, 'windows-icon-probe.ps1')
    writeFileSync(script, this.probe)
    const output = await IconGate.run('powershell.exe', ['-NoProfile', '-NonInteractive', '-File', script, mode, target], { timeoutMilliseconds: 300000 })
    return JSON.parse(output.trim().split(/\r?\n/).at(-1))
  }

  /** @param {IconBuild} built */
  static async artifact(built) {
    const exe = this.executable(built)
    const lines = [IconGate.expectColor(`the icon Explorer shows for ${path.basename(exe)}`, (await this.run('file', exe)).file, IconFixture.small.color)]
    const [msix] = IconGate.find(IconGate.output(built, 'windows'), (file) => file.endsWith('.msix'))
    if (!msix) return [...lines, 'no MSIX package (pass --release with TAC_WINDOWS_CERTIFICATE to check its tiles)']
    const unpacked = path.join(built.project, 'msix')
    copyFileSync(msix, `${unpacked}.zip`)
    await IconGate.run('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `Expand-Archive -LiteralPath '${unpacked}.zip' -DestinationPath '${unpacked}' -Force`])
    for (const tile of ['Square44x44Logo.png', 'Square150x150Logo.png', 'StoreLogo.png']) {
      lines.push(IconGate.expectFile(`MSIX ${tile}`, path.join(unpacked, 'Assets', tile), IconFixture.small))
    }
    return lines
  }

  /** @param {IconBuild} built */
  static async screen(built) {
    const log = path.join(process.env.LOCALAPPDATA ?? '', 'Tachyon', `${IconFixture.id}.jsonl`)
    rmSync(log, { force: true })
    const app = spawn(this.executable(built), [], { detached: true, stdio: 'ignore' })
    try {
      await IconGate.waitForEvents(log, ['controller.created', 'icon.loaded', 'tray.icon.loaded'])
      await sleep(3000)
      const report = await this.run('window', String(app.pid))
      assert.ok(report.small, 'the window class has no small icon')
      assert.ok(report.taskbar >= 40, `the taskbar shows ${report.taskbar} pixels in the app icon's color`)
      return [
        IconGate.expectColor('the window icon (title bar, Alt-Tab)', report.window, IconFixture.small.color),
        `the taskbar shows ${report.taskbar} pixels of the app icon`,
        'the host recorded icon.loaded and tray.icon.loaded',
      ]
    } finally {
      await SupervisedCommand.run('taskkill.exe', ['/PID', String(app.pid), '/T', '/F'], { timeoutMilliseconds: 30000 })
    }
  }
}

class LinuxIcons {
  static build() {
    return IconGate.bundle('linux', IconGate.release ? ['--release'] : [])
  }

  /** @param {IconBuild} built */
  static folder(built) {
    const [folder] = IconGate.find(IconGate.output(built, 'linux'), (file) => path.basename(file) === IconFixture.executable && statSync(file).isDirectory() && existsSync(path.join(file, 'bin')))
    assert.ok(folder, `no ${IconFixture.executable} app folder under ${IconGate.output(built, 'linux')}`)
    return folder
  }

  /** @param {IconBuild} built */
  static async artifact(built) {
    const folder = this.folder(built)
    const entry = readFileSync(path.join(folder, `${IconFixture.id}.desktop`), 'utf8')
    assert.match(entry, new RegExp(`^Icon=${IconFixture.themedName.replaceAll('.', '\\.')}$`, 'm'))
    const lines = [
      `the desktop entry names Icon=${IconFixture.themedName}`,
      IconGate.expectFile('the hicolor theme folder in the app folder', path.join(folder, 'share/icons/hicolor/256x256/apps', `${IconFixture.themedName}.png`), IconFixture.large),
    ]
    if (!IconGate.release) return lines
    const [deb] = IconGate.find(IconGate.output(built, 'linux'), (file) => file.endsWith('.deb'))
    const [appImage] = IconGate.find(IconGate.output(built, 'linux'), (file) => file.endsWith('.AppImage'))
    assert.ok(deb && appImage, 'a --release build needs a .deb and an AppImage')
    for (const icon of [IconFixture.large, IconFixture.small]) {
      const member = `./usr/share/icons/hicolor/${icon.edge}x${icon.edge}/apps/${IconFixture.themedName}.png`
      const file = path.join(built.project, `hicolor-${icon.edge}.png`)
      writeFileSync(file, await IconGate.bytes('bash', ['-c', 'dpkg-deb --fsys-tarfile "$1" | tar -xO "$2"', 'deb', deb, member]))
      lines.push(IconGate.expectFile(`.deb ${member}`, file, icon))
    }
    const extract = path.join(built.project, 'appimage')
    mkdirSync(extract, { recursive: true })
    await IconGate.run(appImage, ['--appimage-extract'], { cwd: extract })
    lines.push(IconGate.expectFile('AppImage .DirIcon', path.join(extract, 'squashfs-root/.DirIcon'), IconFixture.large))
    return lines
  }

  /** @param {IconBuild} built */
  static async screen(built) {
    const folder = this.folder(built)
    const state = path.join(built.project, 'state')
    const script = `set -euo pipefail
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
for _ in $(seq 1 120); do
  if grep -q '"icon.loaded"' "$LOG" 2>/dev/null && grep -q '"tray.icon' "$LOG" 2>/dev/null; then break; fi
  sleep 0.5
done
sleep 2
cat "$LOG"
echo "ICON $(xprop -name "$NAME" -notype -f _NET_WM_ICON 32c _NET_WM_ICON)"`
    const env = {
      ...process.env,
      APP: path.join(folder, 'bin', IconFixture.executable),
      LOG: path.join(state, 'tachyon', `${IconFixture.id}.jsonl`),
      NAME: IconFixture.name,
      XDG_STATE_HOME: state,
      XDG_DATA_HOME: path.join(built.project, 'data'),
      XDG_CACHE_HOME: path.join(built.project, 'cache'),
      GDK_BACKEND: 'x11',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const live = process.env.TAC_LINUX_DISPLAY === 'desktop'
    const command = live ? ['bash', ['-c', script]] : ['xvfb-run', ['--auto-servernum', '--server-args=-screen 0 1280x900x24', 'dbus-run-session', '--', 'bash', '-c', script]]
    const output = await IconGate.run(command[0], command[1], { env, timeoutMilliseconds: 300000 })
    const events = output.split('\n').filter((line) => line.startsWith('{')).map((line) => JSON.parse(line).event)
    assert.ok(events.includes('icon.loaded'), `the host recorded ${events.join(', ')}, not icon.loaded`)
    assert.ok(events.includes('tray.icon.loaded'), `the host recorded ${events.join(', ')}, not tray.icon.loaded`)
    const numbers = (output.match(/^ICON .*?=\s*(.*)$/m)?.[1] ?? '').split(',').map((value) => Number(value.trim())).filter(Number.isFinite)
    const [width, height] = numbers
    assert.ok(width > 0 && height > 0 && numbers.length >= 2 + width * height, `the window has no _NET_WM_ICON:\n${output.slice(-2000)}`)
    const argb = numbers[2 + Math.floor(height / 2) * width + Math.floor(width / 2)]
    const center = [(argb >>> 16) & 0xff, (argb >>> 8) & 0xff, argb & 0xff]
    return [
      IconGate.expectColor(`the X11 window icon (_NET_WM_ICON, ${width} by ${height})`, center, IconFixture.large.color),
      'the host recorded icon.loaded and tray.icon.loaded',
    ]
  }
}

await IconGate.main()

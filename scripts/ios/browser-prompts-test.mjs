#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

class IosBrowserPromptsTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-browser-prompts-'))
  static project = path.join(IosBrowserPromptsTest.root, 'project')
  static deviceSet = path.join(IosBrowserPromptsTest.root, 'simulator-devices')
  static evidence = process.env.TAC_IOS_BROWSER_PROMPTS_EVIDENCE ?? path.join(IosBrowserPromptsTest.repository, 'target/ios-browser-prompts-evidence')
  static tapper = path.join(IosBrowserPromptsTest.root, 'simulator-tap')
  static reader = path.join(IosBrowserPromptsTest.root, 'screenshot-text')
  static application = 'PromptCheck'
  static applicationId = 'dev.tachyon.ios-browser-prompts'
  static locationReason = 'Shows stores near you in the RM2 check'
  static notificationReason = 'Order updates'
  static place = { latitude: 51.5007, longitude: -0.1246 }
  static waitMilliseconds = 30000
  static pollMilliseconds = 400
  static device = ''

  static files = {
    'client/pages/tac.html': '<prompt-panel />\n',
    'client/components/prompt/panel/tac.html': `<main>
<h1>Prompt check</h1>
<button type="button" on:click="promptCheck.locate()">Find my place</button>
<button type="button" on:click="promptCheck.ask()">Ask for notifications</button>
<button type="button" on:click="promptCheck.notify()">Show a notification</button>
<button type="button" on:click="promptCheck.open()">Share a link</button>
<p>Place {promptCheck.place}</p>
<p>Permission {promptCheck.permission}</p>
<p>Shared {promptCheck.shared}</p>
<p>Problem {promptCheck.problem}</p>
</main>
`,
    'client/components/prompt/panel/tac.css': 'button { display: block; font-size: 24px; margin: 12px 0; padding: 12px; }\n',
    'client/services/PromptCheckService.js': `@Service
export default class PromptCheckService {
  /** @type {TacShare} */
  @Needs
  share;
  /** @type {TacNotifications} */
  @Needs
  notifications;
  /** @type {TacGeolocation} */
  @Needs
  geolocation;
  place = 'none';
  permission = 'none';
  shared = 'no';
  problem = 'none';

  locate() {
    this.geolocation.currentPosition();
  }

  ask() {
    this.notifications.requestPermission();
  }

  open() {
    this.share.open('Tachyon RM2', 'Shared by Tachyon', 'https://example.com/tachyon-rm2-share');
  }

  notify() {
    try {
      this.notifications.show('Order 42', 'Your order is ready');
      void this.note('notified');
    } catch (refusal) {
      this.problem = refusal.message;
      void this.note(\`failed:\${refusal.message}\`);
    }
  }

  /** @param {PositionFound} found */
  @OnEvent
  found(found) {
    this.place = \`\${found.latitude.toFixed(4)}, \${found.longitude.toFixed(4)}\`;
    void this.note(\`place:\${this.place}\`);
  }

  /** @param {NotificationPermissionChanged} changed */
  @OnEvent
  changed(changed) {
    this.permission = changed.permission;
    void this.note(\`permission:\${changed.permission}\`);
  }

  /** @param {ShareFinished} finished */
  @OnEvent
  finished(finished) {
    this.shared = 'finished';
    void this.note('shared');
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    this.problem = failed.message;
    void this.note(\`failed:\${failed.message}\`);
  }
}
`,
    'client/services/PromptCheckService.swift': `import Foundation
@Service
final class PromptCheckService {
    func note(_ line: String) -> Int {
        let file = documents().appendingPathComponent("notes.log")
        let data = Data((String(line.prefix(300)) + "\\n").utf8)
        guard let handle = try? FileHandle(forWritingTo: file) else {
            try? data.write(to: file)
            return 0
        }
        handle.seekToEndOfFile()
        handle.write(data)
        handle.closeFile()
        return 0
    }

    private func documents() -> URL {
        FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
    }
}
`,
  }

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

  static write(name, text) {
    const target = path.join(this.project, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static prepareTools() {
    this.run('/usr/bin/xcrun', ['clang', '-fobjc-arc', '-framework', 'Foundation', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/ios/simulator-tap.m'), '-o', this.tapper])
    this.run('/usr/bin/xcrun', ['swiftc', '-parse-as-library', '-O', path.join(this.repository, 'scripts/ios/screenshot-text.swift'), '-o', this.reader], 600000)
  }

  static prepare() {
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    this.write('client/manifest.json', JSON.stringify({
      name: this.application,
      start_url: '/',
      tac: {
        ...checks,
        browserApis: ['share', 'notifications', 'geolocation'],
        application: {
          id: this.applicationId,
          version: '0.0.1',
          permissions: { geolocation: this.locationReason, notifications: this.notificationReason },
        },
      },
    }))
    this.run(this.binary, ['build', this.project, '--target', 'ios'], 900000)
    const plist = path.join(this.project, 'dist/ios', `${this.application}.app`, 'Info.plist')
    const reason = this.run('/usr/bin/plutil', ['-extract', 'NSLocationWhenInUseUsageDescription', 'raw', plist])
    assert.equal(reason, this.locationReason, 'Info.plist carries the manifest reason')
    return reason
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
    this.device = this.simctl('create', `BrowserPrompts-${randomUUID()}`, phone.identifier, runtime.identifier)
    console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    this.simctl('boot', this.device)
    this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'bootstatus', this.device, '-b'])
    return { runtime: runtime.version, device: phone.name }
  }

  static shutdownSimulator() {
    if (!this.device) return
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
  }

  static documents() {
    return path.join(this.simctl('get_app_container', this.device, this.applicationId, 'data'), 'Documents')
  }

  static notes() {
    const file = path.join(this.documents(), 'notes.log')
    return existsSync(file) ? readFileSync(file, 'utf8').trim().split('\n').filter(Boolean) : []
  }

  /** @param {string} name */
  static read(name) {
    const file = path.join(this.root, `${name}.png`)
    this.simctl('io', this.device, 'screenshot', file)
    return { file, lines: JSON.parse(this.run(this.reader, [file])) }
  }

  /** @param {{ lines: Array<{ text: string }> }} shot */
  static joined(shot) {
    return shot.lines.map(({ text }) => text).join(' ').replace(/\s+/g, ' ')
  }

  /** @param {string} name @param {(text: string) => boolean} check */
  static async screenWith(name, check) {
    const deadline = Date.now() + this.waitMilliseconds
    let shot = this.read(name)
    while (!check(this.joined(shot)) && Date.now() < deadline) {
      await sleep(this.pollMilliseconds)
      shot = this.read(name)
    }
    copyFileSync(shot.file, path.join(this.evidence, `${name}.png`))
    return { screenshot: path.join(this.evidence, `${name}.png`), text: shot.lines.map(({ text }) => text), found: check(this.joined(shot)) }
  }

  /** @param {string} label */
  static tap(label) {
    const shot = this.read('before-tap')
    const target = shot.lines.find(({ text }) => text.trim() === label) ?? shot.lines.find(({ text }) => text.includes(label))
    assert.ok(target, `no "${label}" on screen: ${shot.lines.map(({ text }) => text).join(' | ')}`)
    this.run(this.tapper, [this.deviceSet, this.device, String(target.x), String(target.y)])
    return { label: target.text, x: Number(target.x.toFixed(3)), y: Number(target.y.toFixed(3)) }
  }

  /** @param {string} prefix */
  static async noteStarting(prefix, before) {
    const deadline = Date.now() + this.waitMilliseconds
    while (Date.now() < deadline) {
      const found = this.notes().slice(before).find((line) => line.startsWith(prefix) || line.startsWith('failed:'))
      if (found) return found
      await sleep(this.pollMilliseconds)
    }
    return null
  }

  /** @param {string} label */
  static press(label) {
    const before = this.notes().length
    const tapped = this.tap(label)
    return { before, tapped }
  }

  static async location() {
    const { before, tapped: button } = this.press('Find my place')
    const prompt = await this.screenWith('location-prompt', (text) => text.includes(this.locationReason))
    const tapped = prompt.found ? this.tap('Allow While Using App') : null
    const first = await this.noteStarting('place:', before)
    const note = first?.startsWith('place:') ? first : await this.noteStarting('place:', this.press('Find my place').before)
    const after = await this.screenWith('location-found', (text) => text.includes('Place 51.'))
    return { button, prompt, tapped, first, note, after: after.found }
  }

  static async notificationPermission() {
    const { before, tapped: button } = this.press('Ask for notifications')
    const prompt = await this.screenWith('notification-prompt', (text) => /Would Like to Send You Notifications/i.test(text))
    const tapped = prompt.found ? this.tap('Allow') : null
    const note = await this.noteStarting('permission:', before)
    return { button, prompt, tapped, note }
  }

  static async banner() {
    const { before, tapped: button } = this.press('Show a notification')
    const shown = await this.screenWith('notification-banner', (text) => text.includes('Order 42') && text.includes('Your order is ready'))
    const note = await this.noteStarting('notified', before)
    return { button, shown, note }
  }

  static async shareSheet() {
    const { before, tapped: button } = this.press('Share a link')
    const sheet = await this.screenWith('share-sheet', (text) => /Copy|AirDrop|Messages/.test(text) && /Tachyon RM2|example\.com/.test(text))
    this.run(this.tapper, [this.deviceSet, this.device, '0.5', '0.08'])
    const note = await this.noteStarting('shared', before)
    const closed = await this.screenWith('share-closed', (text) => text.includes('Prompt check') && text.includes('Share a link'))
    return { button, sheet, note, closed: closed.found }
  }

  static async main() {
    mkdirSync(this.evidence, { recursive: true })
    const report = {}
    try {
      this.prepareTools()
      report.infoPlistReason = this.prepare()
      Object.assign(report, this.bootSimulator())
      this.simctl('location', this.device, 'set', `${this.place.latitude},${this.place.longitude}`)
      this.simctl('install', this.device, path.join(this.project, 'dist/ios', `${this.application}.app`))
      this.simctl('launch', this.device, this.applicationId)
      await this.screenWith('started', (text) => text.includes('Find my place'))
      report.location = await this.location()
      report.notificationPermission = await this.notificationPermission()
      report.banner = await this.banner()
      report.share = await this.shareSheet()
      report.notes = this.notes()
    } finally {
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
      this.shutdownSimulator()
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    assert.ok(report.location.prompt.found, `the location prompt did not show the manifest reason: ${report.location.prompt.text.join(' | ')}`)
    assert.equal(report.location.note, `place:${this.place.latitude.toFixed(4)}, ${this.place.longitude.toFixed(4)}`, 'the page got the simulated position')
    assert.ok(report.notificationPermission.prompt.found, 'the notification permission prompt did not show')
    assert.equal(report.notificationPermission.note, 'permission:granted', 'the page heard the granted permission')
    assert.ok(report.banner.shown.found, 'the notification banner did not show')
    assert.ok(report.share.sheet.found, 'the share sheet did not show')
    console.log(`PASS: on the iOS ${report.runtime} simulator (${report.device}) the location prompt showed "${this.locationReason}", the page got the position, the notification prompt was answered, a banner showed, and the share sheet opened. Report: ${path.join(this.evidence, 'report.json')}`)
  }
}

await IosBrowserPromptsTest.main()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { MobileAccessibilityFixture } from '../native/mobile-accessibility-fixture.mjs'

class AndroidAccessibilityTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
  static adb = path.join(AndroidAccessibilityTest.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'android-accessibility-'))
  static evidence = process.env.TAC_ANDROID_ACCESSIBILITY_EVIDENCE ?? path.join(AndroidAccessibilityTest.repository, 'target/android-accessibility-evidence')
  static applicationId = 'dev.tachyon.accessibility_audit'
  static dumpFile = '/sdcard/tachyon-accessibility.xml'
  static webViewClass = 'android.webkit.WebView'
  static editableClass = 'android.widget.EditText'
  static buttonClass = 'android.widget.Button'
  static minimumTarget = 48
  static redundantWords = ['button', 'link', 'image', 'text field']
  static waitAttempts = 60
  static pauseMilliseconds = 500
  static dumps = 0

  static run(program, args, options = {}) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout: 900000, maxBuffer: 64 * 1024 * 1024, ...options })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stderr}${result.stdout}`)
    return result.stdout.trim()
  }

  static adbShell(...args) {
    return this.run(this.adb, ['shell', ...args])
  }

  static pause() {
    return new Promise((resolve) => setTimeout(resolve, this.pauseMilliseconds))
  }

  static attributes(source) {
    const values = {}
    for (const match of source.matchAll(/([\w-]+)="([^"]*)"/g)) {
      values[match[1]] = match[2].replaceAll('&quot;', '"').replaceAll('&apos;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&')
    }
    const bounds = values.bounds?.match(/\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]/)?.slice(1).map(Number) ?? [0, 0, 0, 0]
    return { ...values, bounds, children: [], name: [values.text, values['content-desc'], values.hint].filter(Boolean).join(' ') }
  }

  static parse(xml) {
    const root = { children: [], bounds: [0, 0, 0, 0], name: '' }
    const stack = [root]
    for (const match of xml.matchAll(/<node\b([^>]*?)(\/?)>|<\/node>/g)) {
      if (match[0] === '</node>') { stack.pop(); continue }
      const node = this.attributes(match[1])
      stack.at(-1).children.push(node)
      if (match[2] !== '/') stack.push(node)
    }
    return root
  }

  static flatten(node) {
    return [node, ...node.children.flatMap((child) => this.flatten(child))]
  }

  static screen() {
    const dump = spawnSync(this.adb, ['shell', 'uiautomator', 'dump', this.dumpFile], { encoding: 'utf8', timeout: 60000 })
    if (!`${dump.stdout}${dump.stderr}`.includes('dumped to')) return { xml: '', nodes: [] }
    const xml = this.adbShell('cat', this.dumpFile)
    const tree = this.parse(xml)
    const web = this.flatten(tree).find((node) => node.class === this.webViewClass && node.package === this.applicationId)
    return { xml, web, nodes: web ? this.flatten(web).slice(1) : [] }
  }

  static async waitFor(predicate, message) {
    let screen = { xml: '', nodes: [] }
    for (let attempt = 0; attempt < this.waitAttempts; attempt++) {
      screen = this.screen()
      if (screen.web && predicate(screen.nodes)) {
        this.dumps += 1
        writeFileSync(path.join(this.evidence, `dump-${this.dumps}.xml`), screen.xml)
        return screen
      }
      await this.pause()
    }
    writeFileSync(path.join(this.evidence, 'dump-failed.xml'), screen.xml)
    assert.fail(`${message}: ${screen.xml.slice(0, 8000)}`)
  }

  static named(nodes, name) {
    return nodes.find((node) => node.text === name || node['content-desc'] === name || node.hint === name)
  }

  static center(node) {
    const [left, top, right, bottom] = node.bounds
    return [String(Math.round((left + right) / 2)), String(Math.round((top + bottom) / 2))]
  }

  static density() {
    const output = this.adbShell('wm', 'density')
    const values = [...output.matchAll(/density: (\d+)/g)].map((match) => Number(match[1]))
    assert.ok(values.length, `wm density gave no density: ${output}`)
    return values.at(-1) / 160
  }

  static speakable(node) {
    return Boolean(node.name) || node.children.some((child) => child.clickable !== 'true' && child.focusable !== 'true' && this.speakable(child))
  }

  static audit(nodes, density) {
    const findings = []
    const label = (node) => `${node.class} '${node.name}'`
    const actionable = nodes.filter((node) => node.clickable === 'true' || node.focusable === 'true')
    for (const node of actionable) {
      if (!this.speakable(node)) findings.push({ check: 'SpeakableTextPresentCheck', element: label(node), detail: 'a focusable or clickable node has no text, description or hint' })
    }
    for (const node of nodes.filter((item) => item.clickable === 'true')) {
      const [left, top, right, bottom] = node.bounds
      const width = (right - left) / density
      const height = (bottom - top) / density
      if (width < this.minimumTarget || height < this.minimumTarget) findings.push({ check: 'TouchTargetSizeCheck', element: label(node), detail: `${width.toFixed(0)}x${height.toFixed(0)} dp is under ${this.minimumTarget}x${this.minimumTarget} dp` })
    }
    const clickable = nodes.filter((node) => node.clickable === 'true')
    for (const [index, node] of clickable.entries()) {
      const twin = clickable.slice(index + 1).find((other) => other.bounds.join() === node.bounds.join())
      if (twin) findings.push({ check: 'DuplicateClickableBoundsCheck', element: label(node), detail: `shares its bounds with ${label(twin)}` })
    }
    for (const node of nodes.filter((item) => item.class === this.editableClass && item['content-desc'])) {
      findings.push({ check: 'EditableContentDescCheck', element: label(node), detail: 'an editable field has a content description instead of a label' })
    }
    for (const node of actionable) {
      const description = (node['content-desc'] ?? '').toLowerCase()
      const word = this.redundantWords.find((item) => description.includes(item))
      if (word) findings.push({ check: 'RedundantDescriptionCheck', element: label(node), detail: `the description repeats the role "${word}"` })
    }
    return findings
  }

  static build() {
    const project = path.join(this.fixture, 'app')
    MobileAccessibilityFixture.write(project, 'dev.tachyon.accessibility-audit')
    console.log('==> building the Android app')
    this.run(this.binary, ['build', project, '--target', 'android'])
    return path.join(project, 'dist/android', MobileAccessibilityFixture.NAME, `${MobileAccessibilityFixture.NAME}.apk`)
  }

  static launch(apk) {
    this.run(this.adb, ['wait-for-device'])
    this.run(this.adb, ['install', '-r', apk])
    this.adbShell('am', 'force-stop', this.applicationId)
    this.adbShell('am', 'start', '-W', '-n', `${this.applicationId}/.MainActivity`)
  }

  static async check() {
    const fixture = MobileAccessibilityFixture
    const checks = []
    const home = await this.waitFor((nodes) => [fixture.HEADING, fixture.BUTTON, fixture.LINK].every((name) => this.named(nodes, name)), 'the page reaches the Android accessibility tree')
    const field = home.nodes.find((node) => node.class === this.editableClass)
    assert.ok(field, `the input is an ${this.editableClass}`)
    checks.push(`the input is an ${this.editableClass} named '${field.name}'`)
    assert.ok(field.name.includes(fixture.FIELD) || this.named(home.nodes, fixture.FIELD), 'the input or its label reads Note')
    const button = this.named(home.nodes, fixture.BUTTON)
    assert.equal(button.class, this.buttonClass, `the button is an ${this.buttonClass}`)
    assert.equal(button.clickable, 'true', 'the button can be clicked through accessibility')
    checks.push(`the button is an ${this.buttonClass} named '${fixture.BUTTON}'`)
    const link = this.named(home.nodes, fixture.LINK)
    assert.equal(link.clickable, 'true', 'the link can be clicked through accessibility')
    checks.push(`the link is a clickable ${link.class} named '${fixture.LINK}'`)
    const heading = this.named(home.nodes, fixture.HEADING)
    checks.push(`the heading is a ${heading.class} named '${fixture.HEADING}'`)
    const image = this.named(home.nodes, fixture.IMAGE)
    assert.ok(image, 'the svg with role img is named by its aria-label')
    checks.push(`the image is a ${image.class} named '${fixture.IMAGE}'`)
    assert.ok(this.named(home.nodes, fixture.IDLE), 'the status text reads its first value')
    const findings = this.audit(home.nodes, this.density())
    this.adbShell('input', 'tap', ...this.center(field))
    await this.pause()
    this.adbShell('input', 'text', fixture.TYPED)
    this.adbShell('input', 'keyevent', 'KEYCODE_ESCAPE')
    const typed = await this.waitFor((nodes) => this.named(nodes, fixture.BUTTON), 'the button stays in the tree after typing')
    this.adbShell('input', 'tap', ...this.center(this.named(typed.nodes, fixture.BUTTON)))
    const saved = await this.waitFor((nodes) => this.named(nodes, fixture.SAVED), 'pressing the button changes the status the system reads')
    checks.push(`typing and pressing the button changed the status to '${fixture.SAVED}'`)
    this.adbShell('input', 'tap', ...this.center(this.named(saved.nodes, fixture.LINK)))
    await this.waitFor((nodes) => this.named(nodes, fixture.ABOUT_HEADING), 'the link opens the next page in the accessibility tree')
    checks.push('the link opened the next page in the accessibility tree')
    return { checks, findings, nodes: home.nodes.map(({ children, ...node }) => node) }
  }

  static async main() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required')
    mkdirSync(this.evidence, { recursive: true })
    try {
      this.launch(this.build())
      const report = await this.check()
      const device = this.adbShell('getprop', 'ro.build.version.release')
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify({ device, ...report }, null, 2) + '\n')
      for (const check of report.checks) console.log(`OK: ${check}`)
      for (const finding of report.findings) console.log(`FINDING: ${finding.check} on ${finding.element}: ${finding.detail}`)
      assert.deepEqual(report.findings, [], 'the accessibility checks found issues')
      console.log(`PASS: on the Android ${device} emulator, the page's heading, image, text field, button and link reached the platform accessibility tree by name, typing, a press and a link worked from their accessibility bounds, and the ATF-style checks found no issue. Report: ${path.join(this.evidence, 'report.json')}`)
    } finally {
      spawnSync(this.adb, ['shell', 'am', 'force-stop', this.applicationId], { stdio: 'ignore' })
      rmSync(this.fixture, { recursive: true, force: true })
    }
  }
}

await AndroidAccessibilityTest.main()

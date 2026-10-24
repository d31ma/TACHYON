#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { MobileAccessibilityFixture } from '../native/mobile-accessibility-fixture.mjs'
import { AndroidAccessibilityTest } from './accessibility-test.mjs'

class AndroidSystemBackTest extends AndroidAccessibilityTest {
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'android-system-back-'))
  static evidence = process.env.TAC_ANDROID_SYSTEM_BACK_EVIDENCE ?? path.join(AndroidAccessibilityTest.repository, 'target/android-system-back-evidence')
  static gestureNavigation = '2'

  static resumed() {
    const activities = this.adbShell('dumpsys', 'activity', 'activities')
    return activities.split('\n').filter((line) => /ResumedActivity/.test(line)).some((line) => line.includes(`${this.applicationId}/`))
  }

  static async waitUntil(predicate, message) {
    for (let attempt = 0; attempt < this.waitAttempts; attempt++) {
      if (predicate()) return
      await this.pause()
    }
    assert.fail(message)
  }

  static backRecords() {
    return this.telemetry().split('\n').filter((line) => line.includes('"navigation.back"')).length
  }

  static telemetry() {
    const read = spawnSync(this.adb, ['shell', 'run-as', this.applicationId, 'sh', '-c', "'cat files/tachyon/*.jsonl'"], { encoding: 'utf8', timeout: 60000 })
    return read.status === 0 ? read.stdout : ''
  }

  static edgeSwipe() {
    const size = this.adbShell('wm', 'size').match(/(\d+)x(\d+)\s*$/)
    assert.ok(size, 'wm size gave no screen size')
    const [width, height] = [Number(size[1]), Number(size[2])]
    this.adbShell('input', 'swipe', '1', String(Math.round(height / 2)), String(Math.round(width * 0.6)), String(Math.round(height / 2)), '300')
  }

  static async openAbout(home) {
    this.adbShell('input', 'tap', ...this.center(this.named(home.nodes, MobileAccessibilityFixture.LINK)))
    return this.waitFor((nodes) => this.named(nodes, MobileAccessibilityFixture.ABOUT_HEADING), 'the link opens the next route')
  }

  static async check() {
    const fixture = MobileAccessibilityFixture
    const checks = []
    const earlier = this.backRecords()
    const atHome = (nodes) => this.named(nodes, fixture.HEADING) && this.named(nodes, fixture.LINK)
    const home = await this.waitFor(atHome, 'the first route reaches the screen')
    await this.openAbout(home)
    this.adbShell('input', 'keyevent', 'KEYCODE_BACK')
    const back = await this.waitFor(atHome, 'Back goes to the previous route')
    assert.ok(this.resumed(), 'Back from the second route keeps the app on screen')
    checks.push('Back on the second route opened the first route and kept the app on screen')
    const navigationMode = this.adbShell('settings', 'get', 'secure', 'navigation_mode')
    if (navigationMode === this.gestureNavigation) {
      await this.openAbout(back)
      this.edgeSwipe()
      await this.waitFor(atHome, 'the back gesture goes to the previous route')
      checks.push('the back gesture from the left edge on the second route opened the first route')
    } else {
      checks.push(`the emulator uses navigation mode ${navigationMode}, not gestures, so the back gesture was not swiped (the Back key reaches the same OnBackInvokedCallback)`)
    }
    const recorded = this.backRecords() - earlier
    assert.ok(recorded >= 1, 'the host recorded navigation.back')
    checks.push(`the host recorded navigation.back ${recorded} time(s)`)
    this.adbShell('input', 'keyevent', 'KEYCODE_BACK')
    await this.waitUntil(() => !this.resumed(), 'Back on the first route leaves the app')
    checks.push('Back on the first route left the app the system way')
    return { checks }
  }

  static async main() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required')
    mkdirSync(this.evidence, { recursive: true })
    try {
      this.installProbe()
      this.launch(this.build())
      const report = await this.check()
      const device = this.adbShell('getprop', 'ro.build.version.release')
      const level = this.adbShell('getprop', 'ro.build.version.sdk')
      writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify({ device, level, ...report }, null, 2) + '\n')
      for (const check of report.checks) console.log(`OK: ${check}`)
      console.log(`PASS: on the Android ${device} (API ${level}) emulator, Back went back in page history to the previous route and left the app from the first route. Report: ${path.join(this.evidence, 'report.json')}`)
    } finally {
      spawnSync(this.adb, ['shell', 'am', 'force-stop', this.applicationId], { stdio: 'ignore' })
      spawnSync(this.adb, ['shell', 'rm', '-f', this.probeDevicePath], { stdio: 'ignore' })
      rmSync(this.fixture, { recursive: true, force: true })
      rmSync(AndroidAccessibilityTest.fixture, { recursive: true, force: true })
    }
  }
}

await AndroidSystemBackTest.main()

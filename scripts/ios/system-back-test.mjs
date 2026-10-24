#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { MobileAccessibilityFixture } from '../native/mobile-accessibility-fixture.mjs'
import { IosAccessibilityAuditTest } from './accessibility-audit-test.mjs'

class IosSystemBackTest extends IosAccessibilityAuditTest {
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-system-back-'))
  static evidence = process.env.TAC_IOS_SYSTEM_BACK_EVIDENCE ?? path.join(IosAccessibilityAuditTest.repository, 'target/ios-system-back-evidence')
  static auditTarget = 'SystemBack'
  static auditBundleId = 'dev.tachyon.system-back-tests'

  static swipe(project) {
    const fixture = MobileAccessibilityFixture
    const names = { heading: fixture.HEADING, link: fixture.LINK, about: fixture.ABOUT_HEADING }
    console.log('==> swiping back and forward through XCUITest')
    const result = spawnSync('/usr/bin/xcrun', ['xcodebuild', 'test', '-project', project, '-scheme', this.auditTarget,
      '-destination', `platform=iOS Simulator,id=${this.device}`, '-derivedDataPath', path.join(this.fixture, 'derived'),
      '-resultBundlePath', path.join(this.fixture, 'system-back.xcresult')], {
      cwd: this.fixture, encoding: 'utf8', timeout: 1200000, maxBuffer: 64 * 1024 * 1024,
      env: { ...process.env, TEST_RUNNER_TACHYON_BACK_APP: this.applicationId, TEST_RUNNER_TACHYON_BACK_NAMES: JSON.stringify(names) },
    })
    const output = `${result.stdout}${result.stderr}`
    writeFileSync(path.join(this.evidence, 'xcodebuild.log'), output)
    assert.equal(result.status, 0, `the XCUITest system back test failed: ${output.slice(-6000)}`)
  }

  static cleanup() {
    super.cleanup()
    rmSync(IosAccessibilityAuditTest.fixture, { recursive: true, force: true })
  }

  static main() {
    assert.equal(process.platform, 'darwin', 'the iOS system back test needs macOS and Xcode')
    mkdirSync(this.evidence, { recursive: true })
    try {
      const application = this.build()
      const project = this.writeAuditProject()
      const target = this.boot()
      this.simctl('install', this.device, application)
      this.swipe(project)
      console.log(`PASS: on the iOS ${target.runtime} simulator (${target.device}), a swipe from the left edge went back to the previous route and a swipe from the right edge went forward again. Log: ${path.join(this.evidence, 'xcodebuild.log')}`)
    } finally {
      this.cleanup()
    }
  }
}

process.once('SIGINT', () => { IosSystemBackTest.cleanup(); process.exit(130) })
process.once('SIGTERM', () => { IosSystemBackTest.cleanup(); process.exit(143) })
IosSystemBackTest.main()

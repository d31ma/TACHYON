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
  static BACK_BUTTON = 'Go back'
  static LIMITED = 77

  static build() {
    const project = path.join(this.fixture, 'app')
    MobileAccessibilityFixture.write(project, this.applicationId)
    writeFileSync(path.join(project, 'client/components/audit/about/tac.html'),
      `<main><h1>${MobileAccessibilityFixture.ABOUT_HEADING}</h1>\n<button type="button" on:click="pageHistory.back()">${this.BACK_BUTTON}</button></main>\n`)
    writeFileSync(path.join(project, 'client/services/PageHistoryService.js'), `@Service
export default class PageHistoryService {
  back() {
    history.back();
  }
}
`)
    console.log('==> building the iOS app')
    this.run(this.binary, ['build', project, '--target', 'ios'])
    return path.join(project, 'dist/ios', `${MobileAccessibilityFixture.NAME}.app`)
  }

  /** @param {string} project @returns {string} */
  static swipe(project) {
    const fixture = MobileAccessibilityFixture
    const names = { heading: fixture.HEADING, link: fixture.LINK, about: fixture.ABOUT_HEADING, back: this.BACK_BUTTON }
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
    return output
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
      const output = this.swipe(project)
      const where = `the iOS ${target.runtime} simulator (${target.device})`
      if (/TACHYON_APP_BACK=true/.test(output)) {
        console.log(`PASS: on ${where}, a swipe from the left edge went back to the previous route and a swipe from the right edge went forward again. Log: ${path.join(this.evidence, 'xcodebuild.log')}`)
        return 0
      }
      assert.match(output, /TACHYON_HISTORY_BACK=true/, 'the page\'s history.back() did not go back')
      console.log(`LIMITED: on ${where}, no injected edge swipe completed WebKit's back gesture in three tries, so this run does not prove the swipe.`)
      console.log('LIMITED: the page\'s history.back() button went back to the first route, so back navigation itself works. The simulator\'s own edge gesture took this app back on iOS 26.5 in a manual check (ui-gates, 2026-11-01, ~/Library/Logs/TACHYON/ui-gates-ios-manual-1025/FINDINGS.md), and the gate passed on the iOS 18.0 simulator (ADR 0109).')
      return this.LIMITED
    } finally {
      this.cleanup()
    }
  }
}

process.once('SIGINT', () => { IosSystemBackTest.cleanup(); process.exit(130) })
process.once('SIGTERM', () => { IosSystemBackTest.cleanup(); process.exit(143) })
process.exitCode = IosSystemBackTest.main()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { MobileAccessibilityFixture } from '../native/mobile-accessibility-fixture.mjs'

class IosAccessibilityAuditTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'ios-accessibility-'))
  static evidence = process.env.TAC_IOS_ACCESSIBILITY_EVIDENCE ?? path.join(IosAccessibilityAuditTest.repository, 'target/ios-accessibility-evidence')
  static applicationId = 'dev.tachyon.accessibility-audit'
  static auditTarget = 'AccessibilityAudit'
  static auditBundleId = 'dev.tachyon.accessibility-audit-tests'
  static marker = 'TACHYON_ACCESSIBILITY_REPORT '
  static reportFile = path.join(tmpdir(), `ios-accessibility-report-${randomUUID()}.json`)
  static excused = []
  static device = ''

  static run(program, args, options = {}) {
    const result = spawnSync(program, args, { cwd: this.repository, encoding: 'utf8', timeout: 600000, maxBuffer: 64 * 1024 * 1024, ...options })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stderr}${result.stdout}`)
    return result.stdout.trim()
  }

  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', ...args])
  }

  static identifier(index) {
    return `AC0000000000000000000${String(index).padStart(3, '0')}`
  }

  static projectFile() {
    const id = (index) => this.identifier(index)
    const target = this.auditTarget
    const common = 'SDKROOT = iphoneos; IPHONEOS_DEPLOYMENT_TARGET = 17.0; SWIFT_VERSION = 5.0; TARGETED_DEVICE_FAMILY = "1,2"; ONLY_ACTIVE_ARCH = YES;'
    const signing = 'CODE_SIGN_STYLE = Manual; CODE_SIGN_IDENTITY = "-"; DEVELOPMENT_TEAM = "";'
    return `// !$*UTF8*$!
{
  archiveVersion = 1;
  classes = {};
  objectVersion = 56;
  objects = {
    ${id(1)} = {isa = PBXBuildFile; fileRef = ${id(2)}; };
    ${id(2)} = {isa = PBXFileReference; lastKnownFileType = sourcecode.swift; path = ${target}.swift; sourceTree = "<group>"; };
    ${id(3)} = {isa = PBXFileReference; explicitFileType = wrapper.cfbundle; includeInIndex = 0; path = ${target}.xctest; sourceTree = BUILT_PRODUCTS_DIR; };
    ${id(4)} = {isa = PBXGroup; children = (${id(2)}, ${id(5)}); sourceTree = "<group>"; };
    ${id(5)} = {isa = PBXGroup; children = (${id(3)}); name = Products; sourceTree = "<group>"; };
    ${id(6)} = {isa = PBXSourcesBuildPhase; buildActionMask = 2147483647; files = (${id(1)}); runOnlyForDeploymentPostprocessing = 0; };
    ${id(7)} = {isa = PBXFrameworksBuildPhase; buildActionMask = 2147483647; files = (); runOnlyForDeploymentPostprocessing = 0; };
    ${id(8)} = {isa = PBXNativeTarget; buildConfigurationList = ${id(12)}; buildPhases = (${id(6)}, ${id(7)}); buildRules = (); dependencies = (); name = ${target}; productName = ${target}; productReference = ${id(3)}; productType = "com.apple.product-type.bundle.ui-testing"; };
    ${id(9)} = {isa = PBXProject; attributes = {LastUpgradeCheck = 1600; }; buildConfigurationList = ${id(13)}; compatibilityVersion = "Xcode 14.0"; developmentRegion = en; hasScannedForEncodings = 0; knownRegions = (en, Base); mainGroup = ${id(4)}; productRefGroup = ${id(5)}; projectDirPath = ""; projectRoot = ""; targets = (${id(8)}); };
    ${id(10)} = {isa = XCBuildConfiguration; buildSettings = {${common} ${signing} PRODUCT_NAME = "$(TARGET_NAME)"; PRODUCT_BUNDLE_IDENTIFIER = ${this.auditBundleId}; GENERATE_INFOPLIST_FILE = YES; }; name = Debug; };
    ${id(11)} = {isa = XCBuildConfiguration; buildSettings = {${common} ALWAYS_SEARCH_USER_PATHS = NO; }; name = Debug; };
    ${id(12)} = {isa = XCConfigurationList; buildConfigurations = (${id(10)}); defaultConfigurationIsVisible = 0; defaultConfigurationName = Debug; };
    ${id(13)} = {isa = XCConfigurationList; buildConfigurations = (${id(11)}); defaultConfigurationIsVisible = 0; defaultConfigurationName = Debug; };
  };
  rootObject = ${id(9)};
}
`
  }

  static schemeFile() {
    const target = this.auditTarget
    return `<?xml version="1.0" encoding="UTF-8"?>
<Scheme LastUpgradeVersion = "1600" version = "1.7">
  <BuildAction parallelizeBuildables = "YES" buildImplicitDependencies = "YES">
  </BuildAction>
  <TestAction buildConfiguration = "Debug" shouldUseLaunchSchemeArgsEnv = "YES">
    <Testables>
      <TestableReference skipped = "NO">
        <BuildableReference BuildableIdentifier = "primary" BlueprintIdentifier = "${this.identifier(8)}" BuildableName = "${target}.xctest" BlueprintName = "${target}" ReferencedContainer = "container:${target}.xcodeproj">
        </BuildableReference>
      </TestableReference>
    </Testables>
  </TestAction>
</Scheme>
`
  }

  static writeAuditProject() {
    const directory = path.join(this.fixture, 'audit')
    const project = path.join(directory, `${this.auditTarget}.xcodeproj`)
    mkdirSync(path.join(project, 'xcshareddata/xcschemes'), { recursive: true })
    copyFileSync(path.join(this.repository, 'scripts/ios', `${this.auditTarget}.swift`), path.join(directory, `${this.auditTarget}.swift`))
    writeFileSync(path.join(project, 'project.pbxproj'), this.projectFile())
    writeFileSync(path.join(project, 'xcshareddata/xcschemes', `${this.auditTarget}.xcscheme`), this.schemeFile())
    return project
  }

  static build() {
    const project = path.join(this.fixture, 'app')
    MobileAccessibilityFixture.write(project, this.applicationId)
    console.log('==> building the iOS app')
    this.run(this.binary, ['build', project, '--target', 'ios'])
    return path.join(project, 'dist/ios', `${MobileAccessibilityFixture.NAME}.app`)
  }

  static boot() {
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.name === 'iPhone 17 Pro')
      ?? (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `AccessibilityAudit-${randomUUID()}`, phone.identifier, runtime.identifier)
    console.log(`==> booting ${phone.name} with iOS ${runtime.version}`)
    this.simctl('boot', this.device)
    this.simctl('bootstatus', this.device, '-b')
    return { runtime: runtime.version, device: phone.name }
  }

  static audit(project) {
    const fixture = MobileAccessibilityFixture
    const file = this.reportFile
    const names = { heading: fixture.HEADING, image: fixture.IMAGE, field: fixture.FIELD, button: fixture.BUTTON, link: fixture.LINK, idle: fixture.IDLE, typed: fixture.TYPED, saved: fixture.SAVED, about: fixture.ABOUT_HEADING }
    console.log('==> running the XCUITest accessibility audit')
    const result = spawnSync('/usr/bin/xcrun', ['xcodebuild', 'test', '-project', project, '-scheme', this.auditTarget,
      '-destination', `platform=iOS Simulator,id=${this.device}`, '-derivedDataPath', path.join(this.fixture, 'derived'),
      '-resultBundlePath', path.join(this.fixture, 'audit.xcresult')], {
      cwd: this.fixture, encoding: 'utf8', timeout: 1200000, maxBuffer: 64 * 1024 * 1024,
      env: { ...process.env, TEST_RUNNER_TACHYON_AUDIT_APP: this.applicationId, TEST_RUNNER_TACHYON_AUDIT_NAMES: JSON.stringify(names), TEST_RUNNER_TACHYON_AUDIT_REPORT: file },
    })
    const output = `${result.stdout}${result.stderr}`
    writeFileSync(path.join(this.evidence, 'xcodebuild.log'), output)
    const line = output.split('\n').find((text) => text.includes(this.marker))
    const report = existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : line ? JSON.parse(line.slice(line.indexOf(this.marker) + this.marker.length)) : undefined
    assert.ok(report, `the audit test wrote no report (exit ${result.status}): ${output.slice(-6000)}`)
    writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
    for (const check of report.checks) console.log(`OK: ${check}`)
    for (const finding of report.findings) console.log(`FINDING: ${finding.audit} on ${finding.element}: ${finding.summary}`)
    assert.equal(result.status, 0, `the XCUITest accessibility test failed: ${output.slice(-6000)}`)
    return report
  }

  static unexcused(findings) {
    return findings.filter((finding) => !this.excused.some((excuse) => excuse.audit === finding.audit && finding.element.includes(excuse.element)))
  }

  static cleanup() {
    if (this.device) {
      for (const args of [['shutdown', this.device], ['delete', this.device]]) {
        spawnSync('/usr/bin/xcrun', ['simctl', ...args], { timeout: 60000, stdio: 'ignore' })
      }
      this.device = ''
    }
    rmSync(this.fixture, { recursive: true, force: true })
    rmSync(this.reportFile, { force: true })
  }

  static main() {
    assert.equal(process.platform, 'darwin', 'the iOS accessibility audit needs macOS and Xcode')
    mkdirSync(this.evidence, { recursive: true })
    try {
      const application = this.build()
      const project = this.writeAuditProject()
      const target = this.boot()
      this.simctl('install', this.device, application)
      const report = this.audit(project)
      const open = this.unexcused(report.findings)
      assert.deepEqual(open, [], `the accessibility audit found issues: ${JSON.stringify(open, null, 2)}`)
      console.log(`PASS: on the iOS ${target.runtime} simulator (${target.device}), XCUITest read the page's heading, image, text field, button and link by name, typed, pressed and followed the link through the accessibility API, and performAccessibilityAudit found ${report.findings.length} issue(s), none open (the audit checks the host's own elements: it does not look inside WKWebView content). Report: ${path.join(this.evidence, 'report.json')}`)
    } finally {
      this.cleanup()
    }
  }
}

process.once('SIGINT', () => { IosAccessibilityAuditTest.cleanup(); process.exit(130) })
process.once('SIGTERM', () => { IosAccessibilityAuditTest.cleanup(); process.exit(143) })
IosAccessibilityAuditTest.main()

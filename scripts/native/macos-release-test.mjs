#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawn, spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, readlinkSync, rmSync, rmdirSync, statSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

assert.equal(process.platform, 'darwin', 'The macOS release check needs macOS.')

class MacOsReleaseTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static team = process.env.TAC_APPLE_TEAM_ID ?? ''
  static profile = process.env.TAC_APPLE_NOTARY_PROFILE ?? ''
  static scratch = process.env.TAC_PROBE_DIR ?? path.join(MacOsReleaseTest.repository, 'target')
  static root = MacOsReleaseTest.folder(MacOsReleaseTest.scratch)
  static evidence = process.env.TAC_MACOS_RELEASE_EVIDENCE ?? path.join(MacOsReleaseTest.repository, 'target/macos-release-evidence')
  static compare = process.env.TAC_MACOS_RELEASE_COMPARE !== '0'
  static feature = path.join(MacOsReleaseTest.repository, 'website/content/features/web-assembly-services/client')
  static probe = path.join(MacOsReleaseTest.root, 'page-checks-macos-probe')
  static architecture = path.join(MacOsReleaseTest.root, 'macos-running-architecture')
  // hdiutil refuses a mount point on an external volume, where a worktree's target folder may live.
  static mount = path.join(tmpdir(), `macos-release-mount-${process.pid}`)
  static name = 'Tally Release'
  static application = 'TallyRelease'
  static id = 'dev.tachyon.tally-macos-release'
  static dotnetWasm = 'dotnet.native.wasm'
  static dotnetWasmFile = /^dotnet\.native(\.[a-z0-9]+)?\.wasm$/
  static architectures = ['arm64', 'x86_64']
  static notarized = 'Notarized Developer ID'
  static releaseTimeout = 2_400_000
  static tallySteps = [
    ['wait', '0 in the basket', 'Add two'],
    ['press', 'Add two'],
    ['wait', '2 in the basket', '2 items'],
  ]

  /** @param {string} parent */
  static folder(parent) {
    mkdirSync(parent, { recursive: true })
    return mkdtempSync(path.join(parent, 'macos-release-'))
  }

  /** @param {string} program @param {string[]} args @param {{ timeout?: number, env?: NodeJS.ProcessEnv }} [options] */
  static run(program, args, options = {}) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout: options.timeout ?? 900_000, env: options.env ?? process.env, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')} failed:\n${result.error ?? ''}${result.stdout}${result.stderr}`)
    return `${result.stdout}${result.stderr}`.trim()
  }

  /** @param {string} project @param {boolean} aheadOfTime */
  static stage(project, aheadOfTime) {
    for (const entry of ['pages', 'components']) cpSync(path.join(this.feature, entry), path.join(project, 'client', entry), { recursive: true })
    mkdirSync(path.join(project, 'client/services'), { recursive: true })
    copyFileSync(path.join(this.feature, 'services/TallyService.cs'), path.join(project, 'client/services/TallyService.cs'))
    const manifest = JSON.parse(readFileSync(path.join(this.feature, 'manifest.json'), 'utf8'))
    manifest.name = this.name
    manifest.start_url = '/'
    manifest.tac = { ...manifest.tac, application: { ...manifest.tac?.application, id: this.id, version: '1.0.0', build: 1 }, dotnet: { aot: aheadOfTime } }
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest, null, 2))
  }

  /** @param {string} app */
  static webAssembly(app) {
    const folder = path.join(app, 'Contents/Resources')
    const files = readdirSync(folder, { recursive: true }).filter((name) => String(name).endsWith('.wasm'))
    return Object.fromEntries(files.map((name) => [String(name), statSync(path.join(folder, String(name))).size]).sort())
  }

  /** @param {Record<string, number>} modules */
  static dotnetSize(modules) {
    const found = Object.entries(modules).find(([name]) => this.dotnetWasmFile.test(path.basename(name)))
    assert.ok(found, `the app holds no ${this.dotnetWasm}: ${Object.keys(modules).join(', ')}`)
    return found[1]
  }

  /** @param {string[]} output */
  static notes(output) {
    return output.join('\n').split('\n').filter((line) => line.startsWith('Note:'))
  }

  static interpreterBuild() {
    const project = path.join(this.root, 'interpreter')
    this.stage(project, false)
    const output = this.run(this.binary, ['bundle', project, '--target', 'macos'], { timeout: 1_800_000 })
    const modules = this.webAssembly(path.join(project, 'dist/macos', `${this.application}.app`))
    return { modules, dotnet: this.dotnetSize(modules), notes: this.notes([output]) }
  }

  static releaseBuild() {
    const project = path.join(this.root, 'release')
    this.stage(project, true)
    const environment = { ...process.env, TAC_APPLE_TEAM_ID: this.team, TAC_APPLE_NOTARY_PROFILE: this.profile }
    delete environment.TAC_MACOS_SIGNING_IDENTITY
    const started = Date.now()
    const output = this.run(this.binary, ['bundle', project, '--target', 'macos', '--arch', 'universal', '--release'], { timeout: this.releaseTimeout, env: environment })
    const seconds = Math.round((Date.now() - started) / 1000)
    assert.ok(!output.includes('signed ad hoc'), `the release was signed ad hoc:\n${output}`)
    const folder = path.join(project, 'dist/macos')
    const dmg = path.join(folder, `${this.application}.dmg`)
    assert.ok(existsSync(dmg), `no ${this.application}.dmg:\n${output}`)
    const app = path.join(folder, `${this.application}.app`)
    const modules = this.webAssembly(app)
    return { project, dmg, app, seconds, modules, dotnet: this.dotnetSize(modules), notes: this.notes([output]) }
  }

  /** @param {string} app */
  static signature(app) {
    const executable = path.join(app, 'Contents/MacOS', this.application)
    const slices = this.run('/usr/bin/lipo', ['-archs', executable]).split(/\s+/).sort()
    assert.deepEqual(slices, this.architectures, `the app executable holds ${slices.join(' ')}`)
    this.run('/usr/bin/codesign', ['--verify', '--strict', '--deep', '--verbose=2', app])
    const details = this.run('/usr/bin/codesign', ['--display', '--verbose=2', app])
    assert.ok(details.includes(`TeamIdentifier=${this.team}`), details)
    assert.ok(details.includes('Authority=Developer ID Application'), details)
    assert.ok(details.split('\n').some((line) => line.startsWith('CodeDirectory') && line.includes('runtime')), details)
    assert.ok(details.includes('Timestamp='), `the signature has no secure timestamp:\n${details}`)
    const lines = details.split('\n').filter((line) => /^(Identifier|Format|Authority|TeamIdentifier|Timestamp|CodeDirectory)/.test(line))
    return { slices, details: lines }
  }

  /** @param {string} dmg */
  static diskImage(dmg) {
    const stapled = this.run('/usr/bin/xcrun', ['stapler', 'validate', dmg])
    const assessed = this.run('/usr/sbin/spctl', ['--assess', '--type', 'open', '--context', 'context:primary-signature', '--verbose=2', dmg])
    assert.ok(assessed.includes(this.notarized), assessed)
    const sha256 = createHash('sha256').update(readFileSync(dmg)).digest('hex')
    return { bytes: statSync(dmg).size, sha256, stapled, assessed }
  }

  /** @param {string} dmg */
  static attach(dmg) {
    mkdirSync(this.mount)
    this.run('/usr/bin/hdiutil', ['attach', '-nobrowse', '-readonly', '-mountpoint', this.mount, dmg])
    const app = path.join(this.mount, `${this.application}.app`)
    assert.ok(existsSync(app), 'the disk image holds no app')
    assert.equal(readlinkSync(path.join(this.mount, 'Applications')), '/Applications')
    this.run('/usr/bin/codesign', ['--verify', '--strict', '--deep', app])
    const assessed = this.run('/usr/sbin/spctl', ['--assess', '--type', 'execute', '--verbose=2', app])
    assert.ok(assessed.includes(this.notarized), assessed)
    return { app, assessed }
  }

  static detach() {
    spawnSync('/usr/bin/hdiutil', ['detach', this.mount, '-force'], { stdio: 'ignore', timeout: 60_000 })
    if (existsSync(this.mount)) rmdirSync(this.mount)
  }

  static forgetWebsiteData() {
    const library = path.join(homedir(), 'Library')
    for (const folder of ['WebKit', 'Caches', 'HTTPStorages']) rmSync(path.join(library, folder, this.id), { recursive: true, force: true })
  }

  /** @param {Array<string[]>} steps */
  static async drive(steps) {
    for (const [action, ...values] of steps) {
      await promisify(execFile)(this.probe, [this.id, action, ...values], { timeout: 180_000 })
    }
  }

  /** @param {string} app @param {string} slice */
  static launch(app, slice) {
    if (slice === 'arm64') {
      this.run('/usr/bin/open', ['-n', app])
      return undefined
    }
    return spawn('/usr/bin/arch', [`-${slice}`, path.join(app, 'Contents/MacOS', this.application)], { stdio: 'ignore', detached: true })
  }

  /** @param {string} app @param {string} slice */
  static async answer(app, slice) {
    const opened = Date.now()
    const child = this.launch(app, slice)
    try {
      const [pid, running] = this.run(this.architecture, [this.id]).split(' ')
      assert.equal(running, slice, `the ${slice} launch runs as ${running}`)
      await this.drive(this.tallySteps)
      const seconds = Number(((Date.now() - opened) / 1000).toFixed(1))
      await this.drive([['quit']])
      console.log(`ok - the ${slice} slice of the notarized app answered a click from its ahead-of-time C# service in ${seconds} s (pid ${pid})`)
      return { slice, pid: Number(pid), running, seconds }
    } finally {
      child?.kill()
      spawnSync('/usr/bin/pkill', ['-f', path.join(app, 'Contents/MacOS')])
      this.forgetWebsiteData()
    }
  }

  static compileTools() {
    this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/native/page-checks-macos-probe.swift'), '-o', this.probe])
    this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/native/macos-running-architecture.swift'), '-o', this.architecture])
  }

  /** @param {object} report */
  static write(report) {
    writeFileSync(path.join(this.evidence, 'report.json'), JSON.stringify(report, null, 2) + '\n')
  }

  static async main() {
    assert.match(this.team, /^[A-Z0-9]{10}$/, 'TAC_APPLE_TEAM_ID names the Developer ID team')
    assert.ok(this.profile, 'TAC_APPLE_NOTARY_PROFILE names a notarytool keychain profile')
    mkdirSync(this.evidence, { recursive: true })
    const report = { team: this.team, profile: this.profile, sw_vers: this.run('/usr/bin/sw_vers', []), xcode: this.run('/usr/bin/xcrun', ['xcodebuild', '-version']) }
    try {
      this.compileTools()
      if (this.compare) report.interpreter = this.interpreterBuild()
      const release = this.releaseBuild()
      report.release = { seconds: release.seconds, modules: release.modules, dotnet: release.dotnet, notes: release.notes }
      console.log(`ok - ty bundle --target macos --arch universal --release built, signed and notarized ${this.application}.dmg in ${release.seconds} s`)
      if (report.interpreter) {
        assert.ok(release.dotnet > report.interpreter.dotnet, `the ahead-of-time ${this.dotnetWasm} (${release.dotnet} bytes) is not larger than the interpreter one (${report.interpreter.dotnet} bytes)`)
        console.log(`ok - ${this.dotnetWasm} is ${(release.dotnet / 1048576).toFixed(2)} MB ahead of time against ${(report.interpreter.dotnet / 1048576).toFixed(2)} MB for the interpreter`)
      }
      report.signature = this.signature(release.app)
      console.log(`ok - the app is ${report.signature.slices.join(' ')}, Developer ID signed by team ${this.team} with the hardened runtime and a secure timestamp`)
      report.diskImage = this.diskImage(release.dmg)
      copyFileSync(release.dmg, path.join(this.evidence, `${this.application}.dmg`))
      console.log(`ok - ${this.application}.dmg is stapled and Gatekeeper reads it as ${this.notarized}`)
      this.write(report)
      try {
        report.mounted = this.attach(release.dmg)
        console.log(`ok - the app inside the mounted disk image is accepted by Gatekeeper as ${this.notarized}`)
        report.launches = []
        for (const slice of this.architectures) report.launches.push(await this.answer(report.mounted.app, slice))
      } finally {
        this.detach()
      }
      this.write(report)
    } finally {
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    }
    console.log(`PASS: a universal Developer ID release of a C# page service with tac.dotnet.aot was notarized, stapled and accepted by Gatekeeper, and both its arm64 and x86_64 slices answered a click from the mounted disk image. Report: ${path.join(this.evidence, 'report.json')}`)
  }
}

await MacOsReleaseTest.main()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { PeersNativeProbe } from './peers-native-probe.mjs'

class MacOsPeersTest {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static temporary = mkdtempSync(path.join(tmpdir(), 'tachyon-peers-macos-'))
  static project = path.join(MacOsPeersTest.temporary, 'project')
  static helper = path.join(MacOsPeersTest.temporary, 'accessibility')
  static processName = `TachyonPeers${process.pid}`
  static bundleIdentifier = `dev.tachyon.peers-gate.g${process.pid}`
  static waitRounds = 120
  static waitStep = 500

  /** @param {string} program @param {string[]} args */
  static run(program, args) {
    const result = spawnSync(program, args, { cwd: MacOsPeersTest.repository, encoding: 'utf8', timeout: 900000, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`)
    return result.stdout
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds))
  }

  /** @param {number} port */
  static stage(port) {
    const settings = { id: MacOsPeersTest.bundleIdentifier, name: MacOsPeersTest.processName, ...PeersNativeProbe.relayed(port) }
    for (const [name, text] of Object.entries(PeersNativeProbe.files(settings))) {
      const file = path.join(MacOsPeersTest.project, name)
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, text)
    }
  }

  static compileHelper() {
    MacOsPeersTest.run('/usr/bin/xcrun', ['swiftc', '-O', '-framework', 'AppKit', '-framework', 'ApplicationServices', path.join(MacOsPeersTest.repository, 'scripts/phase4-macos-accessibility.swift'), '-o', MacOsPeersTest.helper])
  }

  static build() {
    MacOsPeersTest.run(MacOsPeersTest.binary, ['build', MacOsPeersTest.project, '--target', 'macos'])
    const app = path.join(MacOsPeersTest.project, 'dist/macos', `${MacOsPeersTest.processName}.app`)
    assert.ok(existsSync(app), `the build made no app at ${app}`)
    return app
  }

  static read() {
    const result = spawnSync(MacOsPeersTest.helper, [MacOsPeersTest.processName], { encoding: 'utf8', timeout: 30000 })
    if (result.status !== 0) return ''
    const element = JSON.parse(result.stdout).elements.find(candidate => candidate.value?.startsWith(PeersNativeProbe.PREFIX))
    return element ? element.value.slice(PeersNativeProbe.PREFIX.length) : ''
  }

  static async result() {
    let text = ''
    for (let round = 0; round < MacOsPeersTest.waitRounds; round++) {
      text = MacOsPeersTest.read()
      if (text && text !== PeersNativeProbe.CHECKING) return text
      await MacOsPeersTest.pause(MacOsPeersTest.waitStep)
    }
    return text || 'no result in the Accessibility tree'
  }

  static cleanup(app, turn) {
    app?.kill('SIGTERM')
    turn?.kill('SIGTERM')
    for (const folder of [path.join(homedir(), 'Library/WebKit', MacOsPeersTest.bundleIdentifier), path.join(homedir(), 'Library/Caches', MacOsPeersTest.bundleIdentifier)]) rmSync(folder, { recursive: true, force: true })
    rmSync(MacOsPeersTest.temporary, { recursive: true, force: true })
  }

  static async main() {
    assert.equal(process.platform, 'darwin', 'the macOS peers check needs macOS')
    const port = await PeersNativeProbe.freePort()
    const turn = PeersNativeProbe.startTurn(port)
    let app
    try {
      MacOsPeersTest.stage(port)
      MacOsPeersTest.compileHelper()
      const bundle = MacOsPeersTest.build()
      app = spawn(path.join(bundle, 'Contents/MacOS', MacOsPeersTest.processName), [], { stdio: 'ignore' })
      const result = await MacOsPeersTest.result()
      assert.equal(result, PeersNativeProbe.PASSED, `two Peers connections in the macOS WKWebView must connect through the local TURN relay and carry ${PeersNativeProbe.TOTAL} bytes, got ${result}`)
      console.log(`PASS: in the macOS app (tachyon-app://bundle), Peers connected two connections through a local coturn relay with relayOnly and delivered ${PeersNativeProbe.TOTAL} bytes on an ordered channel`)
    } finally {
      MacOsPeersTest.cleanup(app, turn)
    }
  }
}

await MacOsPeersTest.main()

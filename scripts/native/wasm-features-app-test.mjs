#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { WasmFeaturesFixture } from './wasm-features-fixture.mjs'

class LinuxWasmApp {
  static screen = '1280x1000x24'
  static waitSeconds = 240
  static lines = ['ok - ', 'not ok - ']

  /** @param {string} project @param {string} application */
  static bundle(project, application) {
    return path.join(project, 'dist/linux', application, 'resources/WebBundle')
  }

  /** @param {string[]} gestures */
  static steps(gestures) {
    const clicks = gestures.flatMap((label) => [['wait', [`Probe waiting for ${label}`]], ['pointer', label]])
    return [...clicks, ['verdict', { passed: WasmFeaturesFixture.passed, failed: WasmFeaturesFixture.failed, lines: this.lines, seconds: this.waitSeconds }]]
  }

  /** @param {string} project @param {{ application: string, gestures: string[] }} staged */
  static async drive(project, staged) {
    const app = path.join(project, 'dist/linux', staged.application, 'bin', staged.application)
    const script = `set -euo pipefail
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
timeout ${this.waitSeconds + 120}s python3 "$PROBE" "$STEPS"`
    const env = {
      ...process.env,
      APP: app,
      PROBE: path.join(WasmFeaturesAppTest.repository, 'scripts/linux/page-steps-probe.py'),
      STEPS: JSON.stringify(this.steps(staged.gestures)),
      TAC_PROBE_APPLICATION: staged.application,
      XDG_STATE_HOME: path.join(project, 'state'),
      XDG_DATA_HOME: path.join(project, 'data'),
      XDG_CACHE_HOME: path.join(project, 'cache'),
      GDK_BACKEND: 'x11',
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const output = await WasmFeaturesAppTest.run('xvfb-run', ['--auto-servernum', `--server-args=-screen 0 ${this.screen}`, 'dbus-run-session', '--', 'bash', '-c', script], { env, timeoutMilliseconds: (this.waitSeconds + 180) * 1000 })
    return output.split('\n').filter((line) => /^(OK|LOADED|REPORT|PASS)/.test(line))
  }

  /** @param {string} project @param {{ application: string, gestures: string[] }} staged */
  static async run(project, staged) {
    await WasmFeaturesAppTest.run(WasmFeaturesAppTest.binary, ['build', project, '--target', 'linux'], { timeoutMilliseconds: 1800000 })
    const modules = WasmFeaturesFixture.assertWebAssembly(path.join(project, 'dist/linux'), this.bundle(project, staged.application))
    const lines = await this.drive(project, staged)
    return { modules, lines }
  }
}

class WasmFeaturesAppTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'wasm-features-app-'))
  static hosts = { linux: LinuxWasmApp }
  static target = WasmFeaturesAppTest.option('target', 'linux')
  static checks = WasmFeaturesAppTest.option('check', Object.keys(WasmFeaturesFixture.checks).join(',')).split(',')
  static languages = (process.env.TAC_WASM_FEATURE_LANGUAGES ?? '').split(',').filter(Boolean)
  static gestures = process.env.TAC_WASM_FEATURE_GESTURES !== 'false'
  static receipt = process.env.TAC_WASM_FEATURES_RECEIPT

  /** @param {string} name @param {string} fallback */
  static option(name, fallback) {
    return process.argv.find((argument) => argument.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback
  }

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} options */
  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address()
        server.close(() => resolve(port))
      })
    })
  }

  /** @param {{ check: string, language: string }} item */
  static async check({ check, language }) {
    const host = this.hosts[this.target]
    const project = path.join(this.root, `${check}-${language}`)
    const port = await this.freePort()
    const staged = WasmFeaturesFixture.stage(project, check, language, { yonOrigin: `http://127.0.0.1:${port}`, gestures: this.gestures })
    const started = Date.now()
    const serve = ['serve', project, '--host', '127.0.0.1', '--port', String(port), '--out-dir', 'serve-dist', '--no-watch', '--no-self-restart']
    const answer = staged.needsYon
      ? await withServer(this.binary, serve, () => host.run(project, staged), { readinessTimeoutMilliseconds: 600000, timeoutMilliseconds: 3600000 })
      : await host.run(project, staged)
    return { ...answer, seconds: Math.round((Date.now() - started) / 1000) }
  }

  static async main() {
    assert.ok(this.hosts[this.target], `no in-app driver for ${this.target} yet; the drivers are ${Object.keys(this.hosts).join(', ')}`)
    const plan = WasmFeaturesFixture.plan(this.target, this.checks, this.languages)
    assert.ok(plan.length, 'nothing to run: no check has a WebAssembly language on this target')
    const results = []
    for (const item of plan) {
      const label = `${this.target} ${item.check} ${item.language}`
      try {
        const answer = await this.check(item)
        console.log(`==> PASS ${label} (${answer.seconds} s, ${answer.modules.length} WebAssembly modules)\n${answer.lines.join('\n')}`)
        results.push({ ...item, status: 'pass', seconds: answer.seconds, lines: answer.lines })
      } catch (error) {
        console.log(`==> FAIL ${label}\n${String(error?.message ?? error).slice(-6000)}`)
        results.push({ ...item, status: 'fail', error: String(error?.message ?? error).slice(-6000) })
      }
    }
    if (this.receipt) {
      mkdirSync(path.dirname(path.resolve(this.receipt)), { recursive: true })
      writeFileSync(this.receipt, `${JSON.stringify({ target: this.target, results }, null, 2)}\n`)
    }
    console.table(results.map(({ check, language, status, seconds }) => ({ check, language, status, seconds })))
    if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(this.root, { recursive: true, force: true })
    const failed = results.filter((result) => result.status !== 'pass')
    assert.deepEqual(failed.map(({ check, language }) => `${check} ${language}`), [], 'every in-app WebAssembly feature check passed')
    console.log(`PASS: ${results.length} WebAssembly feature checks ran inside the packaged ${this.target} app: ${results.map(({ check, language }) => `${check} ${language}`).join(', ')}`)
  }
}

await WasmFeaturesAppTest.main()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { accessSync, constants, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { PageChecksFixture } from '../native/page-checks-fixture.mjs'

class LinuxPageChecksTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-page-checks-'))
  static application = PageChecksFixture.application
  static pageOrigin = 'tachyon://app'
  static stopRequest = PageChecksFixture.stopRequest
  static pollMilliseconds = 100
  static privateRuntimeFolder = null
  static firstLaunchSteps = PageChecksFixture.firstLaunchSteps('Quit')
  static secondLaunchSteps = PageChecksFixture.secondLaunchSteps

  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static async prepare(yonOrigin, undeclaredOrigin) {
    PageChecksFixture.writeProject(this.fixture, { yonOrigin, undeclaredOrigin, pageOrigin: this.pageOrigin })
    await this.run(this.binary, ['build', this.fixture, '--target', 'linux'], { timeoutMilliseconds: 240000 })
  }

  static checkBuild(yonOrigin, undeclaredOrigin) {
    const hostSource = path.join(this.fixture, 'dist/linux/project/tachyon_host.c')
    const bundle = path.join(this.fixture, 'dist/linux', this.application, 'resources/WebBundle')
    PageChecksFixture.checkBuild(hostSource, bundle, yonOrigin, undeclaredOrigin)
  }

  static writable(folder) {
    try {
      accessSync(folder, constants.W_OK)
      return true
    } catch {
      return false
    }
  }

  static runtimeFolder() {
    const current = process.env.XDG_RUNTIME_DIR
    if (current && this.writable(current)) return current
    console.log('==> using a private XDG_RUNTIME_DIR, because this shell has no writable one')
    this.privateRuntimeFolder = mkdtempSync(path.join(tmpdir(), 'ty-linux-runtime-'))
    return this.privateRuntimeFolder
  }

  static launchScript() {
    return `set -euo pipefail
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
timeout 300s python3 "$PROBE" "$FIRST_STEPS"
for attempt in $(seq 150); do kill -0 "$host" 2>/dev/null || break; sleep 0.2; done
if kill -0 "$host" 2>/dev/null; then echo "The app was still running 30 seconds after File > Quit" >&2; exit 1; fi
wait "$host"
echo "==> the app quit; starting it again"
"$APP" &
host=$!
timeout 120s python3 "$PROBE" "$SECOND_STEPS"`
  }

  static async startDrive() {
    const environment = { ...process.env }
    delete environment.WAYLAND_DISPLAY
    const env = {
      ...environment,
      APP: path.join(this.fixture, 'dist/linux', this.application, 'bin', this.application),
      PROBE: path.join(this.repository, 'scripts/linux/page-checks-probe.py'),
      FIRST_STEPS: JSON.stringify(this.firstLaunchSteps),
      SECOND_STEPS: JSON.stringify(this.secondLaunchSteps),
      TAC_PROBE_APPLICATION: this.application,
      TAC_PROBE_HANDSHAKE: path.join(this.fixture, 'handshake'),
      XDG_RUNTIME_DIR: this.runtimeFolder(),
      XDG_STATE_HOME: path.join(this.fixture, 'state'),
      XDG_DATA_HOME: path.join(this.fixture, 'data'),
      XDG_CACHE_HOME: path.join(this.fixture, 'cache'),
      GDK_BACKEND: 'x11',
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    mkdirSync(env.TAC_PROBE_HANDSHAKE, { recursive: true })
    const args = ['--auto-servernum', '--server-args=-screen 0 1024x1400x24', 'dbus-run-session', '--', 'bash', '-c', this.launchScript()]
    return await SupervisedCommand.start('xvfb-run', args, {
      cwd: this.repository, env, timeoutMilliseconds: 480000,
      onStdout: (chunk) => process.stdout.write(chunk), onStderr: (chunk) => process.stderr.write(chunk),
    })
  }

  static async waitForRequest(drive, name) {
    const asked = path.join(this.fixture, 'handshake', `${name}.asked`)
    while (!existsSync(asked)) {
      assert.ok(!drive.exited, `The probe ended before it asked to ${name}`)
      await sleep(this.pollMilliseconds)
    }
  }

  static answerRequest(name) {
    writeFileSync(path.join(this.fixture, 'handshake', `${name}.done`), 'done\n')
  }

  static async finish(drive) {
    const result = await drive.completion
    assert.ok(result.status === 0 && result.cleanupConfirmed, `The app probe failed (${result.status ?? result.signal})\n${result.error?.message ?? ''}`)
  }

  static async main() {
    const started = Date.now()
    const yonPort = await PageChecksFixture.freePort()
    const yonOrigin = `http://127.0.0.1:${yonPort}`
    const undeclared = await PageChecksFixture.undeclaredHost()
    let drive = null
    try {
      await this.prepare(yonOrigin, undeclared.origin)
      this.checkBuild(yonOrigin, undeclared.origin)
      await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
        await PageChecksFixture.checkYonAnswers(yon, this.pageOrigin)
        drive = await this.startDrive()
        await this.waitForRequest(drive, this.stopRequest)
        assert.deepEqual(undeclared.requests, [], `The undeclared host saw requests: ${undeclared.requests.join(', ')}`)
        console.log(`==> ${undeclared.origin} saw no request; stopping Yon`)
      })
      this.answerRequest(this.stopRequest)
      await this.finish(drive)
    } finally {
      await drive?.stop()
      await new Promise((resolve) => undeclared.server.close(resolve))
      if (this.privateRuntimeFolder) rmSync(this.privateRuntimeFolder, { recursive: true, force: true })
    }
    const seconds = Math.round((Date.now() - started) / 1000)
    console.log(`PASS: in the packaged Linux app (WebKitGTK), typing kept focus and caret through reactive writes; connect-src refused ${undeclared.origin} (it saw no request) while the Yon origin answered; a network-first Yon read answered from the cache with Yon stopped; a @SessionState field survived a navigation; a preference string survived a quit and restart (${seconds} s).`)
  }
}

await LinuxPageChecksTest.main()

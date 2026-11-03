#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { DevLoopFixture } from '../native/dev-loop-fixture.mjs'
import { LinuxDisplaySession } from './linux-display-session.mjs'

class LinuxDevLoopTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-dev-loop-'))
  static application = DevLoopFixture.application
  static display = process.env.TAC_LINUX_DISPLAY ?? 'x11'
  static sessionScript = `set -euo pipefail
"$TY" preview "$DEV_LOOP_PROJECT" --target linux --port "$PORT" >"$DEV_LOOP_LOG" 2>&1 &
preview=$!
trap 'kill "$preview" \${compositor:-} 2>/dev/null || true; wait 2>/dev/null || true; cat "$DEV_LOOP_LOG"' EXIT
timeout 600s python3 "$PROBE" "$STEPS"
kill -INT "$preview"
wait "$preview" || true
for _ in $(seq 40); do
  if ! pgrep -f "$APP" >/dev/null; then echo 'OK: stopping ty preview closed the app'; exit 0; fi
  sleep 0.25
done
echo "the app $APP still runs after ty preview stopped" >&2
exit 1`

  static async drive(port) {
    const log = path.join(this.fixture, 'preview.log')
    const env = {
      ...process.env,
      TY: this.binary,
      PORT: String(port),
      PROBE: path.join(this.repository, 'scripts/linux/dev-loop-probe.py'),
      STEPS: JSON.stringify(DevLoopFixture.steps),
      APP: path.join(this.fixture, 'dist/preview/linux', this.application, 'bin', this.application),
      DEV_LOOP_PROJECT: this.fixture,
      DEV_LOOP_LOG: log,
      TAC_PROBE_APPLICATION: this.application,
      XDG_STATE_HOME: path.join(this.fixture, 'state'),
      XDG_DATA_HOME: path.join(this.fixture, 'data'),
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const session = LinuxDisplaySession.of(this.display, this.sessionScript, '1024x1000', this.fixture)
    const result = await SupervisedCommand.run(session.command, session.args, { cwd: this.repository, env: { ...env, ...session.env }, timeoutMilliseconds: 720000 })
    process.stdout.write(result.stdout)
    assert.ok(result.status === 0 && result.cleanupConfirmed, `the dev loop session failed\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
  }

  static async main() {
    DevLoopFixture.write(this.fixture)
    const port = await DevLoopFixture.freePort()
    await this.drive(port)
    console.log(`PASS: ty preview --target linux (${this.display}) opened the app against http://127.0.0.1:${port}; its Yon calls reached the development server, a style edit arrived without a reload, a page edit reached the app, a manifest edit reopened it, and stopping ty preview closed it.`)
  }
}

await LinuxDevLoopTest.main()

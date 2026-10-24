#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'

class LinuxDevLoopTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-dev-loop-'))
  static application = 'DevLoop'
  static probeSteps = [
    ['wait', ['Dev loop ready', 'Count 0', 'Style one']],
    ['click', 'Add one'],
    ['wait', ['Count 1']],
    ['clickUntil', ['Ask Yon', 'Yon says hello']],
    ['edit', ['client/components/loop/panel/tac.css', 'Style one', 'Style two']],
    ['wait', ['Style two', 'Count 1']],
    ['edit', ['client/components/loop/panel/tac.html', 'Dev loop ready', 'Dev loop edited']],
    ['wait', ['Dev loop edited']],
    ['edit', ['client/manifest.json', '"Loop"', '"Loop Two"']],
    ['opened', 2],
    ['wait', ['Dev loop edited', 'Count 0', 'Style two']],
  ]

  static files = {
    'client/manifest.json': JSON.stringify({
      name: 'Dev Loop',
      short_name: 'Loop',
      start_url: '/',
      tac: {
        application: { id: 'dev.tachyon.dev-loop' },
        visibility: { check: false },
      },
    }, null, 2),
    'client/components/loop/panel/tac.css': 'main::after {\n  content: "Style one";\n}\n',
    'client/pages/tac.html': '<loop-panel hydrate="load" />\n',
    'client/components/loop/panel/tac.html': '<main>\n  <h1>Dev loop ready</h1>\n  <button on:click="counter.add()">Add one</button>\n  <p>{counter.label}</p>\n  <button on:click="greeting.ask()">Ask Yon</button>\n  <p>{greeting.text}</p>\n</main>\n',
    'client/models/Greeting.ts': 'export type Greeting = { text: string }\n',
    'client/remotes/GreetingClient.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@HTTPClient('/api/greeting')
export default class GreetingClient {
  /** @returns {Promise<Greeting>} */
  @GetRequest
  async read() {}
}
`,
    'client/services/CounterService.js': `@Service
export default class CounterService {
  taps = 0;
  label = 'Count 0';

  add() {
    this.taps += 1;
    this.label = \`Count \${this.taps}\`;
  }
}
`,
    'client/services/GreetingService.js': `@Service
export default class GreetingService {
  /** @type {GreetingClient} */
  @Needs
  greetingClient;
  text = '';

  async ask() {
    this.text = (await this.greetingClient.read()).text;
  }
}
`,
    'server/models/Greeting.ts': 'export type Greeting = { text: string }\n',
    'server/controllers/GreetingController.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/greeting")
@Public
class GreetingController {
  /** @returns {Greeting} */
  @GetRoute
  getGreeting() {
    return { text: 'Yon says hello' }
  }
}
`,
  }

  static write(name, text) {
    const target = path.join(this.fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
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

  static sessionScript = `set -euo pipefail
"$TY" preview "$DEV_LOOP_PROJECT" --target linux --port "$PORT" >"$DEV_LOOP_LOG" 2>&1 &
preview=$!
trap 'kill "$preview" 2>/dev/null || true; wait 2>/dev/null || true; cat "$DEV_LOOP_LOG"' EXIT
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
      STEPS: JSON.stringify(this.probeSteps),
      APP: path.join(this.fixture, 'dist/preview/linux', this.application, 'bin', this.application),
      DEV_LOOP_PROJECT: this.fixture,
      DEV_LOOP_LOG: log,
      TAC_PROBE_APPLICATION: this.application,
      XDG_STATE_HOME: path.join(this.fixture, 'state'),
      XDG_DATA_HOME: path.join(this.fixture, 'data'),
      GTK_A11Y: 'atspi',
      GDK_BACKEND: 'x11',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const args = ['--auto-servernum', '--server-args=-screen 0 1024x1000x24', 'dbus-run-session', '--', 'bash', '-c', this.sessionScript]
    const result = await SupervisedCommand.run('xvfb-run', args, { cwd: this.repository, env, timeoutMilliseconds: 720000 })
    process.stdout.write(result.stdout)
    assert.ok(result.status === 0 && result.cleanupConfirmed, `the dev loop session failed\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
  }

  static async main() {
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    const port = await this.freePort()
    await this.drive(port)
    console.log(`PASS: ty preview --target linux opened the app against http://127.0.0.1:${port}; its Yon calls reached the development server, a style edit arrived without a reload, a page edit reached the app, a manifest edit reopened it, and stopping ty preview closed it.`)
  }
}

await LinuxDevLoopTest.main()

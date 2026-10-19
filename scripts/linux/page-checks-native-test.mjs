#!/usr/bin/env node
import assert from 'node:assert/strict'
import { accessSync, constants, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer as createHttpServer } from 'node:http'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'

class LinuxPageChecksTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-page-checks-'))
  static application = 'PageChecks'
  static pageOrigin = 'tachyon://app'
  static stopRequest = 'stop-yon'
  static pollMilliseconds = 100
  static privateRuntimeFolder = null
  static firstLaunchSteps = [
    ['wait', ['Typed', 'Saved theme none', 'Visit note none']],
    ['focus', 'Typing box'],
    ['type', { entry: 'Typing box', keys: 'hello', shown: 'Typed ', pace: 'each' }],
    ['type', { entry: 'Typing box', keys: ' world', shown: 'Typed ', pace: 'burst' }],
    ['move', { entry: 'Typing box', keys: ['Left', 'Left'], caret: 9 }],
    ['type', { entry: 'Typing box', keys: 'xy', shown: 'Typed ', pace: 'each' }],
    ['click', 'Reach the undeclared host'],
    ['wait', ['Undeclared host refused by connect-src']],
    ['click', 'Reach the Yon origin'],
    ['wait', ['Yon origin reached 200']],
    ['click', 'Read the note'],
    ['wait', ['Read 1: Note from Yon']],
    ['ask', 'stop-yon'],
    ['click', 'Read the fresh note'],
    ['wait', ['Fresh read failed']],
    ['click', 'Read the note'],
    ['wait', ['Read 2: Note from Yon']],
    ['click', 'Save the dark theme'],
    ['wait', ['Saved theme dark']],
    ['click', 'Remember this visit'],
    ['wait', ['Visit note remembered']],
    ['click', 'Open the other page'],
    ['wait', ['The other page']],
    ['click', 'Back to the first page'],
    ['wait', ['Visit note remembered', 'Saved theme dark', 'Read the note']],
    ['quit', 'Quit'],
  ]
  static secondLaunchSteps = [
    ['wait', ['Saved theme dark', 'Visit note none', 'Read the note']],
  ]

  static clientFiles(addresses) {
    return {
      'client/pages/tac.html': '<typing-box hydrate="load" />\n<reach-panel hydrate="load" />\n<note-panel hydrate="load" />\n<theme-panel hydrate="load" />\n<visit-panel hydrate="load" />\n',
      'client/pages/other/tac.html': '<other-page />\n',
      'client/components/typing/box/tac.html': '<input aria-label="Typing box" :value="typing.text" on:input="typing.text = $event.target.value" />\n<p>Typed {typing.text}</p>\n',
      'client/components/reach/panel/tac.html': '<button on:click="reach.tryUndeclared()">Reach the undeclared host</button>\n<button on:click="reach.tryYon()">Reach the Yon origin</button>\n<p>{reach.undeclared}</p>\n<p>{reach.yon}</p>\n',
      'client/components/note/panel/tac.html': '<button on:click="note.read()">Read the note</button>\n<button on:click="note.readFresh()">Read the fresh note</button>\n<p>{note.text}</p>\n<p>{note.fresh}</p>\n',
      'client/components/theme/panel/tac.html': '<button on:click="theme.save()">Save the dark theme</button>\n<p>Saved theme {theme.saved}</p>\n',
      'client/components/visit/panel/tac.html': '<button on:click="visit.remember()">Remember this visit</button>\n<p>Visit note {visit.note}</p>\n<a href="/other/">Open the other page</a>\n',
      'client/components/other/page/tac.html': '<h1>The other page</h1>\n<a href="/">Back to the first page</a>\n',
      'client/services/TypingService.js': `@Service
export default class TypingService {
  text = '';
}
`,
      'client/setups/ReachSetup.js': `@Setup
export default class ReachSetup {
  static _ADDRESSES = ${JSON.stringify(addresses)};
  static _REPORT_WAIT = 2000;

  /** @returns {object} */
  static configure() {
    return { http: { reach: { exchange: (request) => ReachSetup._exchange(request) } } };
  }

  /**
   * @param {{ path: string }} request
   * @returns {Promise<{ outcome: string }>}
   */
  static async _exchange(request) {
    const report = ReachSetup._policyReport();
    try {
      const response = await fetch(ReachSetup._ADDRESSES[request.path], { credentials: 'omit', cache: 'no-store' });
      return { outcome: \`reached \${response.status}\` };
    } catch {
      return { outcome: \`refused by \${await report}\` };
    }
  }

  /** @returns {Promise<string>} */
  static _policyReport() {
    return new Promise((resolve) => {
      document.addEventListener('securitypolicyviolation', (event) => resolve(event.effectiveDirective), { once: true });
      setTimeout(() => resolve('no policy report'), ReachSetup._REPORT_WAIT);
    });
  }
}
`,
      'client/services/ReachService.js': `@Service
export default class ReachService {
  static ADAPTER = 'reach';
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {BehaviorContext | null} */
  behaviors = null;
  undeclared = 'Undeclared host not tried';
  yon = 'Yon origin not tried';

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(ReachSetup, { partition: 'reach', signal: this.lifecycle });
  }

  async tryUndeclared() {
    this.undeclared = \`Undeclared host \${await this.#reach('/undeclared')}\`;
  }

  async tryYon() {
    this.yon = \`Yon origin \${await this.#reach('/yon')}\`;
  }

  /**
   * @param {string} target
   * @returns {Promise<string>}
   */
  async #reach(target) {
    return this.behaviors.restClient(ReachService.ADAPTER).get(target).retrieve().then((answer) => answer.outcome, (error) => \`failed: \${error.message}\`);
  }
}
`,
      'client/remotes/NoteClient.js': `@HTTPClient
@CacheSettings({"name":"notes","ttlMs":1,"maxBytes":4096,"maxEntries":4})
export default class NoteClient {
  /** @returns {Promise<{ text: string }>} */
  @Cached
  @GetRequest("/api/notes")
  async read() {}

  /** @returns {Promise<{ text: string }>} */
  @GetRequest("/api/fresh")
  async fresh() {}
}
`,
      'client/services/NoteService.js': `@Service
export default class NoteService {
  /** @type {NoteClient} */
  @Needs
  noteClient;
  reads = 0;
  text = 'No read yet';
  fresh = 'No fresh read yet';

  async read() {
    this.reads += 1;
    const label = \`Read \${this.reads}\`;
    this.text = await this.noteClient.read().then((note) => \`\${label}: \${note.text}\`, () => \`\${label} failed\`);
  }

  async readFresh() {
    this.fresh = await this.noteClient.fresh().then((note) => \`Fresh read: \${note.text}\`, () => 'Fresh read failed');
  }
}
`,
      'client/services/ThemeService.js': `@Service
export default class ThemeService {
  static KEY = 'theme';
  static DARK = 'dark';
  /** @type {PreferenceStrings} */
  @Needs
  preferenceStrings;
  saved = 'none';

  @OnStart
  load() {
    this.saved = this.#shown(this.preferenceStrings.read(ThemeService.KEY));
  }

  save() {
    this.preferenceStrings.write(ThemeService.KEY, ThemeService.DARK);
    this.saved = this.#shown(this.preferenceStrings.read(ThemeService.KEY));
  }

  /**
   * @param {string | null} value
   * @returns {string}
   */
  #shown(value) {
    return value ?? 'none';
  }
}
`,
      'client/services/VisitService.js': `@Service
export default class VisitService {
  @SessionState
  note = 'none';

  remember() {
    this.note = 'remembered';
  }
}
`,
    }
  }

  static serverFiles = {
    'server/controllers/NoteController.js': `@RequestRoute("/api/notes")
@Public
class NoteController {
  @GetRoute
  getNote() {
    return Response.json({ text: 'Note from Yon' }, { headers: { 'cache-control': 'public, no-cache' } })
  }
}
`,
    'server/controllers/FreshController.js': `@RequestRoute("/api/fresh")
@Public
class FreshController {
  @GetRoute
  getFresh() {
    return { text: 'Fresh from Yon' }
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

  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static async prepare(yonOrigin, undeclaredOrigin) {
    const addresses = { '/undeclared': `${undeclaredOrigin}/probe`, '/yon': `${yonOrigin}/api/fresh` }
    for (const [name, text] of Object.entries({ ...this.clientFiles(addresses), ...this.serverFiles })) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({
      name: 'Page Checks',
      start_url: '/',
      tac: {
        application: { id: 'dev.tachyon.page-checks' },
        visibility: { check: false },
        yon: { origin: yonOrigin },
        cache: [{ path: '/api/notes', policy: 'network-first' }],
        browserApis: ['preferenceStrings'],
      },
    }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [this.pageOrigin] } }))
    await this.run(this.binary, ['build', this.fixture, '--target', 'linux'], { timeoutMilliseconds: 240000 })
  }

  static checkBuild(yonOrigin, undeclaredOrigin) {
    const source = readFileSync(path.join(this.fixture, 'dist/linux/project/tachyon_host.c'), 'utf8')
    const policy = source.match(/connect-src [^;]*/)?.[0] ?? ''
    assert.ok(policy.includes(yonOrigin), `connect-src lacks the Yon origin ${yonOrigin}: ${policy}`)
    assert.ok(!policy.includes(undeclaredOrigin), `connect-src lists the undeclared ${undeclaredOrigin}: ${policy}`)
    const bundle = path.join(this.fixture, 'dist/linux', this.application, 'resources/WebBundle')
    assert.ok(existsSync(path.join(bundle, '.tachyon/tac-yon-reads.js')), 'the bundle has no .tachyon/tac-yon-reads.js')
    assert.match(readFileSync(path.join(bundle, 'index.html'), 'utf8'), /tachyon-browser-apis" content="preferenceStrings"/)
  }

  static async undeclaredHost() {
    const port = await this.freePort()
    const host = { origin: `http://127.0.0.1:${port}`, requests: [] }
    host.server = createHttpServer((request, response) => {
      host.requests.push(`${request.method} ${request.url} from ${request.headers.origin ?? 'no origin'}`)
      response.writeHead(200, { 'content-type': 'application/json', 'access-control-allow-origin': '*' })
      response.end('{"reached":true}')
    })
    await new Promise((resolve) => host.server.listen(port, '127.0.0.1', resolve))
    return host
  }

  static async checkYonAnswers(yon) {
    const note = await yon('/api/notes', { headers: { origin: this.pageOrigin } })
    assert.equal(note.headers.get('access-control-allow-origin'), this.pageOrigin)
    assert.equal(note.headers.get('cache-control'), 'public, no-cache')
    assert.deepEqual(await note.json(), { text: 'Note from Yon' })
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
    const yonPort = await this.freePort()
    const yonOrigin = `http://127.0.0.1:${yonPort}`
    const undeclared = await this.undeclaredHost()
    let drive = null
    try {
      await this.prepare(yonOrigin, undeclared.origin)
      this.checkBuild(yonOrigin, undeclared.origin)
      await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
        await this.checkYonAnswers(yon)
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

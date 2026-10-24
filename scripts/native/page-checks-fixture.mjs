import assert from 'node:assert/strict'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { createServer as createHttpServer } from 'node:http'
import { createServer } from 'node:net'
import path from 'node:path'

export class PageChecksFixture {
  static application = 'PageChecks'
  static stopRequest = 'stop-yon'

  /** @param {string} quit */
  static firstLaunchSteps(quit) {
    return [
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
      ['ask', PageChecksFixture.stopRequest],
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
      ['quit', quit],
    ]
  }

  static secondLaunchSteps = [
    ['wait', ['Saved theme dark', 'Visit note none', 'Read the note']],
  ]

  /** @param {Record<string, string>} addresses */
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

  /** @param {string} fixture @param {string} name @param {string} text */
  static write(fixture, name, text) {
    const target = path.join(fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  /** @param {string} fixture @param {{ yonOrigin: string, undeclaredOrigin: string, pageOrigin: string }} origins */
  static writeProject(fixture, { yonOrigin, undeclaredOrigin, pageOrigin }) {
    const addresses = { '/undeclared': `${undeclaredOrigin}/probe`, '/yon': `${yonOrigin}/api/fresh` }
    for (const [name, text] of Object.entries({ ...this.clientFiles(addresses), ...this.serverFiles })) this.write(fixture, name, text)
    this.write(fixture, 'client/manifest.json', JSON.stringify({
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
    this.write(fixture, 'server/application.json', JSON.stringify({ server: { origins: [pageOrigin] } }))
  }

  /** @param {string} hostSource @param {string} bundle @param {string} yonOrigin @param {string} undeclaredOrigin */
  static checkBuild(hostSource, bundle, yonOrigin, undeclaredOrigin) {
    const source = readFileSync(hostSource, 'utf8')
    const policy = source.match(/connect-src [^;]*/)?.[0] ?? ''
    assert.ok(policy.includes(yonOrigin), `connect-src lacks the Yon origin ${yonOrigin}: ${policy}`)
    assert.ok(!policy.includes(undeclaredOrigin), `connect-src lists the undeclared ${undeclaredOrigin}: ${policy}`)
    assert.ok(existsSync(path.join(bundle, '.tachyon/tac-yon-reads.js')), 'the bundle has no .tachyon/tac-yon-reads.js')
    assert.match(readFileSync(path.join(bundle, 'index.html'), 'utf8'), /tachyon-browser-apis" content="preferenceStrings"/)
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

  /** @param {(path: string, options?: RequestInit) => Promise<Response>} yon @param {string} pageOrigin */
  static async checkYonAnswers(yon, pageOrigin) {
    const note = await yon('/api/notes', { headers: { origin: pageOrigin } })
    assert.equal(note.headers.get('access-control-allow-origin'), pageOrigin)
    assert.equal(note.headers.get('cache-control'), 'public, no-cache')
    assert.deepEqual(await note.json(), { text: 'Note from Yon' })
  }
}

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer as createHttpServer, request as httpRequest } from 'node:http'
import { createServer } from 'node:net'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { PageChecksFixture } from './page-checks-fixture.mjs'

assert.equal(process.platform, 'darwin', 'The macOS page checks need macOS.')

class MacOsPageChecksTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'macos-page-checks-'))
  static probe = path.join(MacOsPageChecksTest.fixture, 'page-checks-macos-probe')
  static application = 'PageChecks'
  static identifier = 'dev.tachyon.page-checks-macos'
  static pageOrigin = 'tachyon-app://bundle'
  static onlineSteps = [
    ['wait', 'Saved theme none', 'No read yet', 'No first read yet', 'Visit note none', 'Kept note none', 'Root count 0'],
    ['press', 'Reach the undeclared host'],
    ['wait', 'Undeclared host refused by connect-src'],
    ['press', 'Reach the Yon origin'],
    ['wait', 'Yon origin reached 200'],
    ['press', 'Read the note'],
    ['wait', 'Read 1: Note from Yon'],
    ['press', 'Read the note'],
    ['wait', 'Read 2: Note from Yon'],
    ['press', 'Read the first note'],
    ['wait', 'First read 1: First note from Yon'],
    ['press', 'Read the first note'],
    ['wait', 'First read 2: First note from Yon'],
  ]
  static offlineSteps = [
    ['press', 'Read the fresh note'],
    ['wait', 'Fresh read failed'],
    ['press', 'Read the note'],
    ['wait', 'Read 3: Note from Yon'],
    ['press', 'Read the first note'],
    ['wait', 'First read 3: First note from Yon'],
    ['press', 'Save the dark theme'],
    ['wait', 'Saved theme dark'],
  ]
  static visitSteps = [
    ['press', 'Remember this visit'],
    ['wait', 'Visit note remembered'],
    ['press', 'Open the other page'],
    ['wait', 'The other page'],
    ['press', 'Back to the first page'],
    ['wait', 'Visit note remembered', 'Saved theme dark', 'No read yet'],
  ]
  static routeSteps = [
    ['press', 'Set the root count'],
    ['wait', 'Root count 7', 'Root doubled 14'],
    ['press', 'Open item 7'],
    ['wait', 'Item path /items/7/', 'Item count 0'],
    ['press', 'Set the item count'],
    ['wait', 'Item count 9', 'Item doubled 18'],
    ['press', 'Back to the root'],
    ['wait', 'Root count 0', 'Visit note remembered'],
    ['press', 'Set the root count'],
    ['wait', 'Root count 7', 'Root doubled 14'],
  ]
  static keptSteps = [
    ['press', 'Read the note'],
    ['wait', 'Read 1: Note from Yon'],
    ['press', 'Keep a note in Swift'],
    ['wait', 'Kept note kept by Swift'],
    ['press', 'Reload the page'],
    ['wait', 'Kept note kept by Swift', 'No read yet'],
    ['quit'],
  ]
  static relaunchSteps = [
    ['wait', 'Saved theme dark', 'No read yet', 'Visit note none', 'Kept note kept by Swift', 'Root count 0'],
    ['quit'],
  ]

  static clientFiles(addresses) {
    return {
      'client/pages/tac.html': '<reach-panel hydrate="load" />\n<note-panel hydrate="load" />\n<theme-panel hydrate="load" />\n<visit-panel hydrate="load" />\n<keep-panel hydrate="load" />\n<root-panel hydrate="load" />\n',
      'client/components/note/panel/tac.html': '<button on:click="note.read()">Read the note</button>\n<button on:click="note.readFirst()">Read the first note</button>\n<button on:click="note.readFresh()">Read the fresh note</button>\n<p>{note.text}</p>\n<p>{note.first}</p>\n<p>{note.fresh}</p>\n',
      'client/components/theme/panel/tac.html': '<button on:click="theme.save()">Save the dark theme</button>\n<p>Saved theme {theme.saved}</p>\n',
      ...PageChecksFixture.reachFiles(addresses),
      ...PageChecksFixture.visitFiles,
      ...this.routeFiles,
      ...this.keptFiles,
      'client/remotes/NoteClient.js': `@HTTPClient
@CacheSettings({"name":"notes","ttlMs":1,"maxBytes":4096,"maxEntries":4})
export default class NoteClient {
  /** @returns {Promise<{ text: string }>} */
  @Cached
  @GetRequest("/api/notes")
  async read() {}

  /** @returns {Promise<{ text: string }>} */
  @Cached
  @GetRequest("/api/first")
  async first() {}

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
  firstReads = 0;
  text = 'No read yet';
  first = 'No first read yet';
  fresh = 'No fresh read yet';

  async read() {
    this.reads += 1;
    const label = \`Read \${this.reads}\`;
    this.text = await this.noteClient.read().then((note) => \`\${label}: \${note.text}\`, () => \`\${label} failed\`);
  }

  async readFirst() {
    this.firstReads += 1;
    const label = \`First read \${this.firstReads}\`;
    this.first = await this.noteClient.first().then((note) => \`\${label}: \${note.text}\`, () => \`\${label} failed\`);
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
    }
  }

  static routeFiles = {
    'client/pages/items/_id/tac.html': '<item-panel hydrate="load" />\n',
    'client/components/root/panel/tac.html': '<button on:click="rootRoute.set()">Set the root count</button>\n<p>Root count {rootRoute.count}</p>\n<p>Root doubled {rootRoute.twice}</p>\n<a href="/items/7/">Open item 7</a>\n',
    'client/components/item/panel/tac.html': '<h1>Item path {itemRoute.path}</h1>\n<button on:click="itemRoute.set()">Set the item count</button>\n<p>Item count {itemRoute.count}</p>\n<p>Item doubled {itemRoute.twice}</p>\n<a href="/">Back to the root</a>\n',
    'client/services/RootRouteService.js': `@Service
export default class RootRouteService {
  static COUNT = 7;
  count = 0;
  twice = 'not asked';

  async set() {
    this.count = RootRouteService.COUNT;
    this.twice = String(await this.doubled());
  }
}
`,
    'client/services/RootRouteService.swift': `import Foundation
@Service
final class RootRouteService {
    var count: Int = 0

    func doubled() -> Int {
        count * 2
    }
}
`,
    'client/services/ItemRouteService.js': `@Service
export default class ItemRouteService {
  static COUNT = 9;
  count = 0;
  twice = 'not asked';
  path = '';

  @OnStart
  open() {
    this.path = location.pathname;
  }

  async set() {
    this.count = ItemRouteService.COUNT;
    this.twice = String(await this.doubled());
  }
}
`,
    'client/services/ItemRouteService.swift': `import Foundation
@Service
final class ItemRouteService {
    var count: Int = 0

    func doubled() -> Int {
        count * 2
    }
}
`,
  }

  static keptFiles = {
    'client/components/keep/panel/tac.html': '<button on:click="keep.keepNote()">Keep a note in Swift</button>\n<button on:click="keep.reload()">Reload the page</button>\n<p>Kept note {keep.saved}</p>\n',
    'client/services/KeepService.js': `@Service
export default class KeepService {
  @LocalState
  saved = 'none';

  async keepNote() {
    await this.remember();
  }

  reload() {
    location.reload();
  }
}
`,
    'client/services/KeepService.swift': `import Foundation
@Service
final class KeepService {
    var saved: String = "none"

    func remember() -> String {
        saved = "kept by Swift"
        return saved
    }
}
`,
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
    'server/controllers/FirstController.js': `@RequestRoute("/api/first")
@Public
class FirstController {
  @GetRoute
  getFirst() {
    return Response.json({ text: 'First note from Yon' }, { headers: { 'cache-control': 'public, no-cache' } })
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

  static run(program, args) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout: 600_000 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')} failed:\n${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static prepare(yonOrigin, undeclaredOrigin) {
    const addresses = { '/undeclared': `${undeclaredOrigin}/probe`, '/yon': `${yonOrigin}/api/fresh` }
    for (const [name, text] of Object.entries({ ...this.clientFiles(addresses), ...this.serverFiles })) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({
      name: this.application,
      start_url: '/',
      tac: {
        application: { id: this.identifier, version: '0.0.1' },
        visibility: { check: false },
        yon: { origin: yonOrigin },
        cache: [{ path: '/api/notes', policy: 'network-first' }, { path: '/api/first', policy: 'cache-first' }],
        browserApis: ['preferenceStrings'],
      },
    }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [this.pageOrigin] } }))
    this.run(this.binary, ['bundle', this.fixture, '--target', 'macos'])
    this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/native/page-checks-macos-probe.swift'), '-o', this.probe])
  }

  static app() {
    return path.join(this.fixture, 'dist/macos', `${this.application}.app`)
  }

  static checkBuild(yonOrigin, undeclaredOrigin) {
    const resources = path.join(this.app(), 'Contents/Resources')
    const host = readFileSync(path.join(this.fixture, 'dist/macos/project/TachyonHost.swift'), 'utf8')
    const policy = host.match(/connect-src [^;]*/)?.[0] ?? ''
    assert.ok(policy.includes(yonOrigin), `connect-src lacks the Yon origin ${yonOrigin}: ${policy}`)
    assert.ok(!policy.includes(undeclaredOrigin), `connect-src lists the undeclared ${undeclaredOrigin}: ${policy}`)
    const reads = this.run('/usr/bin/find', [resources, '-path', '*/.tachyon/tac-yon-reads.js']).trim()
    assert.ok(reads, 'the app bundle has no .tachyon/tac-yon-reads.js')
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

  static async countingFront(yonPort) {
    const port = await this.freePort()
    const front = { origin: `http://127.0.0.1:${port}`, reads: {}, sockets: new Set() }
    front.server = createHttpServer((incoming, outgoing) => {
      const route = new URL(incoming.url, front.origin).pathname
      if (incoming.method === 'GET') front.reads[route] = (front.reads[route] ?? 0) + 1
      const headers = { ...incoming.headers, host: `127.0.0.1:${yonPort}` }
      const forwarded = httpRequest({ host: '127.0.0.1', port: yonPort, method: incoming.method, path: incoming.url, headers }, (answer) => {
        outgoing.writeHead(answer.statusCode ?? 502, answer.headers)
        answer.pipe(outgoing)
      })
      forwarded.on('error', () => outgoing.destroy())
      incoming.pipe(forwarded)
    })
    front.server.on('connection', (socket) => {
      front.sockets.add(socket)
      socket.on('close', () => front.sockets.delete(socket))
    })
    await new Promise((resolve) => front.server.listen(port, '127.0.0.1', resolve))
    return front
  }

  static async stopFront(front) {
    const closed = new Promise((resolve) => front.server.close(resolve))
    for (const socket of front.sockets) socket.destroy()
    await closed
  }

  static websiteData() {
    const library = path.join(homedir(), 'Library')
    return [path.join(library, 'WebKit', this.identifier), path.join(library, 'Caches', this.identifier),
      path.join(library, 'HTTPStorages', this.identifier), path.join(library, 'Saved Application State', `${this.identifier}.savedState`)]
  }

  static forgetWebsiteData() {
    for (const folder of this.websiteData()) rmSync(folder, { recursive: true, force: true })
  }

  static launch() {
    this.run('/usr/bin/open', ['-n', this.app()])
  }

  static async drive(steps) {
    for (const [action, ...values] of steps) {
      await promisify(execFile)(this.probe, [this.identifier, action, ...values], { timeout: 120_000 })
      console.log(`ok - ${action} ${values.join(' | ')}`)
    }
  }

  static async main() {
    const started = Date.now()
    const yonPort = await this.freePort()
    const front = await this.countingFront(yonPort)
    const undeclared = await this.undeclaredHost()
    this.forgetWebsiteData()
    try {
      this.prepare(front.origin, undeclared.origin)
      this.checkBuild(front.origin, undeclared.origin)
      await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async () => {
        this.launch()
        await this.drive(this.onlineSteps)
        await this.stopFront(front)
      })
      assert.deepEqual(undeclared.requests, [], `The undeclared host saw requests: ${undeclared.requests.join(', ')}`)
      const reads = { notes: front.reads['/api/notes'], first: front.reads['/api/first'] }
      assert.deepEqual(reads, { notes: 2, first: 1 }, 'network-first reads go to Yon each time; the second cache-first read stays in the page')
      console.log(`==> Yon read ${JSON.stringify(reads)} by Yon; Yon stopped; ${undeclared.origin} saw no request`)
      await this.drive(this.offlineSteps)
      await this.drive(this.visitSteps)
      await this.drive(this.routeSteps)
      await this.drive(this.keptSteps)
      console.log('==> the app quit; starting it again')
      this.launch()
      await this.drive(this.relaunchSteps)
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(this.app(), 'Contents/MacOS')])
      await new Promise((resolve) => undeclared.server.close(resolve))
      if (front.server.listening) await this.stopFront(front)
      this.forgetWebsiteData()
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1' && existsSync(this.fixture)) rmSync(this.fixture, { recursive: true, force: true })
    }
    const seconds = Math.round((Date.now() - started) / 1000)
    console.log(`PASS: in the packaged macOS app (WKWebView), connect-src refused ${undeclared.origin} (it saw no request) while the Yon origin answered; Yon answered both network-first reads and only the first cache-first read; with Yon stopped, both reads answered from the page's kept copies while a no-store read failed; a @SessionState field was kept across a navigation to another page and back; the _param route /items/7/ opened with its own native state (root 7, item 9, root 7 again from a fresh owner); a @LocalState field set by a Swift method was read after a reload; a preference string and the Swift-set field survived a quit and restart while the @SessionState field did not (${seconds} s).`)
  }
}

await MacOsPageChecksTest.main()

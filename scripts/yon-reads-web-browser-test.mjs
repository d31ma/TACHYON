#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'

class YonReadsWebTest {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'yon-reads-web-'))
  static browsers = [['chromium', chromium], ['firefox', firefox], ['webkit', webkit]]
  static selected = (process.env.TAC_YON_READS_BROWSERS ?? 'chromium,firefox,webkit').split(',')
  static timeout = 15000
  static files = {
    'client/pages/tac.html': '<note-panel hydrate="load" />\n',
    'client/components/note/panel/tac.html': '<button on:click="note.load()">Load note</button>\n<button on:click="note.loadSecret()">Load secret</button>\n<p id="note">{note.text}</p>\n<p id="secret">{note.secret}</p>\n',
    'client/models/Note.ts': 'export type Note = { text: string }\n',
    'client/remotes/NoteClient.js': `/** @import { Note } from '/models/Note.ts' */
@HTTPClient('/api')
@CacheSettings({"name":"notes","ttlMs":1,"maxBytes":4096,"maxEntries":8})
export default class NoteClient {
  /** @returns {Promise<Note>} */
  @Cached
  @GetRequest("/notes")
  async read() {}

  /** @returns {Promise<Note>} */
  @Cached
  @GetRequest("/secret")
  async readSecret() {}
}
`,
    'client/services/NoteService.js': `@Service
export default class NoteService {
  /** @type {NoteClient} */
  @Needs
  noteClient;
  reads = 0;
  text = '';
  secret = '';

  async load() {
    this.reads += 1;
    this.text = 'Read ' + this.reads + ': ' + (await this.noteClient.read()).text;
  }

  async loadSecret() {
    this.secret = await this.#secretOrRefusal();
  }

  /** @returns {Promise<string>} */
  async #secretOrRefusal() {
    try {
      return (await this.noteClient.readSecret()).text;
    } catch {
      return 'Secret refused offline';
    }
  }
}
`,
    'server/models/Note.ts': 'export type Note = { text: string }\n',
    'server/controllers/NoteController.js': `/** @import { Note } from '/models/Note.ts' */
@RequestRoute("/api/notes")
@Public
class NoteController {
  /** @returns {Response} */
  @GetRoute
  getNote() {
    return Response.json({ text: 'Note from Yon' }, { headers: { 'cache-control': 'public, no-cache' } })
  }
}
`,
    'server/controllers/SecretController.js': `/** @import { Note } from '/models/Note.ts' */
@RequestRoute("/api/secret")
@Public
class SecretController {
  /** @returns {Note} */
  @GetRoute
  getSecret() {
    return { text: 'Secret from Yon' }
  }
}
`,
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

  static write(name, text) {
    const target = path.join(this.fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static prepare(pageOrigin, yonOrigin) {
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({
      tac: { visibility: { check: false }, yon: { origin: yonOrigin }, cache: [{ path: '/api/*', policy: 'network-first' }] },
    }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [pageOrigin] } }))
    const built = spawnSync(this.binary, ['bundle', this.fixture], { encoding: 'utf8' })
    assert.equal(built.status, 0, `${built.stdout}${built.stderr}`)
  }

  static async kept(page) {
    return page.evaluate(async () => (await (await caches.open('tachyon-yon-reads')).keys()).map((request) => new URL(request.url).pathname))
  }

  static async open(name, type, url) {
    const browser = await type.launch({ headless: true })
    const context = await browser.newContext()
    const page = await context.newPage()
    const errors = []
    page.on('pageerror', (error) => errors.push(error.message))
    await page.goto(url)
    await page.waitForFunction(() => globalThis[Symbol.for('tachyon.tac.yon-reads')], null, { timeout: this.timeout })
    return { name, browser, context, page, errors }
  }

  static shows(page, selector, text) {
    return page.waitForFunction(([target, expected]) => document.querySelector(target)?.textContent === expected, [selector, text], { timeout: this.timeout })
  }

  static async readOnline({ page }) {
    await page.getByRole('button', { name: 'Load note' }).click()
    await this.shows(page, '#note', 'Read 1: Note from Yon')
    await page.getByRole('button', { name: 'Load secret' }).click()
    await this.shows(page, '#secret', 'Secret from Yon')
  }

  static async readOffline({ name, context, page, errors }) {
    assert.deepEqual(await this.kept(page), ['/api/notes'], `${name}: only the public read is kept`)
    await context.setOffline(true)
    await page.waitForTimeout(50)
    await page.getByRole('button', { name: 'Load note' }).click()
    await this.shows(page, '#note', 'Read 2: Note from Yon')
    await page.getByRole('button', { name: 'Load secret' }).click()
    await this.shows(page, '#secret', 'Secret refused offline')
    assert.deepEqual(errors, [], name)
  }

  static async run() {
    const pagePort = await this.freePort()
    const yonPort = await this.freePort()
    const pageOrigin = `http://127.0.0.1:${pagePort}`
    this.prepare(pageOrigin, `http://127.0.0.1:${yonPort}`)
    const opened = []
    try {
      await withServer(this.binary, ['preview', this.fixture, '--static', '--port', String(pagePort)], async (pages) => {
        const response = await pages('/')
        assert.equal(response.status, 200)
        const html = await response.text()
        assert.match(html, /\/\.tachyon\/tac-yon-reads\.js/, 'the web page loads the Yon read cache')
        assert.match(html, /\/\.tachyon\/register-sw\.js/, 'the web page still registers its worker')
        for (const [name, type] of this.browsers.filter(([name]) => this.selected.includes(name))) opened.push(await this.open(name, type, response.url))
        await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
          const answer = await yon('/api/notes', { headers: { origin: pageOrigin } })
          assert.equal(answer.headers.get('access-control-allow-origin'), pageOrigin)
          assert.equal(answer.headers.get('cache-control'), 'public, no-cache')
          for (const browser of opened) await this.readOnline(browser)
          for (const browser of opened) await this.readOffline(browser)
        })
      })
    } finally {
      for (const { browser } of opened) await browser.close()
    }
    console.log(`Yon reads on the web passed: a web page on one port read Yon on another, went offline while Yon still ran, and answered the public read from its kept copy while the no-store read failed in ${opened.map(({ name }) => name).join(', ')}.`)
  }
}

await YonReadsWebTest.run()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'

class YonOriginTest {
  static repository = fileURLToPath(new URL('..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'yon-origin-'))
  static features = ['sockets', 'realtime']
  static timeout = 15000
  static files = {
    'client/pages/tac.html': '<chat-room hydrate="load" />\n<ticks-stream hydrate="load" />\n<note-panel hydrate="load" />\n',
    'client/models/Note.ts': 'export type Note = { text: string }\n',
    'client/components/note/panel/tac.html': '<button on:click="note.load()">Load note</button>\n<button on:click="note.save()">Save note</button>\n<p id="note">{note.text}</p>\n',
    'client/remotes/NoteClient.js': `/** @import { Note } from '/models/Note.ts' */
@HTTPClient('/api/notes')
export default class NoteClient {
  /** @returns {Promise<Note>} */
  @GetRequest
  async read() {}

  /**
   * @param {Note} note
   * @returns {Promise<Note>}
   */
  @PostRequest
  async write(@RequestBody note) {}
}
`,
    'client/services/NoteService.js': `/** @import { Note } from '/models/Note.ts' */
@Service
export default class NoteService {
  /** @type {NoteClient} */
  @Needs
  noteClient;
  text = '';

  async load() {
    this.text = (await this.noteClient.read()).text;
  }

  async save() {
    this.text = (await this.noteClient.write({ text: 'from another origin' })).text;
  }
}
`,
    'server/controllers/NoteController.js': `/** @import { Note } from '/models/Note.ts' */
@RequestRoute("/api/notes")
@Public
class NoteController {
  /** @returns {Note} */
  @GetRoute
  getNote() {
    return { text: 'Note from Yon' }
  }

  /**
   * @param {Note} note
   * @returns {Note}
   */
  @PostRoute
  postNote(@FromBody note) {
    return { text: \`Saved \${note.text}\` }
  }
}
`,
    'server/models/Note.ts': 'export type Note = { text: string }\n',
  }

  static write(name, text) {
    const target = path.join(this.fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static copyFeature(name) {
    const source = path.join(this.repository, 'website/content/features', name)
    const feature = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
    for (const file of feature.files) {
      const selected = file.name.endsWith('.js') || file.name.endsWith('.html') || file.name.includes('/models/') && file.name.endsWith('.ts')
      if (selected && file.name !== 'client/pages/tac.html') this.write(file.name, readFileSync(path.join(source, file.name), 'utf8'))
    }
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

  static prepare(pagePort, yonPort) {
    for (const feature of this.features) this.copyFeature(feature)
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({ tac: { visibility: { check: false }, yon: { origin: `http://127.0.0.1:${yonPort}` } } }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [`http://127.0.0.1:${pagePort}`] } }))
    const built = spawnSync(this.binary, ['bundle', this.fixture], { encoding: 'utf8' })
    assert.equal(built.status, 0, `${built.stdout}${built.stderr}`)
  }

  static async checkCrossOriginAnswers(yon, pageOrigin) {
    const listed = await yon('/api/notes', { headers: { origin: pageOrigin } })
    assert.equal(listed.headers.get('access-control-allow-origin'), pageOrigin)
    const unlisted = await yon('/api/notes', { headers: { origin: 'https://evil.example' } })
    assert.equal(unlisted.headers.get('access-control-allow-origin'), null)
    const preflight = await yon('/api/notes', { method: 'OPTIONS', headers: { origin: pageOrigin, 'access-control-request-method': 'POST', 'access-control-request-headers': 'content-type' } })
    assert.equal(preflight.status, 204)
    assert.equal(preflight.headers.get('access-control-allow-headers'), 'content-type')
  }

  static async checkPage(url) {
    const browser = await chromium.launch({ headless: true })
    try {
      const page = await browser.newPage()
      const errors = []
      page.on('pageerror', (error) => errors.push(error.message))
      await page.goto(url)
      const shows = (text) => page.waitForFunction((expected) => document.body.textContent.includes(expected), text, { timeout: this.timeout })
      await shows('Welcome to the room')
      await page.getByRole('button', { name: 'Say hello' }).click()
      await shows('Hello from a tab')
      await shows('ANVIL-1 · 3')
      await page.getByRole('button', { name: 'Load note' }).click()
      await shows('Note from Yon')
      await page.getByRole('button', { name: 'Save note' }).click()
      await shows('Saved from another origin')
      assert.deepEqual(errors, [])
    } finally {
      await browser.close()
    }
  }

  static async run() {
    const pagePort = await this.freePort()
    const yonPort = await this.freePort()
    this.prepare(pagePort, yonPort)
    const pageOrigin = `http://127.0.0.1:${pagePort}`
    await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
      await this.checkCrossOriginAnswers(yon, pageOrigin)
      await withServer(this.binary, ['preview', this.fixture, '--static', '--port', String(pagePort)], async (pages) => {
        const response = await pages('/')
        assert.equal(response.status, 200)
        assert.match(response.headers.get('content-security-policy') ?? '', new RegExp(`connect-src [^;]*http://127\\.0\\.0\\.1:${yonPort} ws://127\\.0\\.0\\.1:${yonPort}`))
        await this.checkPage(response.url)
      })
    })
    console.log('Yon origin passed: a page on one port reached Yon on another over HTTP, a CORS preflight, a stream and a socket in Chromium.')
  }
}

await YonOriginTest.run()

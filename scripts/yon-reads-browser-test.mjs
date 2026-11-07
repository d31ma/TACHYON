#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { createServer as createHttpServer } from 'node:http'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'

class YonReadsTest {
  static repository = fileURLToPath(new URL('..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'yon-reads-'))
  static browsers = [['chromium', chromium], ['firefox', firefox], ['webkit', webkit]]
  static selected = (process.env.TAC_YON_READS_BROWSERS ?? 'chromium,firefox,webkit').split(',')
  static rules = JSON.stringify([{ pattern: '^/api/[^?]*$', policy: 'network-first' }])
  static files = {
    'client/pages/tac.html': '<page-notes />\n',
    'client/components/page/notes/tac.html': '<main>Notes</main>\n',
    'server/controllers/NoteController.js': `@RequestRoute("/api/notes")
@Public
class NoteController {
  @GetRoute
  getNote() {
    return Response.json({ text: 'Note from Yon' }, { headers: { 'cache-control': 'public, no-cache' } })
  }
}
`,
    'server/controllers/SecretController.js': `@RequestRoute("/api/secret")
@Public
class SecretController {
  @GetRoute
  getSecret() {
    return { text: 'Secret from Yon' }
  }
}
`,
  }

  static source(relative) {
    return readFileSync(path.join(this.repository, 'crates/tachyon-core/src', relative), 'utf8')
  }

  static scripts() {
    return {
      '/tac-http.js': this.source('template/tac-http.js'),
      '/client-runtime.js': `${this.source('behavior/client_runtime.js')}\nglobalThis.exchange = __tachyon_client_exchange__\n`,
      '/tac-yon-reads.js': this.source('template/tac-yon-reads.js').replace('__CACHE_RULES__', this.rules),
    }
  }

  static document(yonOrigin) {
    const tags = Object.keys(this.scripts()).map((name) => `<script type="module" src="${name}"></script>`).join('')
    return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="tachyon-yon-origin" content="${yonOrigin}">${tags}<title>Yon reads</title></head><body></body></html>`
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

  static pageServer(yonOrigin) {
    const scripts = this.scripts()
    const document = this.document(yonOrigin)
    return createHttpServer((request, response) => {
      const script = scripts[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html', 'cache-control': 'no-store' })
      response.end(script ?? document)
    })
  }

  static async read(page, path) {
    return page.evaluate(async (target) => {
      try {
        return (await globalThis.exchange({ path: target, method: 'GET', query: {}, headers: {}, publicRead: true, maxResponseBytes: 1024 })).text
      } catch (error) { return `refused: ${error.name}` }
    }, path)
  }

  static async kept(page) {
    return page.evaluate(async () => (await (await caches.open('tachyon-yon-reads')).keys()).map((request) => new URL(request.url).pathname))
  }

  static async run() {
    const pagePort = await this.freePort()
    const yonPort = await this.freePort()
    const pageOrigin = `http://127.0.0.1:${pagePort}`
    const yonOrigin = `http://127.0.0.1:${yonPort}`
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({ tac: { visibility: { check: false } } }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [pageOrigin] } }))
    const pages = this.pageServer(yonOrigin)
    await new Promise((resolve) => pages.listen(pagePort, '127.0.0.1', resolve))
    const opened = []
    try {
      for (const [name, type] of this.browsers.filter(([name]) => this.selected.includes(name))) {
        const browser = await type.launch({ headless: true })
        const page = await browser.newPage()
        await page.goto(`${pageOrigin}/`)
        await page.waitForFunction(() => typeof globalThis.exchange === 'function' && globalThis[Symbol.for('tachyon.tac.yon-reads')])
        opened.push({ name, browser, page })
      }
      await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
        const answer = await yon('/api/notes', { headers: { origin: pageOrigin } })
        assert.equal(answer.headers.get('access-control-allow-origin'), pageOrigin)
        assert.match(answer.headers.get('vary') ?? '', /origin/i)
        assert.equal(answer.headers.get('cache-control'), 'public, no-cache')
        for (const { name, page } of opened) {
          assert.equal(await this.read(page, '/api/notes'), 'Note from Yon', name)
          assert.equal(await this.read(page, '/api/secret'), 'Secret from Yon', name)
        }
      })
      for (const { name, page } of opened) {
        assert.equal(await this.read(page, '/api/notes'), 'Note from Yon', `${name}: a network-first read answers from the cache when Yon is gone`)
        assert.match(await this.read(page, '/api/secret'), /^refused:/, `${name}: a no-store answer is never kept`)
        assert.deepEqual(await this.kept(page), ['/api/notes'], name)
      }
    } finally {
      for (const { browser } of opened) await browser.close()
      await new Promise((resolve) => pages.close(resolve))
    }
    console.log(`Yon reads passed: a page reached Yon on another origin, kept the public read and answered it with Yon stopped in ${opened.map(({ name }) => name).join(', ')}.`)
  }
}

await YonReadsTest.run()

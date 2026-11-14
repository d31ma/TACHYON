#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class BlobsBrowserCheck {
  static template = new URL('../crates/tachyon-core/src/template/', import.meta.url)

  static bundles = {
    '/tac-files.js': ['tac-random-id.js', 'tac-sha256.js', 'tac-transfer.js', 'tac-files.js'],
    '/tac-blobs.js': ['tac-random-id.js', 'tac-sha256.js', 'tac-transfer.js', 'tac-blobs.js'],
    '/tac-web.js': ['tac-web.js'],
  }

  static engines = [chromium, firefox, webkit]

  static pixel = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64')

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="tachyon-browser-apis" content="blobs files">
    <meta name="tachyon-blobs" content="200000 150000">
    <meta name="tachyon-outgoing-hosts" content="">
    <title>Blobs</title></head><body>
    <button id="pick">Pick</button>
    <img id="shown" alt="kept attachment">
    <script src="/tac-web.js"></script>
    <script type="module" src="/tac-files.js"></script>
    <script type="module" src="/tac-blobs.js"></script>
    <script>
      window.sent = []
      TacWebApis.events = { publish: () => {}, send: (owner, name, value) => window.sent.push([name, value]) }
      const web = globalThis[Symbol.for('tachyon.tac.web')]
      window.ask = (api, call, args = []) => JSON.parse(web.calling('check', () => web.host(JSON.stringify({ api, call, args }))))
      document.getElementById('pick').addEventListener('click', () => window.ask('files', 'pick', ['', false]))
    </script>
  </body></html>`

  static uploads = []

  static answer(request, response) {
    const routes = {
      '/attachments/photo.png': () => [200, 'image/png', BlobsBrowserCheck.pixel],
      '/attachments/large': () => [200, 'application/octet-stream', Buffer.alloc(150001, 7)],
      '/attachments/half': () => [200, 'application/octet-stream', Buffer.alloc(100000, 8)],
      '/attachments/missing': () => [404, 'text/plain', 'no'],
    }
    const script = BlobsBrowserCheck.files[request.url]
    const [status, type, body] = script ? [200, 'text/javascript', script] : (routes[request.url] ?? (() => [200, 'text/html', BlobsBrowserCheck.page]))()
    response.writeHead(status, { 'content-type': type })
    response.end(body)
  }

  static async run() {
    BlobsBrowserCheck.files = Object.fromEntries(await Promise.all(Object.entries(BlobsBrowserCheck.bundles).map(async ([path, names]) => [path, (await Promise.all(names.map((name) => readFile(new URL(name, BlobsBrowserCheck.template), 'utf8')))).join('\n')])))
    const server = createServer((request, response) => {
      if (request.method !== 'PUT') return BlobsBrowserCheck.answer(request, response)
      const parts = []
      request.on('data', (part) => parts.push(part))
      request.on('end', () => {
        BlobsBrowserCheck.uploads.push(Buffer.concat(parts))
        response.writeHead(201, { 'content-type': 'text/plain' })
        response.end('stored')
      })
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const engine of BlobsBrowserCheck.engines) await BlobsBrowserCheck.runEngine(engine, url)
    } finally {
      server.close()
    }
  }

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage()
      await page.goto(url)
      await page.waitForFunction(() => Boolean(globalThis[Symbol.for('tachyon.tac.blobs')]))
      await BlobsBrowserCheck.keep(page)
      await BlobsBrowserCheck.show(page)
      await BlobsBrowserCheck.limits(page)
      await page.reload()
      await page.waitForFunction(() => Boolean(globalThis[Symbol.for('tachyon.tac.blobs')]))
      await BlobsBrowserCheck.afterReload(page)
      console.log(`blobs (${engine.name()}): saveHeld, download, list, read, open in an img, upload, size and quota refusals, kept across a reload, remove`)
    } finally {
      await browser.close()
    }
  }

  static async taken(page, count = 1) {
    await page.waitForFunction((wanted) => window.sent.length >= wanted, count, { timeout: 20000 })
    return page.evaluate(() => window.sent.splice(0))
  }

  static async keep(page) {
    const chooser = page.waitForEvent('filechooser')
    await page.click('#pick')
    await (await chooser).setFiles({ name: 'note.txt', mimeType: 'text/plain', buffer: Buffer.from('hello blobs') })
    const [[picked, file]] = await BlobsBrowserCheck.taken(page)
    assert.equal(picked, 'FilePicked')
    await page.evaluate((id) => window.ask('blobs', 'saveHeld', [id, 'mail/1/note']), file.id)
    const [[saved, note]] = await BlobsBrowserCheck.taken(page)
    assert.deepEqual([saved, note.key, note.bytes, note.media], ['BlobSaved', 'mail/1/note', 11, 'text/plain'])
    assert.equal(note.sha256, createHash('sha256').update('hello blobs').digest('hex'))
    await page.evaluate(() => window.ask('blobs', 'download', ['/attachments/photo.png', 'mail/1/photo', '']))
    const [[downloaded, photo]] = await BlobsBrowserCheck.taken(page)
    assert.deepEqual([downloaded, photo.bytes, photo.media], ['BlobSaved', BlobsBrowserCheck.pixel.length, 'image/png'])
    await page.evaluate(() => window.ask('blobs', 'read', ['mail/1/note', 6, 5]))
    const [[read, chunk]] = await BlobsBrowserCheck.taken(page)
    assert.deepEqual([read, Buffer.from(chunk.data, 'base64').toString(), chunk.size], ['BlobChunkRead', 'blobs', 11])
  }

  static async show(page) {
    await page.evaluate(() => window.ask('blobs', 'open', ['mail/1/photo']))
    const [[opened, shown]] = await BlobsBrowserCheck.taken(page)
    assert.equal(opened, 'BlobOpened')
    assert.ok(shown.url.startsWith('blob:'), shown.url)
    assert.equal((await page.evaluate(() => window.ask('blobs', 'url', ['mail/1/photo']))).value, shown.url)
    await page.evaluate((source) => { document.getElementById('shown').src = source }, shown.url)
    await page.waitForFunction(() => document.getElementById('shown').complete && document.getElementById('shown').naturalWidth === 1)
    await page.evaluate(() => window.ask('blobs', 'upload', ['mail/1/note', 'PUT', '/uploads/note', 'content-type: text/plain']))
    const [[uploaded, answer]] = await BlobsBrowserCheck.taken(page)
    assert.deepEqual([uploaded, answer.key, answer.status, answer.body, answer.error], ['BlobUploaded', 'mail/1/note', 201, 'stored', ''])
    assert.equal(BlobsBrowserCheck.uploads.pop().toString(), 'hello blobs')
  }

  static async limits(page) {
    await page.evaluate(() => window.ask('blobs', 'download', ['/attachments/large', 'mail/2/large', '']))
    assert.deepEqual((await BlobsBrowserCheck.taken(page)).map(([name, value]) => [name, value.reason]), [['BlobFailed', 'size']])
    await page.evaluate(() => window.ask('blobs', 'download', ['/attachments/half', 'mail/2/half', '']))
    assert.equal((await BlobsBrowserCheck.taken(page))[0][0], 'BlobSaved')
    await page.evaluate(() => window.ask('blobs', 'download', ['/attachments/half', 'mail/2/again', '']))
    assert.deepEqual((await BlobsBrowserCheck.taken(page)).map(([name, value]) => [name, value.reason]), [['BlobFailed', 'quota']])
    await page.evaluate(() => window.ask('blobs', 'download', ['/attachments/missing', 'mail/2/missing', '']))
    assert.deepEqual((await BlobsBrowserCheck.taken(page)).map(([name, value]) => [name, value.reason]), [['BlobFailed', 'status']])
  }

  static async afterReload(page) {
    await page.evaluate(() => window.ask('blobs', 'list', ['mail/']))
    const listed = await BlobsBrowserCheck.taken(page, 4)
    assert.deepEqual(listed.map(([name, value]) => [name, value.key ?? value.prefix]), [['BlobFound', 'mail/1/note'], ['BlobFound', 'mail/1/photo'], ['BlobFound', 'mail/2/half'], ['BlobsListed', 'mail/']])
    const used = 11 + BlobsBrowserCheck.pixel.length + 100000
    assert.deepEqual(listed[3][1], { prefix: 'mail/', count: 3, bytes: used, usedBytes: used, maxBytes: 200000 })
    for (const key of ['mail/1/note', 'mail/1/photo', 'mail/2/half']) await page.evaluate((name) => window.ask('blobs', 'remove', [name]), key)
    assert.deepEqual((await BlobsBrowserCheck.taken(page, 3)).map(([name]) => name), ['BlobRemoved', 'BlobRemoved', 'BlobRemoved'])
    await page.evaluate(() => window.ask('blobs', 'list', ['']))
    assert.deepEqual(await BlobsBrowserCheck.taken(page), [['BlobsListed', { prefix: '', count: 0, bytes: 0, usedBytes: 0, maxBytes: 200000 }]])
  }
}

await BlobsBrowserCheck.run()

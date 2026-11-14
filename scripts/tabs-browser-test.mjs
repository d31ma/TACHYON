#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class TabsBrowserCheck {
  static sources = ['tac-web.js', 'tac-storage.js', 'tac-locks.js', 'tac-client.js'].map((name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url))

  static engines = [chromium, firefox, webkit]

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="tachyon-browser-apis" content="locks">
    <title>Tabs</title></head><body>
    <script src="/tac-web.js"></script>
    <script src="/tac-storage.js"></script>
    <script src="/tac-locks.js"></script>
    <script>
      window.published = []
      window.sent = []
      window.rendered = 0
      addEventListener('tachyon:rerender', () => { window.rendered += 1 })
      TacWebApis.events = { publish: (name, value) => window.published.push([name, value]), send: (owner, name, value) => window.sent.push([owner, name, value]) }
      const web = globalThis[Symbol.for('tachyon.tac.web')]
      const storage = globalThis[Symbol.for('tachyon.tac.storage')]
      window.lock = (owner, call, name) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'locks', call, args: [name] }))))
      window.entity = (command) => JSON.parse(web.calling('store', () => storage.host(JSON.stringify({ op: 'entity', command }))))
      window.note = (operation, extra = {}) => window.entity({ operation, table: 'notes', idField: 'id', entity: 'Note', ...extra })
      window.message = (operation, extra = {}) => window.entity({ operation, table: 'messages', idField: 'id', entity: 'Message', partition: 'room', ...extra })
      window.everything = { size: 50, offset: 0, sort: [] }
      window.settled = () => storage.settled()
      window.mirrored = storage.mirror()
      window.stored = fetch('/tac-client.js').then((answer) => answer.text()).then((client) => {
        const slice = client.slice(client.indexOf('const webStorage = '), client.indexOf('const persistScope = '))
        const { Owners, bindPersistentFields } = new Function(slice + '\\nreturn { Owners, bindPersistentFields }')()
        const owner = { visits: 0, draft: '' }
        Owners.stored.set(owner, { visits: 'local', draft: 'session' })
        window.draft = bindPersistentFields(owner, 'service:draft:/')
      })
    </script>
  </body></html>`

  static async run() {
    const files = Object.fromEntries(await Promise.all(TabsBrowserCheck.sources.map(async (source) => [`/${source.pathname.split('/').pop()}`, await readFile(source, 'utf8')])))
    const server = createServer((request, response) => {
      const script = files[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? TabsBrowserCheck.page)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const engine of TabsBrowserCheck.engines) await TabsBrowserCheck.runEngine(engine, url)
    } finally {
      server.close()
    }
  }

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const context = await browser.newContext()
      const first = await TabsBrowserCheck.opened(context, url)
      const second = await TabsBrowserCheck.opened(context, url)
      await TabsBrowserCheck.storeRows(first, second)
      await TabsBrowserCheck.partitions(first, second)
      await TabsBrowserCheck.localState(first, second)
      await TabsBrowserCheck.locks(context, url, first, second)
      console.log(`tabs (${engine.name()}): Store rows and a loaded partition reach the other tab with StoreChanged, @LocalState follows the other tab and @SessionState does not, and one tab at a time holds a lock until it releases it or closes`)
    } finally {
      await browser.close()
    }
  }

  static async opened(context, url) {
    const page = await context.newPage()
    await page.goto(url)
    await page.evaluate(() => Promise.all([window.mirrored, window.stored]))
    return page
  }

  static async until(page, test, argument) {
    await page.waitForFunction(test, argument, { timeout: 10000 })
  }

  static async storeRows(first, second) {
    assert.deepEqual(await second.evaluate(() => window.note('find', { page: window.everything })), { value: [] })
    assert.deepEqual(await first.evaluate(() => window.note('save', { value: { id: 'n1', text: 'from the first tab' } })).then((answer) => answer.value.id), 'n1')
    await TabsBrowserCheck.until(second, () => window.published.some(([name]) => name === 'StoreChanged'))
    assert.deepEqual(await second.evaluate(() => window.published.splice(0)), [['StoreChanged', { table: 'notes', partition: '' }]])
    assert.deepEqual(await second.evaluate(() => window.note('find', { page: window.everything })), { value: [{ id: 'n1', text: 'from the first tab' }] })
    await second.evaluate(() => window.note('delete', { id: 'n1' }))
    await TabsBrowserCheck.until(first, () => window.published.length > 0)
    assert.deepEqual(await first.evaluate(() => window.published.splice(0)), [['StoreChanged', { table: 'notes', partition: '' }]])
    assert.deepEqual(await first.evaluate(() => window.note('find', { page: window.everything })), { value: [] })
  }

  static async partitions(first, second) {
    const query = { kind: 'find', where: [{ field: 'room', operator: 'eq', value: 'r1' }], orderBy: [], page: { size: 50, offset: 0, sort: [] } }
    await first.evaluate(() => window.message('save', { value: { id: 'm1', room: 'r1', text: 'first' } }))
    await first.evaluate(() => window.settled())
    await second.waitForTimeout(300)
    await second.evaluate(() => window.message('load', { value: 'r1' }))
    await TabsBrowserCheck.until(second, () => window.sent.some(([, name]) => name === 'StorePartitionLoaded'))
    assert.deepEqual(await second.evaluate(() => window.sent.splice(0)), [['store', 'StorePartitionLoaded', { table: 'messages', partition: 'r1', rows: 1 }]])
    await second.waitForTimeout(300)
    await second.evaluate(() => window.published.splice(0))
    await first.evaluate(() => window.message('save', { value: { id: 'm2', room: 'r1', text: 'second' } }))
    await first.evaluate(() => window.message('save', { value: { id: 'm3', room: 'r2', text: 'other room' } }))
    await TabsBrowserCheck.until(second, () => window.published.length >= 2)
    assert.deepEqual(await second.evaluate(() => window.published.splice(0)), [['StoreChanged', { table: 'messages', partition: 'r1' }], ['StoreChanged', { table: 'messages', partition: 'r2' }]])
    const read = await second.evaluate((wanted) => window.message('query', { query: wanted }), query)
    assert.deepEqual(read.value.map((row) => row.id), ['m1', 'm2'])
    const cold = await second.evaluate(() => window.message('query', { query: { kind: 'count', where: [{ field: 'room', operator: 'eq', value: 'r2' }], orderBy: [], page: { size: 1, offset: 0, sort: [] } } }))
    assert.equal(cold.code, 'storage_not_loaded', 'a partition this tab never loaded stays on disk')
  }

  static async localState(first, second) {
    await second.evaluate(() => { window.rendered = 0 })
    await first.evaluate(() => { window.draft.visits = 7; window.draft.draft = 'only here' })
    await TabsBrowserCheck.until(second, () => window.draft.visits === 7)
    assert.deepEqual(await second.evaluate(() => [window.draft.visits, window.draft.draft, window.rendered > 0]), [7, '', true])
    assert.deepEqual(await first.evaluate(() => [window.draft.visits, window.draft.draft]), [7, 'only here'])
  }

  static async locks(context, url, first, second) {
    assert.deepEqual(await first.evaluate(() => window.lock('sender', 'request', 'outbox')), { value: null })
    await TabsBrowserCheck.until(first, () => window.sent.length > 0)
    assert.deepEqual(await first.evaluate(() => window.sent.splice(0)), [['sender', 'LockGranted', { name: 'outbox' }]])
    await second.evaluate(() => window.lock('waiter', 'request', 'outbox'))
    await second.waitForTimeout(500)
    assert.deepEqual(await second.evaluate(() => window.sent.splice(0)), [], 'the second tab waits while the first holds the lock')
    await first.evaluate(() => window.lock('sender', 'release', 'outbox'))
    await TabsBrowserCheck.until(second, () => window.sent.length > 0)
    assert.deepEqual(await second.evaluate(() => window.sent.splice(0)), [['waiter', 'LockGranted', { name: 'outbox' }]])
    await first.evaluate(() => window.lock('quitter', 'request', 'outbox'))
    await first.waitForTimeout(300)
    await first.evaluate(() => window.lock('quitter', 'release', 'outbox'))
    await second.close()
    await first.waitForTimeout(500)
    assert.deepEqual(await first.evaluate(() => window.sent.splice(0)), [], 'a request released while it waits sends nothing')
    const third = await TabsBrowserCheck.opened(context, url)
    await third.evaluate(() => window.lock('next', 'request', 'outbox'))
    await TabsBrowserCheck.until(third, () => window.sent.length > 0)
    assert.deepEqual(await third.evaluate(() => window.sent.splice(0)), [['next', 'LockGranted', { name: 'outbox' }]], 'closing a tab frees its locks')
  }
}

await TabsBrowserCheck.run()

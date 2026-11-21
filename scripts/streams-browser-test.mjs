#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class StreamsBrowserCheck {
  static template = new URL('../crates/tachyon-core/src/template/application-events.js', import.meta.url)

  static report = new URL('../crates/tachyon-core/src/template/tac-stream-report.js', import.meta.url)

  static query = new URL('../crates/tachyon-core/src/template/tac-stream-query.js', import.meta.url)

  static engines = [chromium, firefox, webkit]

  static models = { JobState: { open: false, fields: { running: '!integer' } } }

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Streams</title></head><body>
    <script src="/application-events.js"></script>
    <script src="/tac-stream-report.js"></script>
    <script src="/tac-stream-query.js"></script>
    <script>
      window.received = []
      window.heard = []
      window.failures = 0
      const bus = new TacApplicationEvents({ changed: () => {}, failed: (error, path, event, data) => { window.failures++; TacStreamReport.failed(error, path, event, data) } })
      const resources = { view: { id: 'owner' }, lifecycle: null, applicationEventPublisher: null }
      const models = ${JSON.stringify(StreamsBrowserCheck.models)}
      const client = { streamState: (state) => window.received.push(state.running) }
      const listener = { heard: (state) => window.heard.push(state.running) }
      bus.attach(listener, { listeners: [{ method: 'heard', events: ['JobState'], models }] }, resources)
      bus.attach(client, { streams: [{ method: 'streamState', path: '/.tachyon/reconnect/3/100/1000/0/.tachyon/event/state/jobs/state', event: 'JobState', models }] }, resources)
      bus.initialize(client)
      window.windowed = []
      const windowedClient = { streamStats: (stats) => window.windowed.push(stats.running) }
      bus.attach(windowedClient, { streams: [{ method: 'streamStats', path: '/.tachyon/event/state/.tachyon/query/StatsWindow/stats/{window}', event: 'JobState', models }] }, resources)
      bus.initialize(windowedClient)
      window.choose = (value) => bus.publish('StatsWindow', { window: value, limit: 5 })
      window.large = []
      window.openLarge = () => {
        const largeClient = { streamLarge: (state) => window.large.push(state.running) }
        bus.attach(largeClient, { streams: [{ method: 'streamLarge', path: '/.tachyon/event/stats/live/stats', event: 'JobState', models }] }, resources)
        bus.initialize(largeClient)
      }
    </script>
  </body></html>`

  static frames = [
    ': ping\n\n',
    'data: {"running": 99}\n\n',
    'event: heartbeat\ndata:\n\n',
    'event: state\ndata: {"running": 2}\n\n',
    'event: other\ndata: {"running": 98}\n\n',
    'event: state\ndata: {"running": 3}\n\n',
  ]

  static largeData = `{"running": 5, "note": "${'x'.repeat(81500)}"}`

  static largeFrames = [
    `event: stats\ndata: ${StreamsBrowserCheck.largeData}\n\n`,
    `event: stats\ndata: ${StreamsBrowserCheck.largeData}\n\n`,
    'event: stats\ndata: {"running": 7}\n\n',
  ]

  static opened = []

  static windows = []

  static closed = []

  static answer(request, response, scripts) {
    if (request.url === '/application-events.js') {
      response.writeHead(200, { 'content-type': 'text/javascript' })
      response.end(`${scripts.bus}\nwindow.TacApplicationEvents = TacApplicationEvents\n`)
      return
    }
    if (request.url === '/tac-stream-report.js') {
      response.writeHead(200, { 'content-type': 'text/javascript' })
      response.end(`${scripts.report}\nwindow.TacStreamReport = TacStreamReport\n`)
      return
    }
    if (request.url === '/tac-stream-query.js') {
      response.writeHead(200, { 'content-type': 'text/javascript' })
      response.end(`${scripts.query}\nwindow.TacStreamQuery = TacStreamQuery\n`)
      return
    }
    if (request.url === '/live/stats') {
      response.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-store' })
      for (const frame of StreamsBrowserCheck.largeFrames) response.write(frame)
      return
    }
    if (request.url.startsWith('/stats/')) {
      StreamsBrowserCheck.windows.push(request.url)
      const running = { '/stats/1h?limit=5': 1, '/stats/24%20h?limit=5': 24 }[request.url] ?? 0
      response.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-store' })
      response.write(`event: state\ndata: {"running": ${running}}\n\n`)
      request.on('close', () => StreamsBrowserCheck.closed.push(request.url))
      return
    }
    if (request.url.startsWith('/jobs/state')) {
      StreamsBrowserCheck.opened.push(request.url)
      response.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-store' })
      for (const frame of StreamsBrowserCheck.frames) response.write(frame)
      return
    }
    response.writeHead(200, { 'content-type': 'text/html' })
    response.end(StreamsBrowserCheck.page)
  }

  static async run() {
    const scripts = { bus: await readFile(StreamsBrowserCheck.template, 'utf8'), report: await readFile(StreamsBrowserCheck.report, 'utf8'), query: await readFile(StreamsBrowserCheck.query, 'utf8') }
    const server = createServer((request, response) => StreamsBrowserCheck.answer(request, response, scripts))
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const engine of StreamsBrowserCheck.engines) await StreamsBrowserCheck.runEngine(engine, url)
    } finally {
      server.closeAllConnections()
      server.close()
    }
  }

  static async until(done) {
    for (let wait = 0; wait < 100 && !done(); wait += 1) await new Promise((resolve) => setTimeout(resolve, 50))
  }

  static async checkLargeMessage(page, errors, name) {
    const failuresBefore = await page.evaluate(() => window.failures)
    await page.evaluate(() => window.openLarge())
    await page.waitForFunction(() => window.large.length >= 1, null, { timeout: 10000 })
    const size = Buffer.byteLength(StreamsBrowserCheck.largeData)
    const reports = errors.filter((text) => text.includes('/live/stats'))
    assert.equal(reports.length, 1, `${name}: two messages over 64 KiB on one stream give exactly one console error`)
    assert.match(reports[0], /stream \/live\/stats\./, `${name}: the report names the stream path`)
    assert.match(reports[0], /'stats' event/, `${name}: the report names the event`)
    assert.ok(reports[0].includes(`${size} bytes`), `${name}: the report gives the message size`)
    assert.ok(reports[0].includes('limit is 65536 bytes'), `${name}: the report gives the limit`)
    const large = await page.evaluate(() => window.large)
    assert.equal(large.includes(5), false, `${name}: a message over 64 KiB is not delivered, so the page keeps its old data`)
    assert.deepEqual(large, [7], `${name}: a later message that fits still arrives`)
    assert.equal(await page.evaluate(() => window.failures), failuresBefore + 2, `${name}: each dropped message still counts as a failure`)
  }

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage()
      const errors = []
      page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
      StreamsBrowserCheck.opened.length = 0
      StreamsBrowserCheck.windows.length = 0
      StreamsBrowserCheck.closed.length = 0
      await page.goto(url)
      await page.waitForFunction(() => window.received.length >= 2, null, { timeout: 10000 })
      const name = engine.name()
      assert.deepEqual(await page.evaluate(() => window.received), [2, 3], `${name}: only messages of the named event type reach the method`)
      assert.deepEqual(await page.evaluate(() => window.heard), [2, 3], `${name}: only messages of the named event type are published`)
      assert.equal(await page.evaluate(() => window.failures), 0, `${name}: comments, unnamed messages, other types and heartbeats are no failures`)
      assert.deepEqual(StreamsBrowserCheck.opened, ['/jobs/state'], `${name}: the stream opens once at its server path`)
      assert.deepEqual(StreamsBrowserCheck.windows, [], `${name}: a stream with a query model waits for its first value`)
      await page.evaluate(() => window.choose('1h'))
      await page.waitForFunction(() => window.windowed.length >= 1, null, { timeout: 10000 })
      await page.evaluate(() => window.choose('24 h'))
      await page.waitForFunction(() => window.windowed.length >= 2, null, { timeout: 10000 })
      await StreamsBrowserCheck.until(() => StreamsBrowserCheck.closed.includes('/stats/1h?limit=5'))
      assert.deepEqual(StreamsBrowserCheck.windows, ['/stats/1h?limit=5', '/stats/24%20h?limit=5'], `${name}: each new value opens the stream with its parameters`)
      assert.deepEqual(StreamsBrowserCheck.closed, ['/stats/1h?limit=5'], `${name}: the old stream is closed when the parameters change`)
      assert.deepEqual(await page.evaluate(() => window.windowed), [1, 24], `${name}: each stream delivers its own messages once`)
      await StreamsBrowserCheck.checkLargeMessage(page, errors, name)
    } finally {
      await browser.close()
    }
  }
}

await StreamsBrowserCheck.run()
console.log('Streams browser check passed in Chromium, Firefox and WebKit: named events, heartbeats, a parameter change and one report for messages over 64 KiB.')

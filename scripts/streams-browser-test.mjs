#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class StreamsBrowserCheck {
  static template = new URL('../crates/tachyon-core/src/template/application-events.js', import.meta.url)

  static engines = [chromium, firefox, webkit]

  static models = { JobState: { open: false, fields: { running: '!integer' } } }

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Streams</title></head><body>
    <script src="/application-events.js"></script>
    <script>
      window.received = []
      window.heard = []
      window.failures = 0
      const bus = new TacApplicationEvents({ changed: () => {}, failed: () => window.failures++ })
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

  static opened = []

  static windows = []

  static closed = []

  static answer(request, response, script) {
    if (request.url === '/application-events.js') {
      response.writeHead(200, { 'content-type': 'text/javascript' })
      response.end(`${script}\nwindow.TacApplicationEvents = TacApplicationEvents\n`)
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
    const script = await readFile(StreamsBrowserCheck.template, 'utf8')
    const server = createServer((request, response) => StreamsBrowserCheck.answer(request, response, script))
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

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage()
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
    } finally {
      await browser.close()
    }
  }
}

await StreamsBrowserCheck.run()
console.log('Streams browser check passed in Chromium, Firefox and WebKit: named events, heartbeats and a parameter change.')

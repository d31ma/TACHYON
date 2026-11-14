#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class CodeScannerBrowserCheck {
  static template = new URL('../crates/tachyon-core/src/template/', import.meta.url)
  static sources = { '/tac-web.js': 'tac-web.js', '/tac-code-scanner.js': 'tac-code-scanner.js', '/tac-qr-decoder.js': 'vendor/jsQR.js' }
  static fixture = new URL('../crates/tachyon-core/tests/fixtures/code-scanner/qr.txt', import.meta.url)
  static text = 'tachyon-link:7Q2f-9wXk'
  static engines = [chromium, firefox, webkit]

  static page = (rows) => `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="tachyon-browser-apis" content="camera">
    <meta name="tachyon-camera-reason" content="Scan the code on your other device">
    <title>Code scanner</title>
    <script>
      window.rows = ${JSON.stringify(rows)}
      window.camera = { answer: 'code', streams: [], asked: 0 }
      window.camera.ended = () => window.camera.streams.filter((stream) => stream.getTracks().every((track) => track.readyState === 'ended')).length
      const picture = (showCode) => {
        const canvas = document.createElement('canvas')
        const scale = 10
        canvas.width = 640
        canvas.height = 480
        const context = canvas.getContext('2d')
        const draw = () => {
          context.fillStyle = '#fff'
          context.fillRect(0, 0, canvas.width, canvas.height)
          if (!showCode) return
          context.fillStyle = '#000'
          const left = (canvas.width - window.rows.length * scale) / 2
          const top = (canvas.height - window.rows.length * scale) / 2
          window.rows.forEach((row, y) => [...row].forEach((cell, x) => { if (cell === '#') context.fillRect(left + x * scale, top + y * scale, scale, scale) }))
        }
        draw()
        setInterval(draw, 100)
        return canvas.captureStream(10)
      }
      MediaDevices.prototype.enumerateDevices = async () => [{ kind: 'videoinput', deviceId: 'canvas', label: '', groupId: '' }]
      MediaDevices.prototype.getUserMedia = async () => {
        window.camera.asked += 1
        if (window.camera.answer !== 'code' && window.camera.answer !== 'blank') throw new DOMException('refused', window.camera.answer)
        const stream = picture(window.camera.answer === 'code')
        window.camera.streams.push(stream)
        return stream
      }
    </script></head><body>
    <button id="go">Link a device</button>
    <script src="/tac-web.js"></script>
    <script type="module" src="/tac-code-scanner.js"></script>
    <script>
      window.sent = []
      TacWebApis.events = { publish: () => {}, send: (owner, name, value) => window.sent.push([owner, name, value]) }
      const web = globalThis[Symbol.for('tachyon.tac.web')]
      window.ask = (owner, call) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'codeScanner', call, args: [] }))))
      window.owner = 'page'
      window.answered = null
      document.getElementById('go').addEventListener('click', () => { window.answered = window.ask(window.owner, 'scan') })
    </script>
  </body></html>`

  static async run() {
    const files = {}
    for (const [path, name] of Object.entries(CodeScannerBrowserCheck.sources)) files[path] = await readFile(new URL(name, CodeScannerBrowserCheck.template), 'utf8')
    const rows = (await readFile(CodeScannerBrowserCheck.fixture, 'utf8')).trim().split('\n')
    const html = CodeScannerBrowserCheck.page(rows)
    const server = createServer((request, response) => {
      const script = files[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? html)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const engine of CodeScannerBrowserCheck.engines) await CodeScannerBrowserCheck.runEngine(engine, url)
    } finally {
      server.close()
    }
  }

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage({ viewport: { width: 800, height: 600 } })
      await page.goto(url)
      await page.waitForFunction(() => globalThis[Symbol.for('tachyon.tac.codeScanner')] !== undefined)
      const detector = await CodeScannerBrowserCheck.detector(page)
      await CodeScannerBrowserCheck.scans(page, 'first')
      await CodeScannerBrowserCheck.view(page)
      await CodeScannerBrowserCheck.refusals(page)
      const forced = detector ? await CodeScannerBrowserCheck.withoutDetector(page) : ''
      console.log(`code scanner (${engine.name()}): read the QR code with ${detector ? 'BarcodeDetector' : 'jsQR'}${forced}; the drawn view, Cancel, Escape, the user-action, grant and refused-camera answers, and the camera stopped after each scan`)
    } finally {
      await browser.close()
    }
  }

  static async detector(page) {
    return page.evaluate(async () => typeof BarcodeDetector === 'function' && (await BarcodeDetector.getSupportedFormats()).includes('qr_code'))
  }

  static async taken(page, count = 1) {
    await page.waitForFunction((wanted) => window.sent.length >= wanted, count, { timeout: 20000 })
    return page.evaluate(() => window.sent.splice(0))
  }

  static async clicked(page, owner, answer) {
    await page.evaluate(([who, how]) => { window.owner = who; window.camera.answer = how }, [owner, answer])
    await page.click('#go')
    return page.evaluate(() => window.answered)
  }

  static async scans(page, owner) {
    assert.deepEqual(await page.evaluate(() => window.ask('page', 'supported')), { value: true })
    const ended = await page.evaluate(() => window.camera.ended())
    assert.deepEqual(await CodeScannerBrowserCheck.clicked(page, owner, 'code'), { value: null })
    assert.deepEqual(await CodeScannerBrowserCheck.taken(page), [[owner, 'CodeScanned', { text: CodeScannerBrowserCheck.text }]])
    const after = await page.evaluate(() => ({ ended: window.camera.ended(), streams: window.camera.streams.length, view: document.querySelector('[data-tachyon-code-scanner]') !== null }))
    assert.deepEqual(after, { ended: ended + 1, streams: after.streams, view: false })
    assert.equal(after.ended, after.streams, 'every camera stream the scanner opened is stopped')
  }

  static async view(page) {
    assert.deepEqual(await CodeScannerBrowserCheck.clicked(page, 'escape', 'blank'), { value: null })
    const shown = await page.evaluate(() => {
      const host = document.querySelector('[data-tachyon-code-scanner]')
      return { host: host !== null, closed: host?.shadowRoot === null, focused: document.activeElement === host }
    })
    assert.deepEqual(shown, { host: true, closed: true, focused: true })
    await page.waitForTimeout(400)
    await page.keyboard.press('Escape')
    assert.deepEqual(await CodeScannerBrowserCheck.taken(page), [['escape', 'ScanFailed', { reason: 'cancelled' }]])
    await CodeScannerBrowserCheck.clicked(page, 'button', 'blank')
    await page.waitForTimeout(400)
    await page.keyboard.press('Enter')
    assert.deepEqual(await CodeScannerBrowserCheck.taken(page), [['button', 'ScanFailed', { reason: 'cancelled' }]])
    await CodeScannerBrowserCheck.clicked(page, 'service', 'blank')
    assert.deepEqual(await page.evaluate(() => window.ask('service', 'cancel')), { value: null })
    assert.deepEqual(await CodeScannerBrowserCheck.taken(page), [['service', 'ScanFailed', { reason: 'cancelled' }]])
    assert.equal(await page.evaluate(() => document.querySelector('[data-tachyon-code-scanner]')), null)
  }

  static async refusals(page) {
    await CodeScannerBrowserCheck.clicked(page, 'denied', 'NotAllowedError')
    assert.deepEqual(await CodeScannerBrowserCheck.taken(page), [['denied', 'ScanFailed', { reason: 'denied' }]])
    const quiet = await page.evaluate(() => [navigator.userActivation?.isActive, window.ask('quiet', 'scan')])
    if (quiet[0] === false) assert.match(quiet[1].error ?? '', /needs a user action/)
    if (quiet[0] !== false) {
      await page.evaluate(() => window.ask('quiet', 'cancel'))
      await CodeScannerBrowserCheck.taken(page)
    }
    const refused = await page.evaluate(() => {
      const grants = document.querySelector('meta[name="tachyon-browser-apis"]')
      grants.content = 'share'
      const answers = [window.ask('page', 'supported'), window.ask('page', 'cancel')]
      grants.content = 'camera'
      return answers
    })
    assert.deepEqual(refused[0], { value: true })
    assert.match(refused[1].error ?? '', /The camera browser API is not granted/)
  }

  static async withoutDetector(page) {
    await page.evaluate(() => { globalThis.BarcodeDetector = undefined })
    await CodeScannerBrowserCheck.scans(page, 'decoded')
    return ', and again with jsQR after removing BarcodeDetector'
  }
}

await CodeScannerBrowserCheck.run()

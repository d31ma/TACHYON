#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium, firefox, webkit } from 'playwright'
import { createServer } from 'node:http'
import { spawnSync } from 'node:child_process'
import { createReadStream, existsSync, mkdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'

class WebComponentsGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static project = path.join(tmpdir(), 'ty-web-components-gate')
  static browsers = [chromium, firefox, webkit]
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static counter = 'shop-counter'
  static hostPage = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Plain host page</title>
  <script type="module" src="/.tachyon/elements/shop-counter.js"></script>
</head>
<body>
  <shop-counter label="Apples" start="5"><span id="slotted">Fresh today</span></shop-counter>
  <p id="heard">-</p>
  <script>
    document.addEventListener('count-changed', (event) => {
      window.heardEvent = { count: event.detail.count, composed: event.composed, target: event.target.localName }
      document.getElementById('heard').textContent = String(event.detail.count)
    })
  </script>
</body>
</html>
`

  /**
   * @param {string} relative
   * @param {string} contents
   */
  static write(relative, contents) {
    const file = path.join(WebComponentsGate.project, relative)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }

  static writeFixture() {
    rmSync(WebComponentsGate.project, { recursive: true, force: true })
    WebComponentsGate.write('client/manifest.json', `{ "name": "Shop", "tac": { "elements": ["${WebComponentsGate.counter}"] } }\n`)
    WebComponentsGate.write('client/components/shop/counter/tac.html', `<section aria-label="Counter">
  <h2 id="label">{label}</h2>
  <p id="count">{counter.count}</p>
  <button id="add" type="button" on:click="counter.add()">Add</button>
  <div id="note"><slot></slot></div>
</section>
`)
    WebComponentsGate.write('client/components/shop/counter/tac.css', '#count { color: rgb(1, 2, 3) }\n')
    WebComponentsGate.write('client/models/CountChanged.js', `/**
 * @typedef {Object} CountChanged
 * @property {integer} count
 */
export {}
`)
    WebComponentsGate.write('client/services/CounterService.js', `@Service()
@NeedsAll
export default class CounterService {
  /** @type {string} */
  start
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher
  count = 0

  @OnStart
  initialize() { this.count = Number(this.start) }

  add() {
    this.count += 1
    /** @type {CountChanged} */
    const changed = { count: this.count }
    this.applicationEventPublisher.publishEvent(changed)
  }
}
`)
  }

  static buildFixture() {
    const built = spawnSync(WebComponentsGate.binary, ['build', WebComponentsGate.project], { encoding: 'utf8' })
    if (built.status !== 0) {
      console.error(built.stderr || built.stdout)
      process.exit(1)
    }
    writeFileSync(path.join(WebComponentsGate.project, 'dist', 'host.html'), WebComponentsGate.hostPage)
  }

  static serveDist() {
    const root = path.join(WebComponentsGate.project, 'dist')
    return createServer((request, response) => {
      const file = path.join(root, decodeURIComponent(request.url.split('?')[0]))
      if (!existsSync(file) || statSync(file).isDirectory()) { response.writeHead(404).end(); return }
      response.writeHead(200, { 'content-type': WebComponentsGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' })
      createReadStream(file).pipe(response)
    })
  }

  /** @param {string} message */
  static fail(message) {
    console.error(`FAIL: ${message}`)
    process.exitCode = 1
  }

  /**
   * @param {unknown} actual
   * @param {unknown} wanted
   * @param {string} label
   */
  static expect(actual, wanted, label) {
    if (actual === wanted) console.log(`    ok   ${label}: ${actual}`)
    else WebComponentsGate.fail(`${label}: expected ${wanted}, got ${actual}`)
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} selector
   * @param {string} wanted
   * @param {string} label
   */
  static async expectShadowText(page, selector, wanted, label) {
    const read = ([tag, inner]) => document.querySelector(tag)?.shadowRoot?.querySelector(inner)?.textContent ?? null
    await page.waitForFunction(([tag, inner, text]) => document.querySelector(tag)?.shadowRoot?.querySelector(inner)?.textContent === text,
      [WebComponentsGate.counter, selector, wanted], { timeout: 5000 }).catch(() => {})
    WebComponentsGate.expect(await page.evaluate(read, [WebComponentsGate.counter, selector]), wanted, label)
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} origin
   */
  static async checkHostPage(page, origin) {
    await page.goto(`${origin}/host.html`, { waitUntil: 'networkidle' })
    await WebComponentsGate.expectShadowText(page, '#label', 'Apples', 'attribute prop at start')
    await WebComponentsGate.expectShadowText(page, '#count', '5', 'service prop at start')
    await page.evaluate((tag) => document.querySelector(tag).setAttribute('label', 'Pears'), WebComponentsGate.counter)
    await WebComponentsGate.expectShadowText(page, '#label', 'Pears', 'attribute change')
    await page.evaluate((tag) => { document.querySelector(tag).label = 'Plums' }, WebComponentsGate.counter)
    await WebComponentsGate.expectShadowText(page, '#label', 'Plums', 'property change')
    await page.focus('#add')
    await page.keyboard.press('Enter')
    await WebComponentsGate.expectShadowText(page, '#count', '6', 'button inside the shadow root')
    const heard = await page.evaluate(() => window.heardEvent ?? null)
    WebComponentsGate.expect(heard?.count, 6, 'event detail')
    WebComponentsGate.expect(heard?.composed, true, 'event leaves the shadow root')
    WebComponentsGate.expect(heard?.target, WebComponentsGate.counter, 'event target seen by the host')
    const focused = await page.evaluate((tag) => document.querySelector(tag).shadowRoot.activeElement?.id ?? null, WebComponentsGate.counter)
    WebComponentsGate.expect(focused, 'add', 'focus stays on the button')
    const assigned = await page.evaluate((tag) => document.querySelector(tag).shadowRoot.querySelector('slot')?.assignedElements().map((element) => element.id).join(',') ?? null, WebComponentsGate.counter)
    WebComponentsGate.expect(assigned, 'slotted', 'slot content')
    WebComponentsGate.expect(await page.isVisible('#slotted'), true, 'slot content is visible')
    const color = await page.evaluate((tag) => getComputedStyle(document.querySelector(tag).shadowRoot.querySelector('#count')).color, WebComponentsGate.counter)
    WebComponentsGate.expect(color, 'rgb(1, 2, 3)', 'component style inside the shadow root')
  }

  /**
   * @param {import('playwright').BrowserType} browserType
   * @param {string} origin
   */
  static async checkBrowser(browserType, origin) {
    console.log(`  ${browserType.name()}`)
    const browser = await browserType.launch()
    const page = await browser.newPage()
    const errors = []
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
    page.on('pageerror', (error) => errors.push(String(error)))
    try {
      await WebComponentsGate.checkHostPage(page, origin)
      WebComponentsGate.expect(errors.join(' | '), '', 'no console errors')
    } finally {
      await browser.close()
    }
  }

  static async run() {
    WebComponentsGate.writeFixture()
    WebComponentsGate.buildFixture()
    const server = WebComponentsGate.serveDist()
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const origin = `http://127.0.0.1:${server.address().port}`
    try {
      for (const browserType of WebComponentsGate.browsers) await WebComponentsGate.checkBrowser(browserType, origin)
    } finally {
      server.close()
    }
    console.log(process.exitCode ? 'FAIL: Tac web components gate' : 'PASS: Tac web components gate')
  }
}

await WebComponentsGate.run()

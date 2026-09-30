#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium, firefox, webkit } from 'playwright'
import { createServer } from 'node:http'
import { spawnSync } from 'node:child_process'
import { createReadStream, existsSync, mkdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'

class WebComponentsGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static project = path.join(tmpdir(), 'ty-web-components-gate')
  static browsers = [chromium, firefox, webkit]
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static counter = 'shop-counter'
  static tally = 'shop-tally'
  static typescript = createRequire(import.meta.url).resolve('typescript/bin/tsc')
  static typedConsumer = `import type { ShopCounterElement } from './dist/.tachyon/elements/shop-counter.js'
const counter: ShopCounterElement | null = document.querySelector('shop-counter')
counter?.addEventListener('count-changed', (event) => {
  const count: number = event.detail.count
  console.log(count)
})
if (counter) counter.label = 'Pears'
`
  static wronglyTypedConsumer = `document.querySelector('shop-counter')?.addEventListener('count-changed', (event) => {
  const count: string = event.detail.count
  console.log(count)
})
export {}
`
  static tacPage = '<shop-counter label="Page" start="1" hydrate="load"><shop-badge slot="title" /></shop-counter>\n'
  static hostPage = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Plain host page</title>
  <script type="module" src="/.tachyon/elements/shop-counter.js"></script>
  <script type="module" src="/.tachyon/elements/shop-tally.js"></script>
</head>
<body>
  <shop-counter label="Apples" start="5"><span id="slotted">Fresh today</span><em id="named" slot="title">Apple of the day</em></shop-counter>
  <shop-tally></shop-tally>
  <p id="heard">-</p>
  <script>
    document.addEventListener('count-changed', (event) => {
      window.heardEvent = { count: event.detail.count, composed: event.composed, target: event.target.localName }
      document.getElementById('heard').textContent = String(event.detail.count)
    })
    document.addEventListener('tally-changed', (event) => {
      window.heardTally = { count: event.detail.count, target: event.target.localName }
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
    WebComponentsGate.write('client/manifest.json', `{ "name": "Shop", "tac": { "elements": ["${WebComponentsGate.counter}", "${WebComponentsGate.tally}"] } }\n`)
    WebComponentsGate.write('client/components/shop/tally/tac.html', `<section aria-label="Tally">
  <p id="tally">{tally.count}</p>
  <button id="tally-add" type="button" on:click="tally.add()">Add</button>
</section>
`)
    WebComponentsGate.write('client/models/TallyChanged.rs', 'pub struct TallyChanged {\n    pub count: i64,\n}\n')
    WebComponentsGate.write('client/services/TallyService.rs', `use crate::models::TallyChanged;

#[derive(Default)]
struct TallyService {
    count: i64,
    application_event_publisher: ApplicationEventPublisher,
}
impl TallyService {
    fn add(&mut self) {
        self.count += 1;
        self.application_event_publisher.publish_event(TallyChanged { count: self.count }, false);
    }
}
`)
    WebComponentsGate.write('client/components/shop/counter/tac.html', `<section aria-label="Counter">
  <h2 id="label">{label}</h2>
  <p id="title"><slot name="title"></slot></p>
  <p id="count">{counter.count}</p>
  <button id="add" type="button" on:click="counter.add()">Add</button>
  <div id="note"><slot></slot></div>
</section>
`)
    WebComponentsGate.write('client/components/shop/badge/tac.html', '<strong id="badge">Tac page</strong>\n')
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

  /** @param {string} source */
  static typeCheck(source) {
    const file = path.join(WebComponentsGate.project, 'consumer.ts')
    writeFileSync(file, source)
    const options = ['--noEmit', '--strict', '--target', 'es2022', '--module', 'esnext', '--moduleResolution', 'bundler', '--lib', 'es2022,dom', file]
    return spawnSync(process.execPath, [WebComponentsGate.typescript, ...options], { encoding: 'utf8' })
  }

  static checkDeclarations() {
    console.log('  TypeScript declarations')
    WebComponentsGate.expect(existsSync(path.join(WebComponentsGate.project, 'dist', 'index.html')), false, 'a project with only elements builds without a page')
    const typed = WebComponentsGate.typeCheck(WebComponentsGate.typedConsumer)
    WebComponentsGate.expect(typed.status, 0, `typed consumer compiles ${typed.stdout.trim()}`)
    const wrong = WebComponentsGate.typeCheck(WebComponentsGate.wronglyTypedConsumer)
    WebComponentsGate.expect(wrong.stdout.includes('TS2322'), true, 'a wrong event detail type is refused')
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

  /** @param {import('playwright').Page} page */
  static async checkWebAssemblyService(page) {
    const read = (tag) => document.querySelector(tag)?.shadowRoot?.querySelector('#tally')?.textContent ?? null
    await page.waitForFunction((tag) => document.querySelector(tag)?.shadowRoot?.querySelector('#tally')?.textContent === '0', WebComponentsGate.tally, { timeout: 30000 }).catch(() => {})
    WebComponentsGate.expect(await page.evaluate(read, WebComponentsGate.tally), '0', 'Rust service at start')
    await page.evaluate((tag) => document.querySelector(tag).shadowRoot.querySelector('#tally-add').click(), WebComponentsGate.tally)
    await page.waitForFunction((tag) => document.querySelector(tag)?.shadowRoot?.querySelector('#tally')?.textContent === '1', WebComponentsGate.tally, { timeout: 5000 }).catch(() => {})
    WebComponentsGate.expect(await page.evaluate(read, WebComponentsGate.tally), '1', 'Rust service method from a shadow root button')
    const heard = await page.evaluate(() => window.heardTally ?? null)
    WebComponentsGate.expect(heard?.count, 1, 'Rust service event detail')
    WebComponentsGate.expect(heard?.target, WebComponentsGate.tally, 'Rust service event target')
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
    await page.focus('#add', { timeout: 5000 })
    await page.keyboard.press('Enter')
    await WebComponentsGate.expectShadowText(page, '#count', '6', 'button inside the shadow root')
    const heard = await page.evaluate(() => window.heardEvent ?? null)
    WebComponentsGate.expect(heard?.count, 6, 'event detail')
    WebComponentsGate.expect(heard?.composed, true, 'event leaves the shadow root')
    WebComponentsGate.expect(heard?.target, WebComponentsGate.counter, 'event target seen by the host')
    const focused = await page.evaluate((tag) => document.querySelector(tag).shadowRoot.activeElement?.id ?? null, WebComponentsGate.counter)
    WebComponentsGate.expect(focused, 'add', 'focus stays on the button')
    const assigned = await page.evaluate((tag) => document.querySelector(tag).shadowRoot.querySelector('slot:not([name])')?.assignedElements().map((element) => element.id).join(',') ?? null, WebComponentsGate.counter)
    WebComponentsGate.expect(assigned, 'slotted', 'slot content')
    const named = await page.evaluate((tag) => document.querySelector(tag).shadowRoot.querySelector('#title slot[name="title"]')?.assignedElements().map((element) => element.id).join(',') ?? null, WebComponentsGate.counter)
    WebComponentsGate.expect(named, 'named', 'named slot content')
    WebComponentsGate.expect(await page.isVisible('#slotted'), true, 'slot content is visible')
    const color = await page.evaluate((tag) => getComputedStyle(document.querySelector(tag).shadowRoot.querySelector('#count')).color, WebComponentsGate.counter)
    WebComponentsGate.expect(color, 'rgb(1, 2, 3)', 'component style inside the shadow root')
    await WebComponentsGate.checkWebAssemblyService(page)
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} origin
   */
  static async checkTacPage(page, origin) {
    await page.goto(`${origin}/index.html`, { waitUntil: 'networkidle' })
    await page.waitForSelector('#title #badge', { timeout: 5000 }).catch(() => {})
    const badge = await page.evaluate(() => document.querySelector('#title #badge')?.textContent ?? null)
    WebComponentsGate.expect(badge, 'Tac page', 'named slot on a Tac page')
  }

  /**
   * @param {import('playwright').BrowserType} browserType
   * @param {(page: import('playwright').Page) => Promise<void>} check
   */
  static async checkBrowser(browserType, check) {
    console.log(`  ${browserType.name()}`)
    const browser = await browserType.launch()
    const page = await browser.newPage()
    const errors = []
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
    page.on('pageerror', (error) => errors.push(String(error)))
    try {
      await check(page)
      WebComponentsGate.expect(errors.join(' | '), '', 'no console errors')
    } finally {
      await browser.close()
    }
  }

  /** @param {(page: import('playwright').Page, origin: string) => Promise<void>} check */
  static async inEveryBrowser(check) {
    const server = WebComponentsGate.serveDist()
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const origin = `http://127.0.0.1:${server.address().port}`
    try {
      for (const browserType of WebComponentsGate.browsers) await WebComponentsGate.checkBrowser(browserType, (page) => check(page, origin))
    } finally {
      server.close()
    }
  }

  static async run() {
    WebComponentsGate.writeFixture()
    WebComponentsGate.buildFixture()
    WebComponentsGate.checkDeclarations()
    await WebComponentsGate.inEveryBrowser(WebComponentsGate.checkHostPage)
    WebComponentsGate.write('client/pages/tac.html', WebComponentsGate.tacPage)
    WebComponentsGate.buildFixture()
    await WebComponentsGate.inEveryBrowser(WebComponentsGate.checkTacPage)
    console.log(process.exitCode ? 'FAIL: Tac web components gate' : 'PASS: Tac web components gate')
  }
}

await WebComponentsGate.run()

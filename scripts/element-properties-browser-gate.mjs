#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { cp, mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} PropertyEngine */
/** @typedef {{engine: PropertyEngine, phase: string, errors: string[], consoleErrors: string[], failedRequests: string[], externalRequests: string[]}} PropertyDiagnostics */
/** @typedef {{kind: string, points: unknown, pointsWrites: number, options: unknown, optionsWrites: number, attributes: string[], ownPoints: boolean, marker: string | null}} ProbeState */

class ElementPropertiesBrowserGate {
  static _timeout = 15_000
  static _engines = ['chromium', 'firefox', 'webkit']
  static _repository = fileURLToPath(new URL('..', import.meta.url))
  static _fixture = 'crates/tachyon-core/tests/fixtures/tac-element-properties'
  static _probeElement = `
    class ProbeChart extends HTMLElement {
      constructor() { super(); this.pointsWrites = 0; this.optionsWrites = 0 }
      get points() { return this._points }
      set points(value) { this.pointsWrites += 1; this._points = value; this.dataset.kind = Array.isArray(value) ? 'array' : typeof value }
      get chartOptions() { return this._options }
      set chartOptions(value) { this.optionsWrites += 1; this._options = value }
    }
    globalThis.ProbeChart = ProbeChart
    customElements.define('probe-chart', ProbeChart)
    customElements.define('probe-fixed', class extends HTMLElement {
      get values() { return [] }
    })
    globalThis.probeLightWrites = []
    customElements.define('probe-light', class extends HTMLElement {
      get points() { return this._points }
      set points(value) { globalThis.probeLightWrites.push(value); this._points = value; this._paint() }
      connectedCallback() { this._paint() }
      _paint() { this.textContent = (this._points ?? []).join(',') }
    })
  `
  static _refusedBinding = '<probe-fixed id="fixed">'

  /** @returns {Promise<void>} */
  static async run() {
    const binary = ElementPropertiesBrowserGate._binary()
    const engines = ElementPropertiesBrowserGate._selectedEngines()
    const project = await mkdtemp(path.join(tmpdir(), 'tac-element-properties-'))
    try {
      await cp(path.join(ElementPropertiesBrowserGate._repository, ElementPropertiesBrowserGate._fixture), project, { recursive: true })
      await promisify(execFile)(binary, ['bundle', project])
      await withServer(binary, ['preview', project, '--static', '--port', '0'], async (request) => {
        const origin = new URL((await request('/')).url).origin
        for (const engine of engines) await ElementPropertiesBrowserGate._runEngine(engine, origin)
      })
    } finally {
      await rm(project, { recursive: true, force: true })
    }
  }

  /** @returns {string} */
  static _binary() {
    const binary = process.env.TAC_BIN
    assert.ok(binary, 'TAC_BIN names the ty binary under test')
    return path.resolve(binary)
  }

  /** @returns {PropertyEngine[]} */
  static _selectedEngines() {
    const requested = process.env.TAC_ELEMENT_PROPERTIES_BROWSERS
    if (!requested) return /** @type {PropertyEngine[]} */ (ElementPropertiesBrowserGate._engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(ElementPropertiesBrowserGate._engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {PropertyEngine[]} */ (engines)
  }

  /** @param {PropertyEngine} engine @returns {import('playwright').BrowserType} */
  static _browser(engine) {
    const browsers = { chromium, firefox, webkit }
    return browsers[engine]
  }

  /** @param {PropertyEngine} engine @param {string} origin @returns {Promise<void>} */
  static async _runEngine(engine, origin) {
    /** @type {PropertyDiagnostics} */
    const diagnostics = { engine, phase: 'starting', errors: [], consoleErrors: [], failedRequests: [], externalRequests: [] }
    const browser = await ElementPropertiesBrowserGate._browser(engine).launch({ timeout: ElementPropertiesBrowserGate._timeout })
    try {
      const page = await browser.newPage()
      ElementPropertiesBrowserGate._observe(page, origin, diagnostics)
      await page.addInitScript(ElementPropertiesBrowserGate._probeElement)
      await page.goto(origin, { waitUntil: 'networkidle', timeout: ElementPropertiesBrowserGate._timeout })
      await page.waitForFunction(() => document.querySelector('#early')?.dataset.kind === 'array')
      diagnostics.phase = 'first-render'
      await ElementPropertiesBrowserGate._checkFirstRender(page)
      diagnostics.phase = 'plain-element'
      await ElementPropertiesBrowserGate._checkPlainElement(page)
      diagnostics.phase = 'update-retains-element'
      await ElementPropertiesBrowserGate._checkUpdate(page)
      diagnostics.phase = 'removed-binding'
      await ElementPropertiesBrowserGate._checkRemovedBinding(page)
      diagnostics.phase = 'late-definition'
      await ElementPropertiesBrowserGate._checkLateDefinition(page)
      diagnostics.phase = 'replaced-view'
      await ElementPropertiesBrowserGate._checkReplacedView(page)
      diagnostics.phase = 'refused-property'
      await ElementPropertiesBrowserGate._checkRefusedProperty(page, diagnostics)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      console.log(JSON.stringify({ status: 'pass', engine, browserVersion: browser.version() }))
    } catch (error) {
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} selector @returns {Promise<ProbeState>} */
  static _state(page, selector) {
    return page.locator(selector).evaluate((element) => ({
      kind: element.dataset.kind ?? '',
      points: element.points,
      pointsWrites: element.pointsWrites,
      options: element.chartOptions ?? null,
      optionsWrites: element.optionsWrites,
      attributes: element.getAttributeNames().sort(),
      ownPoints: Object.hasOwn(element, 'points'),
      marker: element.dataset.marker ?? null,
    }))
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkFirstRender(page) {
    const state = await ElementPropertiesBrowserGate._state(page, '#early')
    assert.equal(state.kind, 'array')
    assert.deepEqual(state.points, [1, 2, 3])
    assert.deepEqual(state.options, { stacked: true, title: 'Load' })
    assert.deepEqual(state.attributes, ['count', 'data-kind', 'id', 'label'])
    assert.equal(await page.locator('#early').getAttribute('label'), 'Load')
    assert.equal(await page.locator('#early').getAttribute('count'), '3')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkPlainElement(page) {
    const plain = await page.locator('#plain').evaluate((element) => ({
      attribute: element.getAttribute('data-points'),
      property: Object.hasOwn(element, 'dataPoints'),
    }))
    assert.deepEqual(plain, { attribute: '', property: false })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkUpdate(page) {
    await page.locator('#early').evaluate((element) => { element.dataset.marker = 'kept' })
    const before = await ElementPropertiesBrowserGate._state(page, '#early')
    await page.locator('#add-point').click()
    await page.waitForFunction(() => document.querySelector('#count')?.textContent === '4')
    const after = await ElementPropertiesBrowserGate._state(page, '#early')
    assert.equal(after.marker, 'kept')
    assert.deepEqual(after.points, [1, 2, 3, 4])
    assert.equal(after.pointsWrites, before.pointsWrites + 1)
    assert.equal(after.optionsWrites, before.optionsWrites)
    assert.equal(await page.locator('#early').getAttribute('count'), '4')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkRemovedBinding(page) {
    await page.locator('#clear-options').click()
    await page.waitForFunction(() => document.querySelector('#early')?.chartOptions === undefined)
    const state = await ElementPropertiesBrowserGate._state(page, '#early')
    assert.equal(state.options, null)
    assert.equal(state.attributes.includes('chart-options'), false)
    assert.equal(state.marker, 'kept')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkLateDefinition(page) {
    const before = await page.locator('#late').evaluate((element) => Object.hasOwn(element, 'points'))
    assert.equal(before, true)
    await page.evaluate(() => customElements.define('probe-late', class extends globalThis.ProbeChart {}))
    await page.waitForFunction(() => document.querySelector('#late')?.dataset.kind === 'array')
    const state = await ElementPropertiesBrowserGate._state(page, '#late')
    assert.equal(state.ownPoints, false)
    assert.deepEqual(state.points, [1, 2, 3, 4])
    assert.equal(state.attributes.includes('points'), false)
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkReplacedView(page) {
    await page.locator('#nested').evaluate((element) => { element.dataset.marker = 'kept' })
    await page.locator('#light').evaluate((element) => { element.dataset.marker = 'kept' })
    const before = await ElementPropertiesBrowserGate._state(page, '#nested')
    for (const turn of [1, 2]) await ElementPropertiesBrowserGate._nextView(page, turn)
    const after = await ElementPropertiesBrowserGate._state(page, '#nested')
    assert.equal(after.marker, 'kept')
    assert.deepEqual(after.points, [2, 20])
    assert.equal(after.pointsWrites, before.pointsWrites + 2)
    const light = await page.evaluate(() => ({
      text: document.querySelector('#light')?.textContent,
      last: globalThis.probeLightWrites.at(-1),
      marker: document.querySelector('#light')?.dataset.marker ?? null,
    }))
    assert.deepEqual({ last: light.last, marker: light.marker }, { last: [2, 20], marker: 'kept' })
    await ElementPropertiesBrowserGate._checkRepeatedView(page, after.pointsWrites)
  }

  /** @param {import('playwright').Page} page @param {number} writes @returns {Promise<void>} */
  static async _checkRepeatedView(page, writes) {
    await page.locator('#repeat-view').click()
    await page.waitForFunction(() => document.querySelector('#view-title')?.textContent === 'View 2 again')
    const repeated = await ElementPropertiesBrowserGate._state(page, '#nested')
    assert.equal(repeated.marker, 'kept')
    assert.equal(repeated.pointsWrites, writes)
    assert.deepEqual(repeated.points, [2, 20])
  }

  /** @param {import('playwright').Page} page @param {number} turn @returns {Promise<void>} */
  static async _nextView(page, turn) {
    await page.locator('#next-view').click()
    await page.waitForFunction((title) => document.querySelector('#view-title')?.textContent === title, `View ${turn}`)
    await page.waitForFunction((first) => document.querySelector('#light')?.textContent?.startsWith(`${first},`), turn)
  }

  /** @param {import('playwright').Page} page @param {PropertyDiagnostics} diagnostics @returns {Promise<void>} */
  static async _checkRefusedProperty(page, diagnostics) {
    const refused = diagnostics.consoleErrors.filter((message) => message.includes(ElementPropertiesBrowserGate._refusedBinding))
    assert.equal(refused.length, 1, diagnostics.consoleErrors.join('\n'))
    assert.ok(refused[0].includes(':values'), refused[0])
    const fixed = await page.locator('#fixed').evaluate((element) => ({
      values: element.values,
      own: Object.hasOwn(element, 'values'),
      attributes: element.getAttributeNames().sort(),
    }))
    assert.deepEqual(fixed, { values: [], own: false, attributes: ['id'] })
    assert.equal(await page.locator('html').getAttribute('data-tachyon-render-error'), null)
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {PropertyDiagnostics} diagnostics @returns {void} */
  static _observe(page, origin, diagnostics) {
    page.on('pageerror', (error) => diagnostics.errors.push(error.message))
    page.on('console', (message) => ElementPropertiesBrowserGate._consoleError(message, diagnostics))
    page.on('requestfailed', (request) => diagnostics.failedRequests.push(request.url()))
    page.on('request', (request) => ElementPropertiesBrowserGate._external(request.url(), origin, diagnostics))
  }

  /** @param {import('playwright').ConsoleMessage} message @param {PropertyDiagnostics} diagnostics @returns {void} */
  static _consoleError(message, diagnostics) {
    if (message.type() === 'error') diagnostics.consoleErrors.push(message.text())
  }

  /** @param {string} url @param {string} origin @param {PropertyDiagnostics} diagnostics @returns {void} */
  static _external(url, origin, diagnostics) {
    if (new URL(url).origin !== origin) diagnostics.externalRequests.push(url)
  }
}

await ElementPropertiesBrowserGate.run()

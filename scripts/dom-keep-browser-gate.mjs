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

/** @typedef {'chromium' | 'firefox' | 'webkit'} KeepEngine */
/** @typedef {{engine: KeepEngine, phase: string, errors: string[]}} KeepDiagnostics */
/** @typedef {{count: number, first: boolean}} RenderedDetail */
/** @typedef {{renders: number, events: RenderedDetail[], boxInserted: number, pinnedRemoved: string[], applied: Record<string, boolean>, kept: Record<string, boolean>, titleKept: boolean, sameBox: boolean, order: string[], count: string, renderedAtLoad: boolean}} KeepSnapshot */

class DomKeepBrowserGate {
  static _timeout = 15_000
  static _launchTimeout = 60_000
  static _afterLastChange = 800
  static _engines = ['chromium', 'firefox', 'webkit']
  static _changes = ['a', 'b', 'c', 'd']
  static _plainHttpPrefetch = 'Prefetch request denied: URL must be secure (HTTPS)'
  static _boxOrder = ['H1#title', 'NAV']
  static _counterOrder = ['H1#title', 'OUTPUT#count', 'BUTTON#bump', 'P#positive', 'NAV']
  static _mountRenders = [1, 2]
  static _repository = fileURLToPath(new URL('..', import.meta.url))
  static _fixture = 'crates/tachyon-core/tests/fixtures/tac-dom-keep'

  /** @returns {Promise<void>} */
  static async run() {
    const binary = DomKeepBrowserGate._binary()
    const engines = DomKeepBrowserGate._selectedEngines()
    const project = await DomKeepBrowserGate._project()
    try {
      await promisify(execFile)(binary, ['bundle', project])
      await withServer(binary, ['preview', project, '--static', '--port', '0'], async (request) => {
        const origin = new URL((await request('/')).url).origin
        for (const engine of engines) await DomKeepBrowserGate._runEngine(engine, origin)
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

  /** @returns {KeepEngine[]} */
  static _selectedEngines() {
    const requested = process.env.TAC_DOM_KEEP_BROWSERS
    if (!requested) return /** @type {KeepEngine[]} */ (DomKeepBrowserGate._engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(DomKeepBrowserGate._engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {KeepEngine[]} */ (engines)
  }

  /** @returns {Promise<string>} */
  static async _project() {
    const project = await mkdtemp(path.join(tmpdir(), 'tac-dom-keep-'))
    await cp(path.join(DomKeepBrowserGate._repository, DomKeepBrowserGate._fixture), project, { recursive: true })
    return project
  }

  /** @param {KeepEngine} engine @returns {import('playwright').BrowserType} */
  static _browserType(engine) {
    if (engine === 'firefox') return firefox
    if (engine === 'webkit') return webkit
    return chromium
  }

  /** @param {KeepEngine} engine @param {string} origin @returns {Promise<void>} */
  static async _runEngine(engine, origin) {
    /** @type {KeepDiagnostics} */
    const diagnostics = { engine, phase: 'starting', errors: [] }
    const browser = await DomKeepBrowserGate._browserType(engine).launch({ timeout: DomKeepBrowserGate._launchTimeout })
    const page = await browser.newPage()
    page.on('pageerror', (error) => diagnostics.errors.push(error.message))
    page.on('console', (message) => DomKeepBrowserGate._recordConsole(message, diagnostics))
    await page.addInitScript(DomKeepBrowserGate._probe)
    try {
      diagnostics.phase = 'plain-load'
      await page.goto(`${origin}/`, { waitUntil: 'load', timeout: DomKeepBrowserGate._timeout })
      await DomKeepBrowserGate._checkPlainLoad(page)
      diagnostics.phase = 'rerender'
      await DomKeepBrowserGate._checkRerender(page)
      diagnostics.phase = 'mounted-load'
      await page.click('#to-two')
      await page.waitForURL((url) => /^\/two\/?$/.test(url.pathname))
      await DomKeepBrowserGate._checkMountedLoad(page)
      diagnostics.phase = 'tac-update'
      await DomKeepBrowserGate._checkTacUpdate(page)
      assert.deepEqual(diagnostics.errors, [])
      console.log(JSON.stringify({ status: 'pass', engine, browserVersion: browser.version() }))
    } catch (error) {
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').ConsoleMessage} message @param {KeepDiagnostics} diagnostics @returns {void} */
  static _recordConsole(message, diagnostics) {
    if (message.type() !== 'error') return
    if (message.text() === DomKeepBrowserGate._plainHttpPrefetch) return
    diagnostics.errors.push(message.text())
  }

  /** @type {() => void} */
  static _probe = () => {
    const state = { renders: 0, events: [], boxInserted: 0, pinnedRemoved: [] }
    globalThis.__keepProbe = state
    document.addEventListener('tachyon:rendered', (event) => state.events.push({ count: event.detail.count, first: event.detail.first }))
    const holdsBox = (node) => node.nodeType === Node.ELEMENT_NODE && (node.matches('.box') || node.querySelector('.box') !== null)
    const holdsPinned = (node) => node.nodeType === Node.ELEMENT_NODE && node.id.startsWith('pinned-child-')
    new MutationObserver((records) => {
      for (const record of records) {
        if (record.type === 'attributes' && record.attributeName === 'data-tachyon-rendered') state.renders += 1
        if (record.type === 'attributes' && record.attributeName.startsWith('data-pinned-') && !record.target.hasAttribute(record.attributeName)) state.pinnedRemoved.push(record.attributeName)
        if (record.type !== 'childList') continue
        for (const node of record.addedNodes) if (holdsBox(node)) state.boxInserted += 1
        for (const node of record.removedNodes) if (holdsPinned(node)) state.pinnedRemoved.push(node.id)
      }
    }).observe(document, { subtree: true, childList: true, attributes: true, attributeOldValue: true })
  }

  /** @param {import('playwright').Page} page @returns {Promise<KeepSnapshot>} */
  static _snapshot(page) {
    return page.evaluate((changes) => {
      const box = document.querySelector('.box')
      const probe = globalThis.__keepProbe
      return {
        renders: probe.renders,
        events: probe.events,
        boxInserted: probe.boxInserted,
        pinnedRemoved: probe.pinnedRemoved,
        applied: Object.fromEntries(changes.map((name) => [name, globalThis.__pinned?.applied[name] === true])),
        kept: Object.fromEntries(changes.map((name) => [name, box?.getAttribute(`data-pinned-${name}`) === 'yes' && document.getElementById(`pinned-child-${name}`)?.parentNode === box])),
        titleKept: document.getElementById('title')?.getAttribute('data-pinned-e') === 'yes',
        sameBox: globalThis.__keptBox === box,
        order: [...(box?.children || [])].map((child) => child.id ? `${child.tagName}#${child.id}` : child.tagName),
        count: document.getElementById('count')?.textContent ?? '',
        renderedAtLoad: globalThis.__pinned?.renderedAtLoad === true,
      }
    }, DomKeepBrowserGate._changes)
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _settle(page) {
    await page.waitForFunction(() => document.documentElement.dataset.tachyonRendered === 'client', null, { timeout: DomKeepBrowserGate._timeout })
    await page.waitForTimeout(DomKeepBrowserGate._afterLastChange)
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkPlainLoad(page) {
    await DomKeepBrowserGate._settle(page)
    const snapshot = await DomKeepBrowserGate._snapshot(page)
    assert.equal(snapshot.renders, 1, 'a page load without a mounted component renders once')
    assert.deepEqual(snapshot.events, [{ count: 1, first: true }], 'tachyon:rendered fires once, for the first render')
    DomKeepBrowserGate._assertLoadKept(snapshot)
    await page.evaluate(() => { globalThis.__keptBox = document.querySelector('.box') })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkMountedLoad(page) {
    await DomKeepBrowserGate._settle(page)
    const snapshot = await DomKeepBrowserGate._snapshot(page)
    assert.ok(DomKeepBrowserGate._mountRenders.includes(snapshot.renders), `a mounted component adds at most one render (${snapshot.renders})`)
    assert.equal(snapshot.events.length, snapshot.renders, 'tachyon:rendered fires once per render')
    assert.deepEqual(snapshot.events[0], { count: 1, first: true })
    DomKeepBrowserGate._assertLoadKept(snapshot)
    await page.evaluate(() => { globalThis.__keptBox = document.querySelector('.box') })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkTacUpdate(page) {
    const before = (await DomKeepBrowserGate._snapshot(page)).events.length
    await page.click('#bump')
    await page.waitForFunction((count) => globalThis.__keepProbe.events.length === count, before + 1, { timeout: DomKeepBrowserGate._timeout })
    const snapshot = await DomKeepBrowserGate._snapshot(page)
    assert.equal(snapshot.count, '1', 'Tac still updates the text it owns')
    assert.deepEqual(snapshot.events[before], { count: before + 1, first: false })
    assert.equal(snapshot.sameBox, true, 'the component element is kept')
    DomKeepBrowserGate._assertKept(snapshot)
    DomKeepBrowserGate._assertOrder(snapshot, DomKeepBrowserGate._counterOrder)
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkRerender(page) {
    await page.evaluate(() => dispatchEvent(new Event('tachyon:rerender')))
    await page.waitForFunction(() => globalThis.__keepProbe.events.length === 2, null, { timeout: DomKeepBrowserGate._timeout })
    const snapshot = await DomKeepBrowserGate._snapshot(page)
    assert.deepEqual(snapshot.events[1], { count: 2, first: false })
    assert.equal(snapshot.sameBox, true, 'the component element is kept after tachyon:rerender')
    DomKeepBrowserGate._assertKept(snapshot)
    DomKeepBrowserGate._assertOrder(snapshot, DomKeepBrowserGate._boxOrder)
  }

  /** @param {KeepSnapshot} snapshot @returns {void} */
  static _assertLoadKept(snapshot) {
    assert.equal(snapshot.boxInserted, 1, 'the component element is inserted once')
    DomKeepBrowserGate._assertKept(snapshot)
  }

  /** @param {KeepSnapshot} snapshot @returns {void} */
  static _assertKept(snapshot) {
    const everyChange = Object.fromEntries(DomKeepBrowserGate._changes.map((name) => [name, true]))
    assert.deepEqual(snapshot.applied, everyChange, `every pinned change ran (renderedAtLoad ${snapshot.renderedAtLoad})`)
    assert.deepEqual(snapshot.kept, everyChange, 'every pinned child and attribute is still there')
    assert.equal(snapshot.titleKept, true, 'an attribute-only change is still there')
    assert.deepEqual(snapshot.pinnedRemoved, [], 'Tac removed nothing the pinned script added')
  }

  /** @param {KeepSnapshot} snapshot @param {string[]} expected @returns {void} */
  static _assertOrder(snapshot, expected) {
    const authored = snapshot.order.filter((name) => !name.startsWith('P#pinned-child-'))
    assert.deepEqual(authored, expected, 'Tac keeps its own nodes in order, removes the ones it no longer renders and adds new ones')
  }
}

await DomKeepBrowserGate.run()

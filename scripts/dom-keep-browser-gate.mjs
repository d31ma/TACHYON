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
/** @typedef {{sameField: boolean, sameInput: boolean, sameSelect: boolean, sameRepaint: boolean, active: string, value: string, caret: Array<number | null>, choice: string, hint: string | null, seen: Array<string | null>, repaintHint: string | null, repaintShows: string | null, repaintChildren: number}} FormSnapshot */
/** @typedef {{sameHost: boolean, calls: number, tacInputs: number, inputInHost: boolean, ownNodes: number, ownFirst: boolean, ownShows: string | null, hint: string | null, active: string}} RebuildSnapshot */

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
  static _typedFirst = 'abc'
  static _typedLater = 'def'
  static _choice = 'Cherry'
  static _updates = 3
  static _inputHints = ['Hint 0', 'Hint 1', null, 'Hint 3']
  static _selectHints = ['Hint 4', null, 'Hint 6']

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
    await page.addInitScript(DomKeepBrowserGate._formElements)
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
      diagnostics.phase = 'form-input'
      await page.goto(`${origin}/form/`, { waitUntil: 'load', timeout: DomKeepBrowserGate._timeout })
      await DomKeepBrowserGate._checkFormInput(page)
      diagnostics.phase = 'form-select'
      await DomKeepBrowserGate._checkFormSelect(page)
      diagnostics.phase = 'rebuilt-children'
      await DomKeepBrowserGate._checkRebuiltChildren(page)
      diagnostics.phase = 'focus-not-stolen'
      await DomKeepBrowserGate._checkFocusNotStolen(page)
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

  /** @type {() => void} */
  static _formElements = () => {
    customElements.define('field-box', class extends HTMLElement {
      static observedAttributes = ['hint']
      seen = []
      attributeChangedCallback(name, previous, value) {
        this.seen.push(value)
      }
    })
    customElements.define('field-repaint', class extends HTMLElement {
      static observedAttributes = ['hint']
      attributeChangedCallback(name, previous, value) {
        const input = document.createElement('input')
        input.setAttribute('aria-label', 'Repainted')
        const shown = document.createElement('small')
        shown.textContent = value ?? ''
        this.replaceChildren(input, shown)
      }
    })
    customElements.define('field-rebuild', class extends HTMLElement {
      static observedAttributes = ['hint']
      calls = 0
      own = null
      attributeChangedCallback(name, previous, value) {
        if (!this.isConnected) return
        this.calls += 1
        this.own = document.createElement('small')
        this.own.className = 'own'
        this.own.textContent = value ?? ''
        this.replaceChildren(this.own)
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<RebuildSnapshot>} */
  static _rebuildSnapshot(page) {
    return page.evaluate(() => {
      const host = document.getElementById('rebuild')
      return {
        sameHost: host === globalThis.__keptRebuild,
        calls: host.calls,
        tacInputs: document.querySelectorAll('#rebuild-input').length,
        inputInHost: document.getElementById('rebuild-input')?.parentNode === host,
        ownNodes: host.querySelectorAll('.own').length,
        ownFirst: host.firstChild === host.own,
        ownShows: host.own?.textContent ?? null,
        hint: host.getAttribute('hint'),
        active: document.activeElement?.id ?? '',
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkRebuiltChildren(page) {
    await page.evaluate(() => { globalThis.__keptRebuild = document.getElementById('rebuild') })
    await page.focus('#rebuild-input')
    const before = (await DomKeepBrowserGate._rebuildSnapshot(page)).calls
    for (let turn = 1; turn <= DomKeepBrowserGate._updates; turn += 1) {
      await DomKeepBrowserGate._updateHint(page)
      const snapshot = await DomKeepBrowserGate._rebuildSnapshot(page)
      assert.equal(snapshot.sameHost, true, 'an element whose callback removed Tac children is kept')
      assert.equal(snapshot.calls, before + turn, 'the callback runs once per update')
      assert.equal(snapshot.tacInputs, 1, 'Tac renders its removed child again exactly once')
      assert.equal(snapshot.inputInHost, true, 'the child is rendered again inside the kept element')
      assert.equal(snapshot.ownNodes, 1, 'the element keeps only its own new node')
      assert.equal(snapshot.ownFirst, true, 'Tac leaves the element\'s own node in place, before its child')
      assert.equal(snapshot.ownShows, snapshot.hint ?? '', 'the element painted the new value')
      assert.equal(snapshot.active, 'rebuild-input', 'the focus comes back to the child Tac rendered again')
    }
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkFocusNotStolen(page) {
    await page.focus('#rebuild-input')
    await page.focus('#elsewhere')
    const before = (await DomKeepBrowserGate._rebuildSnapshot(page)).calls
    await DomKeepBrowserGate._updateHint(page)
    const snapshot = await DomKeepBrowserGate._rebuildSnapshot(page)
    assert.equal(snapshot.calls, before + 1, 'the callback runs once per update')
    assert.equal(snapshot.tacInputs, 1, 'Tac renders its removed child again exactly once')
    assert.equal(snapshot.active, 'elsewhere', 'Tac does not take the focus from where the user put it')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _keepFormNodes(page) {
    await page.evaluate(() => {
      globalThis.__keptForm = ['field', 'field-input', 'field-select', 'repaint'].map((id) => document.getElementById(id))
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<FormSnapshot>} */
  static _formSnapshot(page) {
    return page.evaluate(() => {
      const [field, input, select, repaint] = globalThis.__keptForm
      const current = document.getElementById('repaint')
      return {
        sameField: document.getElementById('field') === field,
        sameInput: document.getElementById('field-input') === input,
        sameSelect: document.getElementById('field-select') === select,
        sameRepaint: current === repaint,
        active: document.activeElement?.id ?? '',
        value: input.value,
        caret: [input.selectionStart, input.selectionEnd],
        choice: select.value,
        hint: field.getAttribute('hint'),
        seen: field.seen,
        repaintHint: current.getAttribute('hint'),
        repaintShows: current.querySelector('small')?.textContent ?? null,
        repaintChildren: current.childNodes.length,
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _updateHint(page) {
    const before = await page.evaluate(() => globalThis.__keepProbe.events.length)
    await page.evaluate(() => document.getElementById('hint-bump').click())
    await page.waitForFunction((count) => globalThis.__keepProbe.events.length > count, before, { timeout: DomKeepBrowserGate._timeout })
  }

  /** @param {import('playwright').Page} page @param {number} times @returns {Promise<void>} */
  static async _updateHints(page, times) {
    for (let turn = 0; turn < times; turn += 1) await DomKeepBrowserGate._updateHint(page)
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkFormInput(page) {
    await DomKeepBrowserGate._settle(page)
    await DomKeepBrowserGate._keepFormNodes(page)
    await page.focus('#field-input')
    await page.keyboard.type(DomKeepBrowserGate._typedFirst)
    await DomKeepBrowserGate._updateHints(page, DomKeepBrowserGate._updates)
    const snapshot = await DomKeepBrowserGate._formSnapshot(page)
    DomKeepBrowserGate._assertSameNodes(snapshot)
    assert.equal(snapshot.active, 'field-input', 'the focused input keeps the focus after three updates')
    assert.equal(snapshot.value, DomKeepBrowserGate._typedFirst, 'the typed text is kept')
    assert.deepEqual(snapshot.caret, [DomKeepBrowserGate._typedFirst.length, DomKeepBrowserGate._typedFirst.length], 'the caret is kept')
    assert.equal(snapshot.hint, 'Hint 3', 'the element has the new attribute value')
    assert.deepEqual(snapshot.seen, DomKeepBrowserGate._inputHints, 'the element sees each change once, removal included')
    DomKeepBrowserGate._assertRepainted(snapshot, 'Hint 3')
    await page.keyboard.type(DomKeepBrowserGate._typedLater)
    const typed = await DomKeepBrowserGate._formSnapshot(page)
    assert.equal(typed.value, `${DomKeepBrowserGate._typedFirst}${DomKeepBrowserGate._typedLater}`, 'typing goes on in the same input')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkFormSelect(page) {
    await page.selectOption('#field-select', DomKeepBrowserGate._choice)
    await page.focus('#field-select')
    await DomKeepBrowserGate._updateHints(page, DomKeepBrowserGate._updates)
    const snapshot = await DomKeepBrowserGate._formSnapshot(page)
    DomKeepBrowserGate._assertSameNodes(snapshot)
    assert.equal(snapshot.active, 'field-select', 'the focused select keeps the focus after three updates')
    assert.equal(snapshot.choice, DomKeepBrowserGate._choice, 'the chosen option is kept')
    assert.equal(snapshot.hint, 'Hint 6', 'the element has the new attribute value')
    assert.deepEqual(snapshot.seen.slice(-3), DomKeepBrowserGate._selectHints, 'the element sees each change once')
    DomKeepBrowserGate._assertRepainted(snapshot, 'Hint 6')
  }

  /** @param {FormSnapshot} snapshot @returns {void} */
  static _assertSameNodes(snapshot) {
    assert.equal(snapshot.sameField, true, 'the custom element with a changed observed attribute is kept')
    assert.equal(snapshot.sameInput, true, 'the input inside it is kept')
    assert.equal(snapshot.sameSelect, true, 'the select inside it is kept')
    assert.equal(snapshot.sameRepaint, true, 'an element that rebuilds itself on an attribute change is kept')
  }

  /** @param {FormSnapshot} snapshot @param {string} hint @returns {void} */
  static _assertRepainted(snapshot, hint) {
    assert.equal(snapshot.repaintHint, hint, 'the rebuilding element has the new attribute value')
    assert.equal(snapshot.repaintShows, hint, 'the rebuilding element painted the new value')
    assert.equal(snapshot.repaintChildren, 2, 'Tac adds nothing to what the element painted')
  }

  /** @param {KeepSnapshot} snapshot @param {string[]} expected @returns {void} */
  static _assertOrder(snapshot, expected) {
    const authored = snapshot.order.filter((name) => !name.startsWith('P#pinned-child-'))
    assert.deepEqual(authored, expected, 'Tac keeps its own nodes in order, removes the ones it no longer renders and adds new ones')
  }
}

await DomKeepBrowserGate.run()

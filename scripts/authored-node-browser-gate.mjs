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

/** @typedef {'chromium' | 'firefox' | 'webkit'} RetentionEngine */
/** @typedef {{engine: RetentionEngine, phase: string, errors: string[], failedRequests: string[], externalRequests: string[]}} RetentionDiagnostics */

class AuthoredNodeBrowserGate {
  static _timeout = 15_000
  static _settle = 80
  static _engines = ['chromium', 'firefox', 'webkit']
  static _repository = fileURLToPath(new URL('..', import.meta.url))
  static _fixture = 'crates/tachyon-core/tests/fixtures/tac-authored-node-retention'
  static _standaloneHost = '<!doctype html><html lang="en"><head><title>Standalone retention</title><script src="/shared/assets/duvay/duvay-wc-card.min.js"></script><script type="module" src="/.tachyon/elements/retention-panel.js"></script></head><body><retention-panel></retention-panel></body></html>'

  /** @returns {Promise<void>} */
  static async run() {
    const binary = AuthoredNodeBrowserGate._binary()
    const engines = AuthoredNodeBrowserGate._selectedEngines()
    const project = await AuthoredNodeBrowserGate._project()
    try {
      await promisify(execFile)(binary, ['bundle', project])
      await withServer(binary, ['preview', project, '--static', '--port', '0'], async (request) => {
        const origin = new URL((await request('/')).url).origin
        for (const engine of engines) await AuthoredNodeBrowserGate._runEngine(engine, origin)
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

  /** @returns {RetentionEngine[]} */
  static _selectedEngines() {
    const requested = process.env.TAC_AUTHORED_NODE_BROWSERS
    if (!requested) return /** @type {RetentionEngine[]} */ (AuthoredNodeBrowserGate._engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(AuthoredNodeBrowserGate._engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {RetentionEngine[]} */ (engines)
  }

  /** @returns {Promise<string>} */
  static async _project() {
    const project = await mkdtemp(path.join(tmpdir(), 'tac-authored-node-'))
    await cp(path.join(AuthoredNodeBrowserGate._repository, AuthoredNodeBrowserGate._fixture), project, { recursive: true })
    return project
  }

  /** @param {RetentionEngine} engine @returns {import('playwright').BrowserType} */
  static _browserType(engine) {
    if (engine === 'firefox') return firefox
    if (engine === 'webkit') return webkit
    return chromium
  }

  /** @param {RetentionEngine} engine @param {string} origin @returns {Promise<void>} */
  static async _runEngine(engine, origin) {
    /** @type {RetentionDiagnostics} */
    const diagnostics = { engine, phase: 'starting', errors: [], failedRequests: [], externalRequests: [] }
    const browser = await AuthoredNodeBrowserGate._browserType(engine).launch({ timeout: AuthoredNodeBrowserGate._timeout })
    try {
      await AuthoredNodeBrowserGate._checkPage(browser, origin, diagnostics)
      diagnostics.phase = 'focus-boundaries'
      await AuthoredNodeBrowserGate._checkBoundaries(browser, origin, diagnostics)
      diagnostics.phase = 'standalone-element'
      await AuthoredNodeBrowserGate._checkStandalone(browser, origin, diagnostics)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      console.log(JSON.stringify({ status: 'pass', engine, browserVersion: browser.version() }))
    } catch (error) {
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {RetentionDiagnostics} diagnostics @param {string} address @returns {Promise<import('playwright').Page>} */
  static async _open(browser, origin, diagnostics, address) {
    const page = await browser.newPage()
    page.on('pageerror', (error) => diagnostics.errors.push(error.message))
    page.on('requestfailed', (request) => diagnostics.failedRequests.push(request.url()))
    page.on('request', (request) => {
      if (new URL(request.url()).origin !== origin) diagnostics.externalRequests.push(request.url())
    })
    await page.addInitScript(AuthoredNodeBrowserGate._defineVendorHosts)
    await page.route(`${origin}/standalone.html`, (route) => route.fulfill({ contentType: 'text/html', body: AuthoredNodeBrowserGate._standaloneHost }))
    await page.goto(`${origin}${address}`, { waitUntil: 'networkidle', timeout: AuthoredNodeBrowserGate._timeout })
    return page
  }

  /** @type {() => void} */
  static _defineVendorHosts = () => {
    globalThis.observedCallbacks = 0
    customElements.define('unknown-wrap', class extends HTMLElement {
      connectedCallback() {
        if (this.querySelector('[data-vendor]')) return
        const wrapper = document.createElement('div')
        wrapper.dataset.vendor = 'opaque'
        wrapper.append(...this.childNodes)
        this.append(wrapper)
      }
    })
    customElements.define('observed-wrap', class extends HTMLElement {
      static observedAttributes = ['mode']
      attributeChangedCallback() {
        if (!this.isConnected) return
        globalThis.observedCallbacks += 1
        this.replaceChildren(document.createTextNode('vendor callback'))
      }
    })
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {RetentionDiagnostics} diagnostics @returns {Promise<void>} */
  static async _checkPage(browser, origin, diagnostics) {
    const page = await AuthoredNodeBrowserGate._open(browser, origin, diagnostics, '/')
    try {
      await page.locator('#vendor-card .w-card__content slot').waitFor()
      await page.waitForFunction(AuthoredNodeBrowserGate._scrollable, 'viewport')
      diagnostics.phase = 'graph-update'
      await AuthoredNodeBrowserGate._checkGraphUpdate(page)
      diagnostics.phase = 'graph-selection'
      await AuthoredNodeBrowserGate._checkGraphSelection(page)
      diagnostics.phase = 'replaced-loop-object'
      await AuthoredNodeBrowserGate._checkReplacedRow(page)
      diagnostics.phase = 'vendor-slot-changes'
      await AuthoredNodeBrowserGate._checkSlotChanges(page)
      diagnostics.phase = 'focused-input'
      await AuthoredNodeBrowserGate._checkFocusedInput(page)
      diagnostics.phase = 'conditional-removal'
      await AuthoredNodeBrowserGate._checkConditionalRemoval(page)
    } finally {
      await page.close()
    }
  }

  /** @type {(id: string) => boolean} */
  static _scrollable = (id) => {
    const viewport = document.getElementById(id)
    return Boolean(viewport) && viewport.scrollWidth > viewport.clientWidth && viewport.scrollHeight > viewport.clientHeight
  }

  /** @type {(tag: string) => boolean} */
  static _scrollableInside = (tag) => {
    const viewport = document.querySelector(tag)?.shadowRoot?.getElementById('viewport')
    return Boolean(viewport) && viewport.scrollWidth > viewport.clientWidth && viewport.scrollHeight > viewport.clientHeight
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkGraphUpdate(page) {
    const result = await page.evaluate(async (settle) => {
      const ids = ['vendor-container', 'native-parent', 'vendor-card', 'viewport', 'graphic', 'card-plan', 'card-build', 'card-ship']
      const nodes = ids.map((id) => document.getElementById(id))
      const viewport = document.getElementById('viewport')
      const wrapper = document.querySelector('#vendor-card .w-card__content')
      const opaque = document.getElementById('opaque')
      viewport.scrollLeft = 20
      viewport.scrollTop = 40
      await new Promise((resolve) => setTimeout(resolve, settle))
      const start = [viewport.scrollLeft, viewport.scrollTop]
      document.activeElement?.blur()
      for (let count = 0; count < 3; count += 1) {
        document.getElementById('update').click()
        await new Promise((resolve) => setTimeout(resolve, settle))
      }
      return {
        kept: ids.filter((id, index) => nodes[index] === document.getElementById(id)),
        start,
        pan: [viewport.scrollLeft, viewport.scrollTop],
        wrapperKept: wrapper === document.querySelector('#vendor-card .w-card__content'),
        opaqueReplaced: opaque !== document.getElementById('opaque'),
        updates: document.getElementById('updates').textContent,
        cardText: document.getElementById('card-ship').textContent,
        svgNamespace: document.getElementById('graphic').namespaceURI,
        cardNamespace: document.getElementById('card-plan').namespaceURI,
      }
    }, AuthoredNodeBrowserGate._settle)
    assert.deepEqual(result.kept, ['vendor-container', 'native-parent', 'vendor-card', 'viewport', 'graphic', 'card-plan', 'card-build', 'card-ship'])
    assert.ok(result.start[0] > 0 && result.start[1] > 0, 'the viewport scrolled before the update')
    assert.deepEqual(result.pan, result.start)
    assert.equal(result.wrapperKept, true)
    assert.equal(result.opaqueReplaced, true)
    assert.equal(result.updates, '3')
    assert.equal(result.cardText, '3 · Ship')
    assert.equal(result.svgNamespace, 'http://www.w3.org/2000/svg')
    assert.equal(result.cardNamespace, 'http://www.w3.org/1999/xhtml')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkGraphSelection(page) {
    const result = await page.evaluate(async (settle) => {
      const ids = ['card-plan', 'card-build', 'card-ship']
      const cards = ids.map((id) => document.getElementById(id))
      const viewport = document.getElementById('viewport')
      const start = [viewport.scrollLeft, viewport.scrollTop]
      cards[1].focus({ preventScroll: true })
      cards[1].click()
      await new Promise((resolve) => setTimeout(resolve, settle))
      return {
        kept: ids.filter((id, index) => cards[index] === document.getElementById(id)),
        pressed: ids.map((id) => document.getElementById(id).getAttribute('data-selected')),
        focused: document.activeElement === cards[1],
        panKept: start[0] > 0 && viewport.scrollLeft === start[0] && viewport.scrollTop === start[1],
        selections: document.getElementById('selections').textContent,
      }
    }, AuthoredNodeBrowserGate._settle)
    assert.deepEqual(result, {
      kept: ['card-plan', 'card-build', 'card-ship'],
      pressed: [null, '', null],
      focused: true,
      panKept: true,
      selections: '1',
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkReplacedRow(page) {
    const result = await page.evaluate(async (settle) => {
      const row = document.querySelector('.row')
      const id = row.getAttribute('data-id')
      row.focus()
      document.getElementById('replace-rows').click()
      await new Promise((resolve) => setTimeout(resolve, settle))
      const fresh = document.querySelector('.row')
      return {
        replaced: row !== fresh && !row.isConnected,
        sameId: fresh.getAttribute('data-id') === id,
        focused: fresh === document.activeElement,
        label: fresh.textContent,
      }
    }, AuthoredNodeBrowserGate._settle)
    assert.deepEqual(result, { replaced: true, sameId: true, focused: false, label: 'Replacement row' })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkSlotChanges(page) {
    const changes = await page.evaluate(async (settle) => {
      const results = []
      const steps = [
        () => document.querySelector('#mutation-card slot:not([name])').append(document.createElement('small')),
        () => document.getElementById('mutation-card').append(document.createElement('slot')),
        () => document.querySelector('#mutation-card .w-card__content').append(document.getElementById('ordinary-slot-child')),
        () => document.getElementById('change-slot').click(),
      ]
      for (const step of steps) {
        const card = document.getElementById('card-plan')
        const host = document.getElementById('mutation-card')
        document.activeElement?.blur()
        step()
        document.getElementById('update').click()
        await new Promise((resolve) => setTimeout(resolve, settle))
        results.push({ hostReplaced: host !== document.getElementById('mutation-card'), cardKept: card === document.getElementById('card-plan') })
      }
      return { results, slot: document.getElementById('named').getAttribute('slot') }
    }, AuthoredNodeBrowserGate._settle)
    const expected = { hostReplaced: true, cardKept: true }
    assert.deepEqual(changes, { results: [expected, expected, expected, expected], slot: 'actions' })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkFocusedInput(page) {
    const draft = page.locator('#draft')
    await draft.fill('abcdef')
    const result = await draft.evaluate(async (input, settle) => {
      const card = document.getElementById('card-plan')
      input.focus()
      input.setSelectionRange(1, 4, 'backward')
      document.getElementById('update').click()
      await new Promise((resolve) => setTimeout(resolve, settle))
      return {
        kept: input === document.getElementById('draft') && input === document.activeElement,
        selection: [input.selectionStart, input.selectionEnd, input.selectionDirection],
        cardKept: card === document.getElementById('card-plan'),
      }
    }, AuthoredNodeBrowserGate._settle)
    assert.deepEqual(result, { kept: true, selection: [1, 4, 'backward'], cardKept: true })
    const slotted = await page.locator('#slotted-draft').evaluate(async (input, settle) => {
      const host = document.getElementById('form-card')
      input.focus()
      input.setSelectionRange(2, 5, 'forward')
      document.getElementById('update').click()
      await new Promise((resolve) => setTimeout(resolve, settle))
      return {
        kept: input === document.getElementById('slotted-draft') && input === document.activeElement,
        hostKept: host === document.getElementById('form-card'),
        selection: [input.selectionStart, input.selectionEnd, input.selectionDirection],
      }
    }, AuthoredNodeBrowserGate._settle)
    assert.deepEqual(slotted, { kept: true, hostKept: true, selection: [2, 5, 'forward'] })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkConditionalRemoval(page) {
    const removed = await page.evaluate(async (settle) => {
      const old = document.getElementById('conditional')
      document.getElementById('hide').click()
      await new Promise((resolve) => setTimeout(resolve, settle))
      return !old.isConnected && !document.getElementById('conditional')
    }, AuthoredNodeBrowserGate._settle)
    assert.equal(removed, true)
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {RetentionDiagnostics} diagnostics @returns {Promise<void>} */
  static async _checkBoundaries(browser, origin, diagnostics) {
    const page = await AuthoredNodeBrowserGate._open(browser, origin, diagnostics, '/boundaries/')
    try {
      const wrapped = await page.locator('#wrapped').evaluate(async (input, settle) => {
        input.focus()
        input.setSelectionRange(1, 3, 'backward')
        document.getElementById('update').click()
        await new Promise((resolve) => setTimeout(resolve, settle))
        const fresh = document.getElementById('wrapped')
        return {
          replaced: input !== fresh && !input.isConnected,
          focused: fresh === document.activeElement,
          selection: [fresh.selectionStart, fresh.selectionEnd, fresh.selectionDirection],
        }
      }, AuthoredNodeBrowserGate._settle)
      assert.deepEqual(wrapped, { replaced: true, focused: true, selection: [1, 3, 'backward'] })
      const observed = await page.locator('#observed-input').evaluate(async (input, settle) => {
        input.focus()
        input.setSelectionRange(2, 2)
        document.getElementById('change-observed').click()
        await new Promise((resolve) => setTimeout(resolve, settle))
        const fresh = document.getElementById('observed-input')
        return {
          replaced: input !== fresh && !input.isConnected,
          focused: fresh === document.activeElement,
          caret: fresh.selectionStart,
          callbacks: globalThis.observedCallbacks,
        }
      }, AuthoredNodeBrowserGate._settle)
      assert.deepEqual(observed, { replaced: true, focused: true, caret: 2, callbacks: 0 })
    } finally {
      await page.close()
    }
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {RetentionDiagnostics} diagnostics @returns {Promise<void>} */
  static async _checkStandalone(browser, origin, diagnostics) {
    const page = await AuthoredNodeBrowserGate._open(browser, origin, diagnostics, '/standalone.html')
    try {
      await page.locator('retention-panel').locator('#card-ship').waitFor()
      await page.waitForFunction(AuthoredNodeBrowserGate._scrollableInside, 'retention-panel')
      const result = await page.locator('retention-panel').evaluate(async (host, settle) => {
        const root = host.shadowRoot
        const ids = ['card-plan', 'card-build', 'card-ship']
        const cards = ids.map((id) => root.getElementById(id))
        const viewport = root.getElementById('viewport')
        viewport.scrollLeft = 20
        viewport.scrollTop = 40
        await new Promise((resolve) => setTimeout(resolve, settle))
        const start = [viewport.scrollLeft, viewport.scrollTop]
        root.getElementById('update').click()
        await new Promise((resolve) => setTimeout(resolve, settle))
        return {
          kept: ids.filter((id, index) => cards[index] === root.getElementById(id)),
          panKept: start[0] > 0 && viewport.scrollLeft === start[0] && viewport.scrollTop === start[1],
          defaultSlot: Boolean(root.querySelector('#vendor-card slot:not([name])')?.contains(viewport)),
        }
      }, AuthoredNodeBrowserGate._settle)
      assert.deepEqual(result, { kept: ['card-plan', 'card-build', 'card-ship'], panKept: true, defaultSlot: true })
    } finally {
      await page.close()
    }
  }
}

await AuthoredNodeBrowserGate.run()

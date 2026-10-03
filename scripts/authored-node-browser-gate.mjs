#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} AuthoredBrowserEngine */
/** @typedef {{root: string, binary: string, sourceCommit: string, compilerSha256: string, engine: AuthoredBrowserEngine, receipt: string}} AuthoredBrowserFixture */
/** @typedef {AuthoredBrowserFixture & {phase: string, errors: string[], failedRequests: string[], externalRequests: string[], browserVersion?: string}} AuthoredBrowserDiagnostics */
/** @typedef {{identities: boolean[], pan: [number, number], wrapperRetained: boolean, opaqueReplaced: boolean, count: string | null, svg: string | null, html: string | null, vendorHostClasses: boolean, authoredClasses: boolean}} AuthoredRetentionProof */
/** @typedef {{replaced: boolean, sameTextId: boolean, focused: boolean, label: string | null}} AuthoredLexicalProof */
/** @typedef {{opaqueReplaced: boolean, graphRetained: boolean}} AuthoredInjectedSlotProof */
/** @typedef {{replaced: boolean, graphRetained: boolean}} AuthoredSlotReplacementProof */
/** @typedef {AuthoredSlotReplacementProof & {slot: string | null}} AuthoredSlotReassignmentProof */
/** @typedef {{injected: AuthoredInjectedSlotProof, duplicate: AuthoredSlotReplacementProof, moved: AuthoredSlotReplacementProof, reassigned: AuthoredSlotReassignmentProof}} AuthoredSlotProof */
/** @typedef {[number | null, number | null, 'forward' | 'backward' | 'none' | null]} AuthoredSelection */
/** @typedef {{retained: boolean, selection: AuthoredSelection}} AuthoredFocusedControlProof */
/** @typedef {{replaced: boolean, focused: boolean, selection: AuthoredSelection, nestedHostRetained: boolean, nestedViewRetained: boolean, pendingStarts: string | null}} AuthoredWrappedFocusProof */
/** @typedef {{replaced: boolean, focused: boolean, caret: number | null, liveChanges: number}} AuthoredObservedFocusProof */
/** @typedef {{hostRetained: boolean, viewRetained: boolean, connected: boolean, starts: string | null, updates: string | null, active: string | null}} AuthoredNestedMountProof */
/** @typedef {{removed: boolean, starts: number, pendingCancelled: boolean}} AuthoredCancelledMountProof */
/** @typedef {{ordinary: AuthoredFocusedControlProof, wrapped: AuthoredWrappedFocusProof, observed: AuthoredObservedFocusProof, nested: AuthoredNestedMountProof, cancellation: AuthoredCancelledMountProof}} AuthoredFocusProof */
/** @typedef {typeof globalThis & {connectedObservedChanges: number}} AuthoredObservedWindow */
/** @typedef {typeof globalThis & {retentionIdleCallbacks: Map<number, IdleRequestCallback>, retentionIdleSequence: number}} AuthoredIdleWindow */
/** @typedef {{retained: boolean, pan: [number, number], defaultSlot: boolean | undefined}} AuthoredStandaloneProof */
/** @typedef {{ordinary: AuthoredRetentionProof, lexical: AuthoredLexicalProof, slots: AuthoredSlotProof, focus: AuthoredFocusProof, removed: boolean, exported: AuthoredStandaloneProof, genuinePinnedDuVay: boolean, freshSnapshotRowsRetained: boolean}} AuthoredBrowserProof */

class AuthoredNodeBrowserGate {
  /** @type {number} */
  static #timeout = 15000
  /** @type {string} */
  static #duvayHash = 'bd252fed5bb8a90f4c4f9eb1ac8d948e8b5c9dcf88e35ccfe30915ba9d2960de'
  /** @type {string} */
  static #host = '<!doctype html><html><head><script src="/shared/assets/duvay/duvay-wc.min.js"></script><script type="module" src="/.tachyon/elements/retention-panel.js"></script></head><body><retention-panel></retention-panel></body></html>'

  /** @returns {Promise<void>} */
  static async run() {
    const fixture = await AuthoredNodeBrowserGate.#fixture()
    await AuthoredNodeBrowserGate.#execute(fixture)
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} identifies the retained qualification input`)
    return value
  }

  /** @returns {AuthoredBrowserEngine} */
  static #engine() {
    const engine = process.env.TAC_AUTHORED_NODE_BROWSER ?? 'chromium'
    assert.ok(['chromium', 'firefox', 'webkit'].includes(engine))
    return /** @type {AuthoredBrowserEngine} */ (engine)
  }

  /** @param {AuthoredBrowserEngine} engine @returns {import('playwright').BrowserType} */
  static #browser(engine) {
    return { chromium, firefox, webkit }[engine]
  }

  /** @returns {Promise<AuthoredBrowserFixture>} */
  static async #fixture() {
    const root = path.resolve(AuthoredNodeBrowserGate.#required('TAC_AUTHORED_NODE_FIXTURE_OUT'))
    const binary = path.resolve(AuthoredNodeBrowserGate.#required('TAC_BIN'))
    const sourceCommit = AuthoredNodeBrowserGate.#required('TAC_AUTHORED_NODE_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const engine = AuthoredNodeBrowserGate.#engine()
    const receipt = path.join(root, `authored-node-browser-${engine}-${randomUUID()}.json`)
    return { root, binary, sourceCommit, compilerSha256, engine, receipt }
  }

  /** @param {AuthoredBrowserFixture} fixture @returns {Promise<void>} */
  static async #execute(fixture) {
    const diagnostics = { ...fixture, phase: 'starting', errors: [], failedRequests: [], externalRequests: [] }
    const started = performance.now()
    try {
      const proof = await withServer(fixture.binary, ['preview', fixture.root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return AuthoredNodeBrowserGate.#probe(origin, diagnostics)
      })
      await writeFile(fixture.receipt, JSON.stringify({ ...diagnostics, status: 'pass', proof, runtimeReplaced: false, elapsedMilliseconds: performance.now() - started }, null, 2) + '\n')
      console.log(JSON.stringify({ status: 'pass', receipt: fixture.receipt, proof }))
    } catch (error) {
      await writeFile(fixture.receipt, JSON.stringify({ ...diagnostics, status: 'fail', error: String(error), runtimeReplaced: false, elapsedMilliseconds: performance.now() - started }, null, 2) + '\n')
      throw error
    }
  }

  /** @param {string} origin @param {AuthoredBrowserDiagnostics} diagnostics @returns {Promise<AuthoredBrowserProof>} */
  static async #probe(origin, diagnostics) {
    const vendor = await fetch(`${origin}/shared/assets/duvay/duvay-wc.min.js`)
    assert.equal(createHash('sha256').update(Buffer.from(await vendor.arrayBuffer())).digest('hex'), AuthoredNodeBrowserGate.#duvayHash)
    const browser = await AuthoredNodeBrowserGate.#browser(diagnostics.engine).launch({ timeout: AuthoredNodeBrowserGate.#timeout })
    diagnostics.browserVersion = browser.version()
    try {
      const page = await browser.newPage()
      AuthoredNodeBrowserGate.#observe(page, origin, diagnostics)
      await page.addInitScript(AuthoredNodeBrowserGate.customHosts)
      await page.goto(origin, { waitUntil: 'networkidle', timeout: AuthoredNodeBrowserGate.#timeout })
      await page.locator('#vendor-card .w-card__content slot').waitFor()
      diagnostics.phase = 'pinned-duvay-same-record-retention'
      const ordinary = await AuthoredNodeBrowserGate.#retention(page)
      diagnostics.phase = 'listener-and-loop-object-boundaries'
      const lexical = await AuthoredNodeBrowserGate.#lexical(page)
      diagnostics.phase = 'opaque-slot-subtree-sibling-retention'
      const slots = await AuthoredNodeBrowserGate.#slotBoundaries(page)
      diagnostics.phase = 'focused-control-and-custom-fallback'
      const focus = await AuthoredNodeBrowserGate.#focus(page, browser, origin, diagnostics)
      diagnostics.phase = 'conditional-removal'
      const removed = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
        const old = query(document, '#conditional', Element)
        query(document, '#hide', HTMLButtonElement).click()
        await new Promise(resolve => setTimeout(resolve, 80))
        return !old.isConnected && !document.getElementById('conditional')
      })
      assert.equal(removed, true)
      diagnostics.phase = 'standalone-exported-root'
      const standalone = await browser.newPage()
      AuthoredNodeBrowserGate.#observe(standalone, origin, diagnostics)
      await standalone.addInitScript(AuthoredNodeBrowserGate.customHosts)
      await standalone.route(`${origin}/retention-host.html`, route => route.fulfill({ contentType: 'text/html', body: AuthoredNodeBrowserGate.#host }))
      await standalone.goto(`${origin}/retention-host.html`, { waitUntil: 'networkidle' })
      await standalone.locator('retention-panel').locator('#card').waitFor()
      const exported = await AuthoredNodeBrowserGate.#standalone(standalone)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      return { ordinary, lexical, slots, focus, removed, exported, genuinePinnedDuVay: true, freshSnapshotRowsRetained: false }
    } finally { await browser.close() }
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {AuthoredBrowserDiagnostics} diagnostics @returns {void} */
  static #observe(page, origin, diagnostics) {
    page.on('pageerror', error => diagnostics.errors.push(error.message))
    page.on('requestfailed', request => diagnostics.failedRequests.push(request.url()))
    page.on('request', request => { if (new URL(request.url()).origin !== origin) diagnostics.externalRequests.push(request.url()) })
  }

  /** @returns {void} */
  static customHosts() {
    const observed = /** @type {AuthoredObservedWindow} */ (globalThis)
    observed.connectedObservedChanges = 0
    customElements.define('unknown-wrap', class extends HTMLElement {
      /** @returns {void} */
      connectedCallback() {
        if (this.querySelector('[data-vendor]')) return
        const wrapper = document.createElement('div')
        wrapper.dataset.vendor = 'opaque'
        wrapper.append(...this.childNodes)
        this.append(wrapper)
      }
    })
    customElements.define('observed-wrap', class extends HTMLElement {
      /** @type {string[]} */
      static observedAttributes = ['mode']
      /** @returns {void} */
      attributeChangedCallback() {
        if (!this.isConnected) return
        observed.connectedObservedChanges += 1
        this.replaceChildren(document.createTextNode('vendor callback'))
      }
    })
  }

  /** @returns {void} */
  static idleQueue() {
    const idle = /** @type {AuthoredIdleWindow} */ (globalThis)
    idle.retentionIdleCallbacks = new Map()
    idle.retentionIdleSequence = 0
    /** @param {IdleRequestCallback} callback @returns {number} */
    globalThis.requestIdleCallback = callback => {
      const id = ++idle.retentionIdleSequence
      idle.retentionIdleCallbacks.set(id, callback)
      return id
    }
    /** @param {number} id @returns {void} */
    globalThis.cancelIdleCallback = id => { idle.retentionIdleCallbacks.delete(id) }
  }

  /** @param {import('playwright').Page} page @returns {Promise<AuthoredRetentionProof>} */
  static async #retention(page) {
    const result = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const nodes = ['vendor-container', 'native-parent', 'vendor-card', 'viewport', 'graphic', 'card'].map(id => query(document, `#${id}`, Element))
      const wrappers = query(nodes[2], '.w-card__content', Element)
      const opaque = query(document, '#opaque', Element)
      nodes[3].scrollLeft = 20; nodes[3].scrollTop = 40
      const active = document.activeElement
      if (active instanceof HTMLElement || active instanceof SVGElement) active.blur()
      for (let index = 0; index < 3; index += 1) {
        query(document, '#update', HTMLButtonElement).click()
        await new Promise(resolve => setTimeout(resolve, 80))
      }
      /** @type {AuthoredRetentionProof} */
      const proof = { identities: nodes.map(node => node === document.getElementById(node.id)), pan: [nodes[3].scrollLeft, nodes[3].scrollTop],
        wrapperRetained: wrappers === document.querySelector('#vendor-card .w-card__content'), opaqueReplaced: opaque !== document.getElementById('opaque'),
        count: query(document, '#updates', HTMLOutputElement).textContent, svg: nodes[4].namespaceURI, html: nodes[5].namespaceURI,
        vendorHostClasses: nodes[0].classList.contains('w-container') && nodes[0].classList.contains('w-container--fluid'),
        authoredClasses: nodes[0].classList.contains('panel-mode-3') && !nodes[0].classList.contains('panel-mode-0') && nodes[2].classList.contains('graph-mode-3') }
      return proof
    })
    assert.deepEqual(result.identities, [true, true, true, true, true, true])
    assert.deepEqual(result.pan, [20, 40])
    assert.equal(result.wrapperRetained, true)
    assert.equal(result.opaqueReplaced, true)
    assert.equal(result.count, '3')
    assert.equal(result.vendorHostClasses, true)
    assert.equal(result.authoredClasses, true)
    assert.equal(result.svg, 'http://www.w3.org/2000/svg')
    assert.equal(result.html, 'http://www.w3.org/1999/xhtml')
    return result
  }

  /** @param {import('playwright').Page} page @returns {Promise<AuthoredLexicalProof>} */
  static async #lexical(page) {
    await page.locator('#card').click()
    await page.waitForFunction(() => document.getElementById('selections')?.textContent === '1')
    const result = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const row = query(document, '.row', HTMLButtonElement)
      const id = row.getAttribute('data-id')
      row.focus()
      query(document, '#replace-rows', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      const fresh = query(document, '.row', HTMLButtonElement)
      return { replaced: row !== fresh && !row.isConnected, sameTextId: fresh.getAttribute('data-id') === id, focused: fresh === document.activeElement, label: fresh.textContent }
    })
    assert.deepEqual(result, { replaced: true, sameTextId: true, focused: false, label: 'Replacement row' })
    await page.locator('.row').click()
    await page.waitForFunction(() => document.getElementById('selections')?.textContent === '2')
    return result
  }

  /** @param {import('playwright').Page} page @returns {Promise<AuthoredSlotProof>} */
  static async #slotBoundaries(page) {
    const injected = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const graph = query(document, '#card', HTMLButtonElement)
      const host = query(document, '#mutation-card', HTMLElement)
      const slot = query(host, 'slot:not([name])', HTMLSlotElement)
      slot.append(document.createElement('small'))
      const active = document.activeElement
      if (active instanceof HTMLElement || active instanceof SVGElement) active.blur()
      query(document, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      return { opaqueReplaced: host !== document.getElementById('mutation-card'), graphRetained: graph === document.getElementById('card') }
    })
    assert.deepEqual(injected, { opaqueReplaced: true, graphRetained: true })
    const duplicate = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const graph = query(document, '#card', HTMLButtonElement)
      const host = query(document, '#mutation-card', HTMLElement)
      host.append(document.createElement('slot'))
      query(document, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      return { replaced: host !== document.getElementById('mutation-card'), graphRetained: graph === document.getElementById('card') }
    })
    assert.deepEqual(duplicate, { replaced: true, graphRetained: true })
    const moved = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const graph = query(document, '#card', HTMLButtonElement)
      const host = query(document, '#mutation-card', HTMLElement)
      query(host, '.w-card__content', HTMLElement).append(query(document, '#ordinary-slot-child', HTMLParagraphElement))
      query(document, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      return { replaced: host !== document.getElementById('mutation-card'), graphRetained: graph === document.getElementById('card') }
    })
    assert.deepEqual(moved, { replaced: true, graphRetained: true })
    const reassigned = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const graph = query(document, '#card', HTMLButtonElement)
      const host = query(document, '#mutation-card', HTMLElement)
      query(document, '#change-slot', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      return { replaced: host !== document.getElementById('mutation-card'), graphRetained: graph === document.getElementById('card'),
        slot: query(document, '#named', HTMLSpanElement).getAttribute('slot') }
    })
    assert.deepEqual(reassigned, { replaced: true, graphRetained: true, slot: 'actions' })
    return { injected, duplicate, moved, reassigned }
  }

  /** @param {import('playwright').Page} page @param {import('playwright').Browser} browser @param {string} origin @param {AuthoredBrowserDiagnostics} diagnostics @returns {Promise<AuthoredFocusProof>} */
  static async #focus(page, browser, origin, diagnostics) {
    const draft = page.locator('#draft')
    await draft.fill('abcdef')
    const ordinary = await draft.evaluate(async input => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      if (!(input instanceof HTMLInputElement)) throw new Error('Fixture editing control must be an input')
      input.focus(); input.setSelectionRange(1, 4, 'backward')
      query(document, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      /** @type {AuthoredFocusedControlProof} */
      const proof = { retained: input === document.getElementById('draft') && input === document.activeElement,
        selection: [input.selectionStart, input.selectionEnd, input.selectionDirection] }
      return proof
    })
    assert.deepEqual(ordinary, { retained: true, selection: [1, 4, 'backward'] })
    const boundary = await browser.newPage()
    AuthoredNodeBrowserGate.#observe(boundary, origin, diagnostics)
    await boundary.addInitScript(AuthoredNodeBrowserGate.customHosts)
    await boundary.addInitScript(AuthoredNodeBrowserGate.idleQueue)
    await boundary.goto(`${origin}/boundaries/`, { waitUntil: 'networkidle' })
    await boundary.locator('#nested-child-view').waitFor()
    assert.equal(await boundary.locator('#child-starts').textContent(), '0')
    const wrapped = await boundary.locator('#wrapped').evaluate(async input => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      if (!(input instanceof HTMLInputElement)) throw new Error('Fixture editing control must be an input')
      const child = query(document, '[data-tachyon-component="retention-child"]', HTMLElement)
      const view = query(document, '#nested-child-view', HTMLElement)
      input.focus(); input.setSelectionRange(1, 3, 'backward')
      query(document, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      const fresh = query(document, '#wrapped', HTMLInputElement)
      /** @type {AuthoredWrappedFocusProof} */
      const proof = { replaced: input !== fresh && !input.isConnected, focused: fresh === document.activeElement,
        selection: [fresh.selectionStart, fresh.selectionEnd, fresh.selectionDirection],
        nestedHostRetained: child === document.querySelector('[data-tachyon-component="retention-child"]') && child.isConnected,
        nestedViewRetained: view === document.getElementById('nested-child-view') && view.isConnected,
        pendingStarts: query(document, '#child-starts', HTMLOutputElement).textContent }
      return proof
    })
    assert.deepEqual(wrapped, { replaced: true, focused: true, selection: [1, 3, 'backward'], nestedHostRetained: true, nestedViewRetained: true, pendingStarts: '0' })
    const observed = await boundary.locator('#observed-input').evaluate(async input => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      if (!(input instanceof HTMLInputElement)) throw new Error('Fixture editing control must be an input')
      input.focus(); input.setSelectionRange(2, 2)
      query(document, '#change-observed', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      const fresh = query(document, '#observed-input', HTMLInputElement)
      return { replaced: input !== fresh && !input.isConnected, focused: fresh === document.activeElement,
        caret: fresh.selectionStart, liveChanges: (/** @type {AuthoredObservedWindow} */ (globalThis)).connectedObservedChanges }
    })
    assert.deepEqual(observed, { replaced: true, focused: true, caret: 2, liveChanges: 0 })
    const nested = await AuthoredNodeBrowserGate.#nestedMount(boundary)
    const cancellation = await AuthoredNodeBrowserGate.#cancelledMount(browser, origin, diagnostics)
    return { ordinary, wrapped, observed, nested, cancellation }
  }

  /** @param {import('playwright').Page} page @returns {Promise<AuthoredNestedMountProof>} */
  static async #nestedMount(page) {
    const result = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const child = query(document, '[data-tachyon-component="retention-child"]', HTMLElement)
      const view = query(document, '#nested-child-view', HTMLElement)
      const idle = /** @type {AuthoredIdleWindow} */ (globalThis)
      const callbacks = [...idle.retentionIdleCallbacks.values()]
      for (const callback of callbacks) await callback({ didTimeout: false, timeRemaining: () => 50 })
      await new Promise(resolve => setTimeout(resolve, 80))
      query(document, '#child-update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      query(document, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      for (const callback of callbacks) await callback({ didTimeout: false, timeRemaining: () => 50 })
      await new Promise(resolve => setTimeout(resolve, 80))
      return { hostRetained: child === document.querySelector('[data-tachyon-component="retention-child"]'),
        viewRetained: view === document.getElementById('nested-child-view'), connected: child.isConnected && view.isConnected,
        starts: query(document, '#child-starts', HTMLOutputElement).textContent, updates: query(document, '#child-updates', HTMLOutputElement).textContent,
        active: child.getAttribute('data-tachyon-active') }
    })
    assert.deepEqual(result, { hostRetained: true, viewRetained: true, connected: true, starts: '1', updates: '1', active: 'true' })
    return result
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {AuthoredBrowserDiagnostics} diagnostics @returns {Promise<AuthoredCancelledMountProof>} */
  static async #cancelledMount(browser, origin, diagnostics) {
    const page = await browser.newPage()
    AuthoredNodeBrowserGate.#observe(page, origin, diagnostics)
    await page.addInitScript(AuthoredNodeBrowserGate.customHosts)
    await page.addInitScript(AuthoredNodeBrowserGate.idleQueue)
    await page.goto(`${origin}/boundaries/`, { waitUntil: 'networkidle' })
    await page.locator('#nested-child-view').waitFor()
    const result = await page.evaluate(async () => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const child = query(document, '[data-tachyon-component="retention-child"]', HTMLElement)
      if (!('tachyonComponent' in child)) throw new Error('Nested child lacks its actual component owner')
      const component = child.tachyonComponent
      if (!component || typeof component !== 'object' || !('instance' in component)) throw new Error('Nested child component owner is invalid')
      const owner = component.instance
      if (!owner || typeof owner !== 'object' || !('retentionChild' in owner)) throw new Error('Nested child Service is missing')
      const service = owner.retentionChild
      if (!service || typeof service !== 'object' || !('starts' in service) || typeof service.starts !== 'number') throw new Error('Nested child Service start counter is invalid')
      const idle = /** @type {AuthoredIdleWindow} */ (globalThis)
      const callbacks = [...idle.retentionIdleCallbacks.values()]
      const scheduled = idle.retentionIdleCallbacks.size
      query(document, '#hide-child', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      const pendingCancelled = idle.retentionIdleCallbacks.size < scheduled
      for (const callback of callbacks) await callback({ didTimeout: false, timeRemaining: () => 50 })
      await new Promise(resolve => setTimeout(resolve, 80))
      return { removed: !child.isConnected && !document.getElementById('nested-child-view'),
        starts: service.starts, pendingCancelled }
    })
    assert.deepEqual(result, { removed: true, starts: 0, pendingCancelled: true })
    return result
  }

  /** @param {import('playwright').Page} page @returns {Promise<AuthoredStandaloneProof>} */
  static async #standalone(page) {
    const result = await page.locator('retention-panel').evaluate(async host => {
      /** @template {Element} T @param {ParentNode} root @param {string} selector @param {new () => T} kind @returns {T} */
      const query = (root, selector, kind) => {
        const element = root.querySelector(selector)
        if (!(element instanceof kind)) throw new Error(`Missing or invalid fixture element: ${selector}`)
        return element
      }
      const root = host.shadowRoot
      if (!root) throw new Error('Exported component lacks its shadow root')
      const card = query(root, '#card', HTMLButtonElement)
      const viewport = query(root, '#viewport', HTMLDivElement)
      viewport.scrollLeft = 20; viewport.scrollTop = 40
      query(root, '#update', HTMLButtonElement).click()
      await new Promise(resolve => setTimeout(resolve, 80))
      /** @type {AuthoredStandaloneProof} */
      const proof = { retained: card === root.querySelector('#card'), pan: [viewport.scrollLeft, viewport.scrollTop],
        defaultSlot: root.querySelector('#vendor-card slot:not([name])')?.contains(viewport) }
      return proof
    })
    assert.deepEqual(result, { retained: true, pan: [20, 40], defaultSlot: true })
    return result
  }
}

await AuthoredNodeBrowserGate.run()

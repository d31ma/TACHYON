#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} ScopeEngine */
/** @typedef {{background: string, outline: string, color: string, font: string, token: string, owner: string | null}} ScopeStyle */
/** @typedef {{phase: string, sourceCommit: string, compilerSha256: string, engine: ScopeEngine, browserVersion?: string, errors: string[], failedRequests: string[], externalRequests: string[]}} ScopeDiagnostics */

class ComponentScopeBrowserGate {
  /** @type {number} */
  static #timeout = 15_000
  /** @type {string} */
  static #parentColor = 'rgb(12, 34, 56)'
  /** @type {string} */
  static #parentBackground = 'rgb(240, 20, 20)'
  /** @type {string} */
  static #childBackground = 'rgb(20, 140, 20)'
  /** @type {string} */
  static #transparent = 'rgba(0, 0, 0, 0)'

  /** @returns {Promise<void>} */
  static async run() {
    const root = path.resolve(ComponentScopeBrowserGate.#required('TAC_COMPONENT_SCOPE_FIXTURE_OUT'))
    const binary = path.resolve(ComponentScopeBrowserGate.#required('TAC_BIN'))
    const sourceCommit = ComponentScopeBrowserGate.#required('TAC_COMPONENT_SCOPE_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const engine = ComponentScopeBrowserGate.#engine()
    const diagnostics = { phase: 'starting', sourceCommit, compilerSha256, engine, errors: [], failedRequests: [], externalRequests: [] }
    await ComponentScopeBrowserGate.#execute(root, binary, diagnostics)
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} identifies this retained fixture`)
    return value
  }

  /** @returns {ScopeEngine} */
  static #engine() {
    const engine = process.env.TAC_COMPONENT_SCOPE_BROWSER ?? 'chromium'
    assert.ok(engine === 'chromium' || engine === 'firefox' || engine === 'webkit')
    return engine
  }

  /** @param {ScopeEngine} engine @returns {import('playwright').BrowserType} */
  static #browser(engine) {
    switch (engine) {
      case 'chromium': return chromium
      case 'firefox': return firefox
      case 'webkit': return webkit
    }
  }

  /** @param {string} root @param {string} binary @param {ScopeDiagnostics} diagnostics @returns {Promise<void>} */
  static async #execute(root, binary, diagnostics) {
    const started = performance.now()
    const receipt = path.join(root, `component-scope-browser-${diagnostics.engine}-${randomUUID()}.json`)
    try {
      const proof = await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return ComponentScopeBrowserGate.#probe(origin, diagnostics)
      })
      await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'pass', proof, elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      console.log(JSON.stringify({ status: 'pass', engine: diagnostics.engine, receipt, proof }))
    } catch (error) {
      await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'fail', error: String(error), elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      throw error
    }
  }

  /** @param {string} origin @param {ScopeDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(origin, diagnostics) {
    const browser = await ComponentScopeBrowserGate.#browser(diagnostics.engine).launch({ timeout: ComponentScopeBrowserGate.#timeout })
    diagnostics.browserVersion = browser.version()
    try {
      const page = await browser.newPage()
      ComponentScopeBrowserGate.#observe(page, origin, diagnostics)
      await page.goto(origin, { waitUntil: 'networkidle', timeout: ComponentScopeBrowserGate.#timeout })
      await page.locator('#first-parent [data-probe="parent"]').waitFor()
      diagnostics.phase = 'nested-and-slotted-ownership'
      const first = await ComponentScopeBrowserGate.#parent(page, '#first-parent')
      const second = await ComponentScopeBrowserGate.#parent(page, '#second-parent')
      const neighbor = await ComponentScopeBrowserGate.#style(page, '#neighbor [data-probe="plain"]')
      const delegated = await ComponentScopeBrowserGate.#style(page, '#delegated [data-probe="plain"]')
      assert.equal(neighbor.background, ComponentScopeBrowserGate.#transparent)
      assert.equal(delegated.background, ComponentScopeBrowserGate.#transparent)
      assert.equal(neighbor.owner, 'plain-child')
      assert.equal(delegated.owner, 'plain-child')
      diagnostics.phase = 'dispose-and-remount'
      await page.getByRole('button', { name: 'Hide parent', exact: true }).click()
      await page.locator('#first-parent [data-probe="parent"]').waitFor({ state: 'detached' })
      assert.deepEqual(await ComponentScopeBrowserGate.#parent(page, '#second-parent'), second)
      await page.getByRole('button', { name: 'Show parent', exact: true }).click()
      await page.locator('#first-parent [data-probe="parent"]').waitFor()
      assert.deepEqual(await ComponentScopeBrowserGate.#parent(page, '#first-parent'), first)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      return { first, second, neighbor, delegated, independentInstances: true, remounted: true, inheritedColorFontAndVariable: true }
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {ScopeDiagnostics} diagnostics @returns {void} */
  static #observe(page, origin, diagnostics) {
    page.on('pageerror', error => diagnostics.errors.push(String(error)))
    page.on('requestfailed', request => diagnostics.failedRequests.push(new URL(request.url()).pathname))
    page.on('request', request => ComponentScopeBrowserGate.#external(request.url(), origin, diagnostics))
  }

  /** @param {string} url @param {string} origin @param {ScopeDiagnostics} diagnostics @returns {void} */
  static #external(url, origin, diagnostics) {
    if (url.startsWith('http') && new URL(url).origin !== origin) diagnostics.externalRequests.push(url)
  }

  /** @param {import('playwright').Page} page @param {string} parent @returns {Promise<Record<string, ScopeStyle>>} */
  static async #parent(page, parent) {
    const owned = await ComponentScopeBrowserGate.#style(page, `${parent} [data-probe="parent"]`)
    const styled = await ComponentScopeBrowserGate.#style(page, `${parent} [data-probe="styled"]`)
    const plain = await ComponentScopeBrowserGate.#style(page, `${parent} [data-probe="plain"]`)
    const slot = await ComponentScopeBrowserGate.#style(page, `${parent} [data-probe="slot"]`)
    assert.equal(owned.background, ComponentScopeBrowserGate.#parentBackground)
    assert.equal(owned.outline, '3px')
    assert.equal(styled.background, ComponentScopeBrowserGate.#childBackground)
    assert.equal(styled.outline, '0px')
    ComponentScopeBrowserGate.#inherited(styled, 'styled-child')
    ComponentScopeBrowserGate.#inherited(plain, 'plain-child')
    ComponentScopeBrowserGate.#inherited(slot, 'slot-child')
    assert.equal(plain.background, ComponentScopeBrowserGate.#transparent)
    assert.equal(slot.background, ComponentScopeBrowserGate.#transparent)
    assert.equal(plain.outline, '0px')
    assert.equal(slot.outline, '0px')
    return { owned, styled, plain, slot }
  }

  /** @param {ScopeStyle} style @param {string} owner @returns {void} */
  static #inherited(style, owner) {
    assert.equal(style.color, ComponentScopeBrowserGate.#parentColor)
    assert.ok(style.font.includes('monospace'))
    assert.equal(style.token, 'parent')
    assert.equal(style.owner, owner)
  }

  /** @param {import('playwright').Page} page @param {string} selector @returns {Promise<ScopeStyle>} */
  static #style(page, selector) {
    return page.locator(selector).evaluate(element => {
      const computed = getComputedStyle(element)
      return { background: computed.backgroundColor, outline: computed.outlineWidth, color: computed.color, font: computed.fontFamily, token: computed.getPropertyValue('--owner-token').trim(), owner: element.closest('tachyon-component')?.getAttribute('data-tac-scope') ?? null }
    })
  }
}

await ComponentScopeBrowserGate.run()

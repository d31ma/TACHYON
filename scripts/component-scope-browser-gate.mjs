#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { createHash, randomUUID } from 'node:crypto'
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} ScopeEngine */
/** @typedef {{background: string, outline: string, outlineStyle: string, color: string, font: string, token: string, scopeToken: string, owner: string | null}} ScopeStyle */
/** @typedef {{phase: string, sourceCommit: string, compilerSha256: string, engine: ScopeEngine, browserVersion?: string, errors: string[], failedRequests: string[], externalRequests: string[]}} ScopeDiagnostics */
/** @typedef {{root: string, retained: boolean, sourceCommit: string, compilerSha256: string}} ScopeRun */

class ComponentScopeBrowserGate {
  /** @type {number} */
  static #timeout = 15_000
  static #engines = ['chromium', 'firefox', 'webkit']
  static #repository = fileURLToPath(new URL('..', import.meta.url))
  static #fixture = 'crates/tachyon-core/tests/fixtures/tac-component-scope'
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
    const binary = ComponentScopeBrowserGate.#binary()
    const engines = ComponentScopeBrowserGate.#selectedEngines()
    const retainedRoot = process.env.TAC_COMPONENT_SCOPE_FIXTURE_OUT
    const root = retainedRoot ? path.resolve(retainedRoot) : await mkdtemp(path.join(tmpdir(), 'tac-component-scope-'))
    try {
      await cp(path.join(ComponentScopeBrowserGate.#repository, ComponentScopeBrowserGate.#fixture), root, { recursive: true })
      await promisify(execFile)(binary, ['bundle', root])
      /** @type {ScopeRun} */
      const run = {
        root,
        retained: Boolean(retainedRoot),
        sourceCommit: process.env.TAC_COMPONENT_SCOPE_SOURCE_COMMIT ?? await ComponentScopeBrowserGate.#headCommit(),
        compilerSha256: createHash('sha256').update(await readFile(binary)).digest('hex'),
      }
      await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        for (const engine of engines) await ComponentScopeBrowserGate.#runEngine(engine, origin, run)
      })
    } finally {
      if (!retainedRoot) await rm(root, { recursive: true, force: true })
    }
  }

  /** @param {ScopeEngine} engine @param {string} origin @param {ScopeRun} run @returns {Promise<void>} */
  static async #runEngine(engine, origin, run) {
    /** @type {ScopeDiagnostics} */
    const diagnostics = { phase: 'starting', sourceCommit: run.sourceCommit, compilerSha256: run.compilerSha256, engine, errors: [], failedRequests: [], externalRequests: [] }
    const receipt = path.join(run.root, `component-scope-browser-${engine}-${randomUUID()}.json`)
    const started = performance.now()
    try {
      const proof = await ComponentScopeBrowserGate.#probe(origin, diagnostics)
      if (run.retained) await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'pass', proof, elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      console.log(JSON.stringify({ status: 'pass', engine, browserVersion: diagnostics.browserVersion, receipt: run.retained ? receipt : null }))
    } catch (error) {
      if (run.retained) await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'fail', error: String(error), elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    }
  }

  /** @returns {Promise<string>} */
  static async #headCommit() {
    const { stdout } = await promisify(execFile)('git', ['rev-parse', 'HEAD'], { cwd: ComponentScopeBrowserGate.#repository })
    return stdout.trim()
  }

  /** @returns {string} */
  static #binary() {
    const binary = process.env.TAC_BIN
    assert.ok(binary, 'TAC_BIN names the ty binary under test')
    return path.resolve(binary)
  }

  /** @returns {ScopeEngine[]} */
  static #selectedEngines() {
    const requested = process.env.TAC_COMPONENT_SCOPE_BROWSERS
    if (!requested) return /** @type {ScopeEngine[]} */ (ComponentScopeBrowserGate.#engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(ComponentScopeBrowserGate.#engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {ScopeEngine[]} */ (engines)
  }

  /** @param {ScopeEngine} engine @returns {import('playwright').BrowserType} */
  static #browser(engine) {
    switch (engine) {
      case 'chromium': return chromium
      case 'firefox': return firefox
      case 'webkit': return webkit
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
      assert.equal(neighbor.token, '')
      assert.notEqual(neighbor.color, ComponentScopeBrowserGate.#parentColor)
      assert.equal(neighbor.scopeToken, 'plain-child')
      assert.equal(delegated.scopeToken, 'plain-child')
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
    assert.equal(owned.outlineStyle, 'solid')
    assert.equal(owned.scopeToken, 'parent-panel')
    assert.equal(styled.background, ComponentScopeBrowserGate.#childBackground)
    assert.equal(styled.outlineStyle, 'none')
    ComponentScopeBrowserGate.#inherited(styled, 'styled-child')
    ComponentScopeBrowserGate.#inherited(plain, 'plain-child')
    ComponentScopeBrowserGate.#inherited(slot, 'slot-child')
    assert.equal(plain.background, ComponentScopeBrowserGate.#transparent)
    assert.equal(slot.background, ComponentScopeBrowserGate.#transparent)
    assert.equal(plain.outlineStyle, 'none')
    assert.equal(slot.outlineStyle, 'none')
    return { owned, styled, plain, slot }
  }

  /** @param {ScopeStyle} style @param {string} owner @returns {void} */
  static #inherited(style, owner) {
    assert.equal(style.color, ComponentScopeBrowserGate.#parentColor)
    assert.ok(style.font.includes('monospace'))
    assert.equal(style.token, 'parent')
    assert.equal(style.owner, owner)
    assert.equal(style.scopeToken, owner)
  }

  /** @param {import('playwright').Page} page @param {string} selector @returns {Promise<ScopeStyle>} */
  static #style(page, selector) {
    return page.locator(selector).evaluate(element => {
      const computed = getComputedStyle(element)
      return { background: computed.backgroundColor, outline: computed.outlineWidth, outlineStyle: computed.outlineStyle, color: computed.color, font: computed.fontFamily, token: computed.getPropertyValue('--owner-token').trim(), scopeToken: computed.getPropertyValue('--_tac-owner').trim(), owner: element.closest('tachyon-component')?.getAttribute('data-tac-scope') ?? null }
    })
  }
}

await ComponentScopeBrowserGate.run()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} LogicEngine */
/** @typedef {{phase: string, sourceCommit: string, compilerSha256: string, engine: LogicEngine, browserVersion?: string, errors: string[], failedRequests: string[], externalRequests: string[]}} LogicDiagnostics */
/** @typedef {{background: string, color: string, font: string, owner: string | null}} LogicStyle */

class LogicControlsBrowserGate {
  static #timeout = 15_000
  static #svgNamespace = 'http://www.w3.org/2000/svg'
  static #htmlNamespace = 'http://www.w3.org/1999/xhtml'
  static #parentColor = 'rgb(12, 34, 56)'
  static #transparent = 'rgba(0, 0, 0, 0)'

  /** @returns {Promise<void>} */
  static async run() {
    const root = path.resolve(LogicControlsBrowserGate.#required('TAC_LOGIC_CONTROLS_FIXTURE_OUT'))
    const binary = path.resolve(LogicControlsBrowserGate.#required('TAC_BIN'))
    const sourceCommit = LogicControlsBrowserGate.#required('TAC_LOGIC_CONTROLS_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const engine = LogicControlsBrowserGate.#engine()
    const diagnostics = { phase: 'starting', sourceCommit, compilerSha256, engine, errors: [], failedRequests: [], externalRequests: [] }
    const receipt = path.join(root, `logic-controls-browser-${engine}-${randomUUID()}.json`)
    const started = performance.now()
    try {
      const proof = await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return LogicControlsBrowserGate.#probe(origin, diagnostics)
      })
      await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'pass', proof, elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      console.log(JSON.stringify({ status: 'pass', engine, receipt, proof }))
    } catch (error) {
      await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'fail', error: String(error), elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      throw error
    }
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} identifies this retained fixture`)
    return value
  }

  /** @returns {LogicEngine} */
  static #engine() {
    const engine = process.env.TAC_LOGIC_CONTROLS_BROWSER ?? 'chromium'
    assert.ok(engine === 'chromium' || engine === 'firefox' || engine === 'webkit')
    return engine
  }

  /** @param {LogicEngine} engine @returns {import('playwright').BrowserType} */
  static #browser(engine) {
    switch (engine) {
      case 'chromium': return chromium
      case 'firefox': return firefox
      case 'webkit': return webkit
    }
  }

  /** @param {string} origin @param {LogicDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(origin, diagnostics) {
    const browser = await LogicControlsBrowserGate.#browser(diagnostics.engine).launch({ timeout: LogicControlsBrowserGate.#timeout })
    diagnostics.browserVersion = browser.version()
    try {
      const page = await browser.newPage()
      LogicControlsBrowserGate.#observe(page, origin, diagnostics)
      await page.goto(origin, { waitUntil: 'networkidle', timeout: LogicControlsBrowserGate.#timeout })
      await page.locator('#first-branch').waitFor()
      diagnostics.phase = 'initial-nested-chain-and-loops'
      await LogicControlsBrowserGate.#branch(page, 'first-branch', 'Ready')
      await LogicControlsBrowserGate.#svgBranch(page, 'svg-first')
      assert.equal(await page.locator('#svg-nested-ready').count(), 1)
      await LogicControlsBrowserGate.#loops(page)
      const first = await LogicControlsBrowserGate.#styles(page, '#first-card')
      const second = await LogicControlsBrowserGate.#styles(page, '#second-card')
      diagnostics.phase = 'nested-else-if'
      await page.getByRole('button', { name: 'Nested waiting', exact: true }).click()
      await LogicControlsBrowserGate.#branch(page, 'first-branch', 'Waiting')
      assert.equal(await page.locator('#svg-nested-ready').count(), 0)
      assert.equal(await page.locator('#svg-nested-fallback').count(), 1)
      diagnostics.phase = 'nested-fallback'
      await page.getByRole('button', { name: 'Nested fallback', exact: true }).click()
      await LogicControlsBrowserGate.#branch(page, 'first-branch', 'Nested fallback')
      diagnostics.phase = 'outer-else-if'
      await page.getByRole('button', { name: 'Choose second', exact: true }).click()
      await LogicControlsBrowserGate.#branch(page, 'second-branch', 'Second')
      await LogicControlsBrowserGate.#svgBranch(page, 'svg-second')
      assert.equal(await page.locator('#svg-slot-fallback').count(), 1)
      diagnostics.phase = 'outer-fallback'
      await page.getByRole('button', { name: 'Choose fallback', exact: true }).click()
      await LogicControlsBrowserGate.#branch(page, 'fallback-branch', 'Fallback')
      await LogicControlsBrowserGate.#svgBranch(page, 'svg-fallback')
      await LogicControlsBrowserGate.#loops(page)
      diagnostics.phase = 'return-to-nested-first'
      await page.getByRole('button', { name: 'Nested ready', exact: true }).click()
      await page.getByRole('button', { name: 'Choose first', exact: true }).click()
      await LogicControlsBrowserGate.#branch(page, 'first-branch', 'Ready')
      await LogicControlsBrowserGate.#svgBranch(page, 'svg-first')
      assert.equal(await page.locator('#svg-slot-first').count(), 1)
      diagnostics.phase = 'conditional-disposal'
      const firstRoot = await page.locator('#first-card tachyon-component[data-tac-scope="logic-panel"]').elementHandle()
      assert.ok(firstRoot)
      assert.equal(await firstRoot.evaluate(element => element.isConnected), true)
      await page.getByRole('button', { name: 'Hide card', exact: true }).click()
      await page.locator('#card-hidden').waitFor()
      assert.equal(await page.locator('#first-card [data-probe="panel-owned"]').count(), 0)
      assert.equal(await page.locator('#first-card [data-probe="slot"]').count(), 0)
      assert.equal(await firstRoot.evaluate(element => element.isConnected), false)
      assert.deepEqual(await LogicControlsBrowserGate.#styles(page, '#second-card'), second)
      diagnostics.phase = 'conditional-remount'
      await page.getByRole('button', { name: 'Show card', exact: true }).click()
      await page.locator('#first-card [data-probe="panel-owned"]').waitFor()
      assert.equal(await page.locator('#card-hidden').count(), 0)
      assert.deepEqual(await LogicControlsBrowserGate.#styles(page, '#first-card'), first)
      assert.deepEqual(await LogicControlsBrowserGate.#styles(page, '#second-card'), second)
      assert.equal(await firstRoot.evaluate(element => element.isConnected), false)
      await LogicControlsBrowserGate.#loops(page)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      return { nestedThreeWayChain: true, outerThreeWayChain: true, triviaPreserved: true, listAndCountedBranches: true, conditionalRemounted: true, independentParents: true, projectedStylesAndInheritance: true, nestedSvgAndHtml: true, first, second }
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {LogicDiagnostics} diagnostics @returns {void} */
  static #observe(page, origin, diagnostics) {
    page.on('pageerror', error => diagnostics.errors.push(String(error)))
    page.on('requestfailed', request => diagnostics.failedRequests.push(new URL(request.url()).pathname))
    page.on('request', request => LogicControlsBrowserGate.#external(request.url(), origin, diagnostics))
  }

  /** @param {string} url @param {string} origin @param {LogicDiagnostics} diagnostics @returns {void} */
  static #external(url, origin, diagnostics) {
    if (url.startsWith('http') && new URL(url).origin !== origin) diagnostics.externalRequests.push(url)
  }

  /** @param {import('playwright').Page} page @param {string} id @param {string} text @returns {Promise<void>} */
  static async #branch(page, id, text) {
    await page.locator(`#${id}`).waitFor()
    await page.waitForFunction(({ id, text }) => document.getElementById(id)?.textContent?.trim() === text, { id, text })
    assert.deepEqual(await page.locator('#first-branch, #second-branch, #fallback-branch').evaluateAll(elements => elements.map(element => element.id)), [id])
    assert.equal(await page.locator(`#${id}`).textContent().then(value => value?.trim()), text)
    assert.equal(await page.locator('logic, loop, if, else, for').count(), 0)
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #loops(page) {
    assert.deepEqual(await page.locator('[data-probe="list"]').evaluateAll(elements => elements.map(element => ({ item: element.getAttribute('data-item'), text: element.textContent }))), [{ item: '0', text: 'First' }, { item: '1', text: 'Second' }, { item: '2', text: 'Fallback' }])
    assert.deepEqual(await page.locator('[data-probe="counted"]').evaluateAll(elements => elements.map(element => ({ item: element.getAttribute('data-item'), text: element.textContent }))), [{ item: '0', text: 'First' }, { item: '1', text: 'Second' }, { item: '2', text: 'Fallback' }])
    assert.equal(await page.locator('logic, loop, if, else, for').count(), 0)
  }

  /** @param {import('playwright').Page} page @param {string} id @returns {Promise<void>} */
  static async #svgBranch(page, id) {
    assert.deepEqual(await page.locator('#svg-first, #svg-second, #svg-fallback').evaluateAll(elements => elements.map(element => element.id)), [id])
    assert.deepEqual(await page.locator('#canonical-svg').evaluate(element => ({ namespace: element.namespaceURI, name: element.constructor.name, width: element.getBoundingClientRect().width, height: element.getBoundingClientRect().height, viewBox: element.getAttribute('viewBox') })), { namespace: LogicControlsBrowserGate.#svgNamespace, name: 'SVGSVGElement', width: 24, height: 24, viewBox: '0 0 24 24' })
    assert.equal(await page.locator('#canonical-svg g, #canonical-svg path, #canonical-svg circle, #canonical-svg rect, #canonical-foreign, #foreign-svg').evaluateAll(elements => elements.some(element => element.namespaceURI !== 'http://www.w3.org/2000/svg')), false)
    assert.deepEqual(await page.locator('[data-probe="svg-loop"]').evaluateAll(elements => elements.map(element => element.namespaceURI)), Array(3).fill(LogicControlsBrowserGate.#svgNamespace))
    assert.equal(await page.locator('#foreign-html').evaluate(element => element.namespaceURI), LogicControlsBrowserGate.#htmlNamespace)
    assert.equal(await page.locator('#foreign-svg-path').evaluate(element => element.namespaceURI), LogicControlsBrowserGate.#svgNamespace)
    assert.deepEqual(await page.locator('#canonical-slot-svg circle, #canonical-slot-svg rect').evaluateAll(elements => elements.map(element => element.namespaceURI)), [LogicControlsBrowserGate.#svgNamespace])
  }

  /** @param {import('playwright').Page} page @param {string} parent @returns {Promise<Record<string, LogicStyle>>} */
  static async #styles(page, parent) {
    const owned = await LogicControlsBrowserGate.#style(page, `${parent} [data-probe="panel-owned"]`)
    const plain = await LogicControlsBrowserGate.#style(page, `${parent} [data-probe="plain"]`)
    const slot = await LogicControlsBrowserGate.#style(page, `${parent} [data-probe="slot"]`)
    const neighbor = await LogicControlsBrowserGate.#style(page, '#neighbor [data-probe="plain"]')
    const harness = await LogicControlsBrowserGate.#style(page, '#harness-owned')
    assert.equal(owned.background, 'rgb(180, 20, 20)')
    assert.equal(owned.owner, 'logic-panel')
    assert.equal(plain.background, LogicControlsBrowserGate.#transparent)
    assert.equal(plain.owner, 'logic-plain')
    assert.equal(slot.background, 'rgb(20, 140, 20)')
    assert.equal(slot.owner, 'logic-slot')
    assert.equal(plain.color, LogicControlsBrowserGate.#parentColor)
    assert.equal(slot.color, LogicControlsBrowserGate.#parentColor)
    assert.ok(plain.font.includes('monospace'))
    assert.ok(slot.font.includes('monospace'))
    assert.equal(neighbor.background, LogicControlsBrowserGate.#transparent)
    assert.equal(harness.background, 'rgb(20, 20, 180)')
    return { owned, plain, slot, neighbor, harness }
  }

  /** @param {import('playwright').Page} page @param {string} selector @returns {Promise<LogicStyle>} */
  static #style(page, selector) {
    return page.locator(selector).evaluate(element => {
      const computed = getComputedStyle(element)
      return { background: computed.backgroundColor, color: computed.color, font: computed.fontFamily, owner: element.closest('tachyon-component')?.getAttribute('data-tac-scope') ?? null }
    })
  }
}

await LogicControlsBrowserGate.run()

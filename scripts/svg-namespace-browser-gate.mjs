#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} SvgEngine */
/** @typedef {{phase: string, sourceCommit: string, compilerSha256: string, engine: SvgEngine, browserVersion?: string, errors: string[], failedRequests: string[], externalRequests: string[]}} SvgDiagnostics */

class SvgNamespaceBrowserGate {
  static #timeout = 15_000
  static #svgNamespace = 'http://www.w3.org/2000/svg'
  static #htmlNamespace = 'http://www.w3.org/1999/xhtml'
  static #standaloneHost = `<!doctype html><html lang="en"><head><title>Standalone SVG namespace regression</title><script type="module" src="/.tachyon/elements/svg-namespace.js"></script></head><body><svg-namespace></svg-namespace></body></html>`

  /** @returns {Promise<void>} */
  static async run() {
    const root = path.resolve(SvgNamespaceBrowserGate.#required('TAC_SVG_NAMESPACE_FIXTURE_OUT'))
    const binary = path.resolve(SvgNamespaceBrowserGate.#required('TAC_BIN'))
    const sourceCommit = SvgNamespaceBrowserGate.#required('TAC_SVG_NAMESPACE_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const engine = SvgNamespaceBrowserGate.#engine()
    const diagnostics = { phase: 'starting', sourceCommit, compilerSha256, engine, errors: [], failedRequests: [], externalRequests: [] }
    const receipt = path.join(root, `svg-namespace-browser-${engine}-${randomUUID()}.json`)
    const started = performance.now()
    try {
      const proof = await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return SvgNamespaceBrowserGate.#probe(origin, diagnostics)
      })
      await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'pass', proof, elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false, standaloneHostFixture: true }, null, 2) + '\n')
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

  /** @returns {SvgEngine} */
  static #engine() {
    const engine = process.env.TAC_SVG_NAMESPACE_BROWSER ?? 'chromium'
    assert.ok(engine === 'chromium' || engine === 'firefox' || engine === 'webkit')
    return engine
  }

  /** @param {SvgEngine} engine @returns {import('playwright').BrowserType} */
  static #browser(engine) {
    switch (engine) {
      case 'chromium': return chromium
      case 'firefox': return firefox
      case 'webkit': return webkit
    }
  }

  /** @param {string} origin @param {SvgDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(origin, diagnostics) {
    const browser = await SvgNamespaceBrowserGate.#browser(diagnostics.engine).launch({ timeout: SvgNamespaceBrowserGate.#timeout })
    diagnostics.browserVersion = browser.version()
    try {
      const page = await browser.newPage()
      SvgNamespaceBrowserGate.#observe(page, origin, diagnostics)
      await page.goto(origin, { waitUntil: 'networkidle', timeout: SvgNamespaceBrowserGate.#timeout })
      await page.locator('#graphic').waitFor()
      diagnostics.phase = 'compiled-page-svg'
      const initial = await SvgNamespaceBrowserGate.#check(page, '0 0 24 24')
      diagnostics.phase = 'conditional-remount'
      await page.getByRole('button', { name: 'Hide path', exact: true }).click()
      await page.locator('#conditional-path').waitFor({ state: 'detached' })
      await page.getByRole('button', { name: 'Show path', exact: true }).click()
      await page.locator('#conditional-path').waitFor()
      await SvgNamespaceBrowserGate.#check(page, '0 0 24 24')
      diagnostics.phase = 'focused-html-update'
      await page.locator('#draft').fill('Typed draft')
      await page.waitForFunction(() => document.querySelector('#graphic')?.getAttribute('viewBox') === '0 0 32 32')
      assert.equal(await page.locator('#draft').inputValue(), 'Typed draft')
      assert.equal(await page.locator('#draft').evaluate(element => element === document.activeElement), true)
      await SvgNamespaceBrowserGate.#check(page, '0 0 32 32')
      assert.deepEqual(await SvgNamespaceBrowserGate.#reference(page), { href: '#gradient', language: 'fr', space: 'preserve' })
      diagnostics.phase = 'focused-svg-attribute-removal'
      await page.locator('#reference').evaluate(element => element.focus())
      assert.equal(await page.locator('#reference').evaluate(element => element === document.activeElement), true)
      await page.locator('#remove-reference').evaluate(element => element.click())
      await page.waitForFunction(() => document.querySelector('#reference')?.getAttributeNS('http://www.w3.org/1999/xlink', 'href') === null)
      assert.equal(await page.locator('#reference').evaluate(element => element === document.activeElement), true)
      assert.deepEqual(await SvgNamespaceBrowserGate.#reference(page), { href: null, language: 'fr', space: 'preserve' })
      diagnostics.phase = 'ordinary-state-update'
      await page.getByRole('button', { name: 'Change view box', exact: true }).click()
      await page.waitForFunction(() => document.querySelector('#graphic')?.getAttribute('viewBox') === '0 0 48 48')
      await SvgNamespaceBrowserGate.#check(page, '0 0 48 48')
      diagnostics.phase = 'exported-standalone-component'
      await page.route(`${origin}/svg-standalone-test`, route => route.fulfill({ contentType: 'text/html', body: SvgNamespaceBrowserGate.#standaloneHost }))
      await page.goto(`${origin}/svg-standalone-test`, { waitUntil: 'networkidle', timeout: SvgNamespaceBrowserGate.#timeout })
      await page.locator('svg-namespace #graphic').waitFor()
      const standalone = await SvgNamespaceBrowserGate.#check(page, '0 0 24 24')
      await page.getByRole('button', { name: 'Change view box', exact: true }).click()
      await page.waitForFunction(() => document.querySelector('svg-namespace')?.shadowRoot?.querySelector('#graphic')?.getAttribute('viewBox') === '0 0 48 48')
      await SvgNamespaceBrowserGate.#check(page, '0 0 48 48')
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      return { initial, standalone, conditionalRemounted: true, htmlFocusPreserved: true, svgFocusPreserved: true, namespacedAttributeRemoved: true, exportedStateUpdated: true }
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {SvgDiagnostics} diagnostics @returns {void} */
  static #observe(page, origin, diagnostics) {
    page.on('pageerror', error => diagnostics.errors.push(String(error)))
    page.on('requestfailed', request => diagnostics.failedRequests.push(new URL(request.url()).pathname))
    page.on('request', request => SvgNamespaceBrowserGate.#external(request.url(), origin, diagnostics))
  }

  /** @param {string} url @param {string} origin @param {SvgDiagnostics} diagnostics @returns {void} */
  static #external(url, origin, diagnostics) {
    if (url.startsWith('http') && new URL(url).origin !== origin) diagnostics.externalRequests.push(url)
  }

  /** @param {import('playwright').Page} page @param {string} viewBox @returns {Promise<Record<string, unknown>>} */
  static async #check(page, viewBox) {
    const svg = await page.locator('#graphic').evaluate(element => ({ namespace: element.namespaceURI, constructor: element.constructor.name, width: element.getBoundingClientRect().width, height: element.getBoundingClientRect().height, viewBox: element.getAttribute('viewBox'), lowercaseViewBox: element.hasAttribute('viewbox'), aspectRatio: element.getAttribute('preserveAspectRatio'), xmlns: element.getAttributeNS('http://www.w3.org/2000/xmlns/', 'xmlns'), xlinkNamespace: element.getAttributeNS('http://www.w3.org/2000/xmlns/', 'xlink') }))
    assert.equal(svg.namespace, SvgNamespaceBrowserGate.#svgNamespace)
    assert.equal(svg.constructor, 'SVGSVGElement')
    assert.equal(svg.width, 16)
    assert.equal(svg.height, 16)
    assert.equal(svg.viewBox, viewBox)
    assert.equal(svg.lowercaseViewBox, false)
    assert.equal(svg.aspectRatio, 'xMidYMid meet')
    assert.equal(svg.xmlns, SvgNamespaceBrowserGate.#svgNamespace)
    assert.equal(svg.xlinkNamespace, 'http://www.w3.org/1999/xlink')
    assert.deepEqual(await page.locator('#moon, #gradient, #reference, #conditional-path, #switched, #foreign').evaluateAll(elements => elements.map(element => element.namespaceURI)), Array(6).fill(SvgNamespaceBrowserGate.#svgNamespace))
    assert.deepEqual(await page.locator('#gradient').evaluate(element => ({ name: element.localName, units: element.getAttribute('gradientUnits'), transform: element.getAttribute('gradientTransform') })), { name: 'linearGradient', units: 'userSpaceOnUse', transform: 'rotate(20)' })
    assert.deepEqual(await page.locator('[data-probe="listed"]').evaluateAll(elements => elements.map(element => element.namespaceURI)), Array(2).fill(SvgNamespaceBrowserGate.#svgNamespace))
    assert.deepEqual(await page.locator('[data-probe="counted"]').evaluateAll(elements => elements.map(element => element.namespaceURI)), Array(2).fill(SvgNamespaceBrowserGate.#svgNamespace))
    assert.equal(await page.locator('#foreign').evaluate(element => element.localName), 'foreignObject')
    assert.deepEqual(await page.locator('#html-child, #draft').evaluateAll(elements => elements.map(element => element.namespaceURI)), Array(2).fill(SvgNamespaceBrowserGate.#htmlNamespace))
    assert.deepEqual(await page.locator('#nested-svg, #nested-svg circle, #slot-graphic, #slotted-circle').evaluateAll(elements => elements.map(element => element.namespaceURI)), Array(4).fill(SvgNamespaceBrowserGate.#svgNamespace))
    return svg
  }

  /** @param {import('playwright').Page} page @returns {Promise<{href: string | null, language: string | null, space: string | null}>} */
  static #reference(page) {
    return page.locator('#reference').evaluate(element => ({ href: element.getAttributeNS('http://www.w3.org/1999/xlink', 'href'), language: element.getAttributeNS('http://www.w3.org/XML/1998/namespace', 'lang'), space: element.getAttributeNS('http://www.w3.org/XML/1998/namespace', 'space') }))
  }
}

await SvgNamespaceBrowserGate.run()

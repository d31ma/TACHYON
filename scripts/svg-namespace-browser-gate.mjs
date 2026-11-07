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

/** @typedef {'chromium' | 'firefox' | 'webkit'} SvgEngine */
/** @typedef {{phase: string, sourceCommit: string, compilerSha256: string, engine: SvgEngine, browserVersion?: string, errors: string[], failedRequests: string[], externalRequests: string[]}} SvgDiagnostics */
/** @typedef {{root: string, retained: boolean, sourceCommit: string, compilerSha256: string}} SvgRun */

class SvgNamespaceBrowserGate {
  static #timeout = 15_000
  static #engines = ['chromium', 'firefox', 'webkit']
  static #repository = fileURLToPath(new URL('..', import.meta.url))
  static #fixture = 'crates/tachyon-core/tests/fixtures/tac-svg-namespace'
  static #svgNamespace = 'http://www.w3.org/2000/svg'
  static #htmlNamespace = 'http://www.w3.org/1999/xhtml'
  static #standaloneHost = `<!doctype html><html lang="en"><head><title>Standalone SVG namespace regression</title><script type="module" src="/.tachyon/elements/svg-namespace.js"></script></head><body><svg-namespace></svg-namespace></body></html>`

  /** @returns {Promise<void>} */
  static async run() {
    const binary = SvgNamespaceBrowserGate.#binary()
    const engines = SvgNamespaceBrowserGate.#selectedEngines()
    const retainedRoot = process.env.TAC_SVG_NAMESPACE_FIXTURE_OUT
    const root = retainedRoot ? path.resolve(retainedRoot) : await mkdtemp(path.join(tmpdir(), 'tac-svg-namespace-'))
    try {
      await cp(path.join(SvgNamespaceBrowserGate.#repository, SvgNamespaceBrowserGate.#fixture), root, { recursive: true })
      await promisify(execFile)(binary, ['bundle', root])
      /** @type {SvgRun} */
      const run = {
        root,
        retained: Boolean(retainedRoot),
        sourceCommit: process.env.TAC_SVG_NAMESPACE_SOURCE_COMMIT ?? await SvgNamespaceBrowserGate.#headCommit(),
        compilerSha256: createHash('sha256').update(await readFile(binary)).digest('hex'),
      }
      await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        for (const engine of engines) await SvgNamespaceBrowserGate.#runEngine(engine, origin, run)
      })
    } finally {
      if (!retainedRoot) await rm(root, { recursive: true, force: true })
    }
  }

  /** @param {SvgEngine} engine @param {string} origin @param {SvgRun} run @returns {Promise<void>} */
  static async #runEngine(engine, origin, run) {
    /** @type {SvgDiagnostics} */
    const diagnostics = { phase: 'starting', sourceCommit: run.sourceCommit, compilerSha256: run.compilerSha256, engine, errors: [], failedRequests: [], externalRequests: [] }
    const receipt = path.join(run.root, `svg-namespace-browser-${engine}-${randomUUID()}.json`)
    const started = performance.now()
    try {
      const proof = await SvgNamespaceBrowserGate.#probe(origin, diagnostics)
      if (run.retained) await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'pass', proof, elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false, standaloneHostFixture: true }, null, 2) + '\n')
      console.log(JSON.stringify({ status: 'pass', engine, browserVersion: diagnostics.browserVersion, receipt: run.retained ? receipt : null, proof }))
    } catch (error) {
      if (run.retained) await writeFile(receipt, JSON.stringify({ ...diagnostics, status: 'fail', error: String(error), elapsedMilliseconds: performance.now() - started, runtimeReplaced: false, stylesInjected: false }, null, 2) + '\n')
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    }
  }

  /** @returns {Promise<string>} */
  static async #headCommit() {
    const { stdout } = await promisify(execFile)('git', ['rev-parse', 'HEAD'], { cwd: SvgNamespaceBrowserGate.#repository })
    return stdout.trim()
  }

  /** @returns {string} */
  static #binary() {
    const binary = process.env.TAC_BIN
    assert.ok(binary, 'TAC_BIN names the ty binary under test')
    return path.resolve(binary)
  }

  /** @returns {SvgEngine[]} */
  static #selectedEngines() {
    const requested = process.env.TAC_SVG_NAMESPACE_BROWSERS
    if (!requested) return /** @type {SvgEngine[]} */ (SvgNamespaceBrowserGate.#engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(SvgNamespaceBrowserGate.#engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {SvgEngine[]} */ (engines)
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

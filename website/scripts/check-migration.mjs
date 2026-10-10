#!/usr/bin/env node
// @ts-check
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, readdir, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { withServer } from '../../scripts/release/server-probe.mjs'

/** @typedef {{time: number, theme: string | null, background: string}} ThemeFrame */
/** @typedef {{frames: ThemeFrame[], changes: {time: number, theme: string | null}[], paints: {name: string, time: number}[]}} ThemeTrace */
/** @typedef {{service(name: string): {theme: string, themeHelper?: {restore(): string, constructor: {name: string}}} | undefined}} WebsiteRuntime */
/** @typedef {{__tachyonTac?: WebsiteRuntime, __websiteAcceptanceTrace?: ThemeTrace, WTheme?: {get(): string}}} WebsiteGlobals */
/** @typedef {{package: string, version: string, license: string, sha256: string}} VendorMetadata */
/** @typedef {{tac: {budgets: {script: number, check?: boolean}, scripts: string[], styles: string[], scriptSources: Record<string, VendorMetadata>, styleSources: Record<string, {kind: string} & Partial<VendorMetadata>>}}} WebsiteManifest */
/** @typedef {{sourceCommit: string, compilerSha256: string, clientSha256: string, output: string, browserVersion?: string, errors: string[], failedRequests: string[], externalRequests: string[], proof: Record<string, unknown>}} AcceptanceReceipt */

class WebsiteMigrationAcceptance {
  static #timeout = 45_000
  static #scriptBudget = 320
  static #website = fileURLToPath(new URL('..', import.meta.url))

  /** @returns {Promise<void>} */
  static async run() {
    const binary = path.resolve(WebsiteMigrationAcceptance.#required('TAC_BIN'))
    const output = path.resolve(WebsiteMigrationAcceptance.#required('TAC_WEBSITE_OUTPUT'))
    const receiptPath = path.resolve(WebsiteMigrationAcceptance.#required('TAC_WEBSITE_ACCEPTANCE_RECEIPT'))
    assert.ok(!receiptPath.startsWith(WebsiteMigrationAcceptance.#website + path.sep), 'Retain acceptance receipts outside synced website source')
    const sourceCommit = WebsiteMigrationAcceptance.#required('TAC_WEBSITE_SOURCE_COMMIT')
    assert.match(sourceCommit, /^[0-9a-f]{40}$/)
    const manifest = /** @type {WebsiteManifest} */ (JSON.parse(await readFile(path.join(WebsiteMigrationAcceptance.#website, 'client/manifest.json'), 'utf8')))
    assert.equal(manifest.tac.budgets.script, WebsiteMigrationAcceptance.#scriptBudget)
    assert.notEqual(manifest.tac.budgets.check, false)
    const clientSha256 = await WebsiteMigrationAcceptance.#clientFingerprint()
    const compilerSha256 = WebsiteMigrationAcceptance.#sha(await readFile(binary))
    /** @type {AcceptanceReceipt} */
    const receipt = { sourceCommit, compilerSha256, clientSha256, output, errors: [], failedRequests: [], externalRequests: [], proof: {} }
    const started = performance.now()
    try {
      receipt.proof.componentStyles = await WebsiteMigrationAcceptance.#styles(output)
      await withServer(binary, ['preview', WebsiteMigrationAcceptance.#website, '--static', '--out-dir', output, '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        receipt.proof.vendors = await WebsiteMigrationAcceptance.#vendors(request, manifest)
        await WebsiteMigrationAcceptance.#browser(origin, receipt)
      })
      assert.equal(await WebsiteMigrationAcceptance.#clientFingerprint(), clientSha256, 'Website source remained frozen throughout qualification')
      assert.deepEqual(receipt.errors, [])
      assert.deepEqual(receipt.failedRequests, [])
      assert.deepEqual(receipt.externalRequests, [])
      await WebsiteMigrationAcceptance.#receipt(receiptPath, receipt, started, 'pass')
      console.log(JSON.stringify({ status: 'pass', receipt: receiptPath, sourceCommit, compilerSha256, clientSha256 }))
    } catch (error) {
      await WebsiteMigrationAcceptance.#receipt(receiptPath, receipt, started, 'fail', String(error))
      throw error
    }
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} must identify the explicit retained website qualification input`)
    return value
  }

  /** @param {Uint8Array | string} bytes @returns {string} */
  static #sha(bytes) {
    return createHash('sha256').update(bytes).digest('hex')
  }

  /** @param {string} directory @returns {Promise<string[]>} */
  static async #files(directory) {
    const files = []
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const child = path.join(directory, entry.name)
      assert.ok(!entry.isSymbolicLink(), `Qualification source must not redirect through a symlink: ${child}`)
      if (entry.isDirectory()) files.push(...await WebsiteMigrationAcceptance.#files(child))
      else files.push(child)
    }
    return files.sort()
  }

  /** @returns {Promise<string>} */
  static async #clientFingerprint() {
    const root = path.join(WebsiteMigrationAcceptance.#website, 'client')
    const hash = createHash('sha256')
    for (const file of await WebsiteMigrationAcceptance.#files(root)) {
      hash.update(path.relative(root, file)).update('\0')
      hash.update(WebsiteMigrationAcceptance.#sha(await readFile(file))).update('\n')
    }
    return hash.digest('hex')
  }

  /** @param {string} output @returns {Promise<Record<string, unknown>>} */
  static async #styles(output) {
    const root = path.join(WebsiteMigrationAcceptance.#website, 'client/components')
    const templates = (await WebsiteMigrationAcceptance.#files(root)).filter(file => path.basename(file) === 'tac.html')
    const css = await readFile(path.join(output, '.tachyon/components.css'), 'utf8')
    const owners = templates.map(file => path.relative(root, path.dirname(file)).split(path.sep).join('-'))
    for (const owner of owners) {
      assert.ok(css.includes(`@scope (tachyon-component[data-tac-scope="${owner}"]) to (tachyon-component[data-tac-scope])`), `${owner} has an emitted component boundary`)
    }
    assert.equal((css.match(/@scope \(tachyon-component\[data-tac-scope=/g) ?? []).length, owners.length)
    return { owners, sha256: WebsiteMigrationAcceptance.#sha(css), count: owners.length }
  }

  /** @param {import('../../scripts/release/server-probe.mjs').ProbeRequest} request @param {WebsiteManifest} manifest @returns {Promise<Record<string, unknown>>} */
  static async #vendors(request, manifest) {
    const declared = [...manifest.tac.scripts, ...Object.keys(manifest.tac.styleSources).filter(resource => manifest.tac.styleSources[resource].kind === 'vendor')]
    const assets = []
    for (const resource of declared) {
      const metadata = manifest.tac.scriptSources[resource] ?? manifest.tac.styleSources[resource]
      const response = await request(resource)
      assert.equal(response.status, 200, resource)
      const bytes = new Uint8Array(await response.arrayBuffer())
      const sha256 = WebsiteMigrationAcceptance.#sha(bytes)
      assert.equal(sha256, metadata.sha256, `${resource} serves the exact declared vendor bytes`)
      assets.push({ resource, sha256, decodedBytes: bytes.length, package: metadata.package, version: metadata.version, license: metadata.license })
    }
    return { assets, declaredScripts: manifest.tac.scripts, declaredStyles: manifest.tac.styles }
  }

  /** @param {string} origin @param {AcceptanceReceipt} receipt @returns {Promise<void>} */
  static async #browser(origin, receipt) {
    const dependencies = path.resolve(WebsiteMigrationAcceptance.#required('TAC_BROWSER_DEPENDENCIES'))
    const load = createRequire(import.meta.url)
    const { chromium } = /** @type {typeof import('playwright')} */ (load(path.join(dependencies, 'playwright')))
    const browser = await chromium.launch({ timeout: WebsiteMigrationAcceptance.#timeout })
    receipt.browserVersion = browser.version()
    try {
      const themes = []
      for (const saved of [null, 'dark', 'light', 'auto']) {
        const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: 'dark' })
        try {
          await WebsiteMigrationAcceptance.#trace(context, saved)
          const page = await context.newPage()
          WebsiteMigrationAcceptance.#observe(page, origin, receipt)
          await WebsiteMigrationAcceptance.#open(page, origin, '/')
          themes.push(await WebsiteMigrationAcceptance.#theme(page, saved ?? 'dark'))
          if (saved === null) receipt.proof.routes = await WebsiteMigrationAcceptance.#routes(page, origin)
          if (saved === 'light') receipt.proof.toggle = await WebsiteMigrationAcceptance.#toggle(page)
        } finally {
          await context.close()
        }
      }
      receipt.proof.themes = themes
      receipt.proof.platformNavigation = await WebsiteMigrationAcceptance.#platform(browser, origin, receipt)
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').BrowserContext} context @param {string | null} saved @returns {Promise<void>} */
  static async #trace(context, saved) {
    await context.addInitScript(value => {
      if (value === null) localStorage.removeItem('w-theme')
      else localStorage.setItem('w-theme', value)
      const globals = /** @type {typeof globalThis & WebsiteGlobals} */ (globalThis)
      /** @type {ThemeTrace} */
      const trace = { frames: [], changes: [], paints: [] }
      globals.__websiteAcceptanceTrace = trace
      const watch = new MutationObserver(() => {
        trace.changes.push({ time: performance.now(), theme: document.documentElement?.getAttribute('w-theme') ?? null })
      })
      watch.observe(document, { subtree: true, attributes: true, attributeFilter: ['w-theme'] })
      new PerformanceObserver(list => {
        for (const entry of list.getEntries()) trace.paints.push({ name: entry.name, time: entry.startTime })
      }).observe({ type: 'paint', buffered: true })
      const sample = (/** @type {number} */ time) => {
        trace.frames.push({ time, theme: document.documentElement?.getAttribute('w-theme') ?? null, background: document.body ? getComputedStyle(document.body).backgroundColor : '' })
        if (trace.frames.length < 180) requestAnimationFrame(sample)
        else watch.disconnect()
      }
      requestAnimationFrame(sample)
    }, saved)
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {AcceptanceReceipt} receipt @returns {void} */
  static #observe(page, origin, receipt) {
    page.setDefaultTimeout(WebsiteMigrationAcceptance.#timeout)
    page.on('pageerror', error => receipt.errors.push(String(error)))
    page.on('console', message => {
      if (message.type() === 'error') receipt.errors.push(`console: ${message.text()}`)
    })
    page.on('requestfailed', request => receipt.failedRequests.push(`${new URL(request.url()).pathname}: ${request.failure()?.errorText}`))
    page.on('response', response => {
      if (response.status() >= 400) receipt.errors.push(`${response.status()} ${new URL(response.url()).pathname}`)
    })
    page.on('request', request => {
      if (request.url().startsWith('http') && new URL(request.url()).origin !== origin) receipt.externalRequests.push(request.url())
    })
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {string} route @returns {Promise<void>} */
  static async #open(page, origin, route) {
    const response = await page.goto(origin + route, { waitUntil: 'networkidle', timeout: WebsiteMigrationAcceptance.#timeout })
    assert.equal(response?.status(), 200, route)
    await page.waitForFunction(() => {
      const runtime = /** @type {typeof globalThis & WebsiteGlobals} */ (globalThis).__tachyonTac
      return runtime?.service('search')?.themeHelper && document.querySelector('#main')?.getAttribute('data-theme')
    })
  }

  /** @param {import('playwright').Page} page @param {string} expected @returns {Promise<Record<string, unknown>>} */
  static async #theme(page, expected) {
    const proof = await page.evaluate(WebsiteMigrationAcceptance.themeState)
    assert.equal(proof.helperClass, 'ThemeHelper', 'The generated SearchService owns the real injected Helper')
    assert.equal(proof.helperRestore, 'function')
    for (const value of [proof.root, proof.saved, proof.main, proof.service, proof.vendorTheme]) assert.equal(value, expected)
    assert.ok(proof.customElements.every(element => element.defined), 'Declared DuVay custom elements are registered')
    assert.deepEqual(proof.scripts, [{ src: '/shared/assets/duvay/duvay-wc.min.js', type: 'module' }, { src: '/shared/assets/duvay/duvay.min.js', type: 'module' }])
    assert.ok(proof.trace, 'Observational first-paint trace exists')
    WebsiteMigrationAcceptance.#paint(proof.trace, expected)
    return { expected, ...proof }
  }

  static themeState() {
    const globals = /** @type {typeof globalThis & WebsiteGlobals} */ (globalThis)
    const search = globals.__tachyonTac?.service('search')
    const elements = ['w-btn', 'w-container', 'w-app-bar', 'w-expansion-panel']
    const scripts = [...document.querySelectorAll('script[src*="/shared/assets/duvay/"]')]
    return {
      root: document.documentElement.getAttribute('w-theme'),
      saved: localStorage.getItem('w-theme'),
      main: document.querySelector('#main')?.getAttribute('data-theme'),
      service: search?.theme,
      helperClass: search?.themeHelper?.constructor.name,
      helperRestore: typeof search?.themeHelper?.restore,
      vendorTheme: globals.WTheme?.get(),
      customElements: elements.map(name => ({ name, defined: Boolean(customElements.get(name)) })),
      scripts: scripts.map(script => ({
        src: new URL(/** @type {HTMLScriptElement} */ (script).src).pathname,
        type: script.getAttribute('type'),
      })),
      trace: globals.__websiteAcceptanceTrace,
    }
  }

  /** @param {ThemeTrace} trace @param {string} expected @returns {void} */
  static #paint(trace, expected) {
    for (const name of ['first-paint', 'first-contentful-paint']) {
      const paint = trace.paints.find(entry => entry.name === name)
      assert.ok(paint, `${name} was observed`)
      const change = trace.changes.filter(entry => entry.time <= paint.time).at(-1)
      const frame = trace.frames.filter(entry => entry.time <= paint.time).at(-1)
      assert.equal(change?.theme ?? frame?.theme, expected, `${expected} root preference was applied before ${name}`)
    }
  }

  /** @param {import('playwright').Page} page @param {string} origin @returns {Promise<Record<string, unknown>>} */
  static async #routes(page, origin) {
    const routes = ['/', '/docs/', '/docs/pages/', '/docs/features/', '/docs/features/companion-publish/']
    const activated = []
    for (const route of routes) {
      if (route !== '/') await WebsiteMigrationAcceptance.#open(page, origin, route)
      const helper = await page.evaluate(() => /** @type {typeof globalThis & WebsiteGlobals} */ (globalThis).__tachyonTac?.service('search')?.themeHelper?.constructor.name)
      assert.equal(helper, 'ThemeHelper', `${route} activates the actual owner without opening search`)
      activated.push({ route, helper })
    }
    return { activated }
  }

  /** @param {import('playwright').Page} page @returns {Promise<Record<string, unknown>>} */
  static async #toggle(page) {
    await page.getByRole('button', { name: 'Switch between the light and dark theme', exact: true }).click()
    await page.waitForFunction(() => document.documentElement.getAttribute('w-theme') === 'dark' && localStorage.getItem('w-theme') === 'dark')
    return { initial: 'light', toggled: 'dark', persisted: await page.evaluate(() => localStorage.getItem('w-theme')) }
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {AcceptanceReceipt} receipt @returns {Promise<Record<string, unknown>>} */
  static async #platform(browser, origin, receipt) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } })
    try {
      const page = await context.newPage()
      WebsiteMigrationAcceptance.#observe(page, origin, receipt)
      await WebsiteMigrationAcceptance.#open(page, origin, '/')
      const tabs = page.locator('#languages [role="tablist"][aria-label="Platform"]').first()
      await tabs.getByRole('tab', { name: 'Web', exact: true }).click()
      await page.waitForFunction(() => localStorage.getItem('tachyon.platform') === 'web')
      await tabs.getByRole('tab', { name: 'macOS', exact: true }).click()
      await page.waitForFunction(() => localStorage.getItem('tachyon.platform') === 'macos')
      assert.equal(await tabs.getByRole('tab', { name: 'macOS', exact: true }).getAttribute('aria-selected'), 'true')
      const desktop = await WebsiteMigrationAcceptance.#layout(page, false)
      await page.reload({ waitUntil: 'networkidle', timeout: WebsiteMigrationAcceptance.#timeout })
      assert.equal(await tabs.getByRole('tab', { name: 'macOS', exact: true }).getAttribute('aria-selected'), 'true', 'Real platform choice survives a raw reload')
      await page.getByRole('navigation', { name: 'Primary', exact: true }).getByRole('link', { name: 'Docs', exact: true }).click()
      await page.waitForURL(/\/docs\/?$/)
      const pages = page.locator('.docs-shell__nav a[href="/docs/pages"]').first()
      if (!await pages.isVisible()) await pages.locator('xpath=ancestor::details').locator('summary').click()
      await pages.click()
      await page.waitForURL(/\/docs\/pages\/?$/)
      assert.equal(await page.evaluate(() => localStorage.getItem('tachyon.platform')), 'macos')
      const mobile = await browser.newContext({ viewport: { width: 390, height: 900 }, hasTouch: true, isMobile: true })
      try {
        const small = await mobile.newPage()
        WebsiteMigrationAcceptance.#observe(small, origin, receipt)
        await WebsiteMigrationAcceptance.#open(small, origin, '/')
        return { savedPlatform: 'macos', reloaded: true, headerAndSidebarNavigation: true, desktop, mobile: await WebsiteMigrationAcceptance.#layout(small, true) }
      } finally {
        await mobile.close()
      }
    } finally {
      await context.close()
    }
  }

  /** @param {import('playwright').Page} page @param {boolean} coarse @returns {Promise<Record<string, unknown>>} */
  static async #layout(page, coarse) {
    const report = await page.evaluate(() => {
      const style = (/** @type {string} */ selector) => {
        const element = document.querySelector(selector)
        if (!element) throw new Error(`Missing rendered ownership consumer: ${selector}`)
        const computed = getComputedStyle(element)
        return { display: computed.display, position: computed.position, scope: element.closest('tachyon-component')?.getAttribute('data-tac-scope'), token: computed.getPropertyValue('--_tac-owner').trim() }
      }
      return { viewport: document.documentElement.clientWidth, scroll: document.documentElement.scrollWidth, header: style('w-app-bar'), quickstart: style('[aria-labelledby="quickstart-heading"] .grid-auto'), footer: style('footer .grid-auto'), search: style('.site-search__trigger') }
    })
    assert.ok(report.scroll <= report.viewport + 1, 'Owned component CSS preserves horizontal fit')
    assert.equal(report.header.position, 'sticky')
    assert.equal(report.quickstart.display, 'grid')
    assert.equal(report.footer.display, 'grid')
    assert.equal(report.search.display, coarse ? 'none' : 'inline-flex')
    /** @type {['header' | 'quickstart' | 'footer' | 'search', string][]} */
    const owners = [['header', 'site-header'], ['quickstart', 'home-quickstart'], ['footer', 'site-footer'], ['search', 'site-search']]
    for (const [name, owner] of owners) {
      const style = report[name]
      assert.equal(style.scope, owner)
      assert.equal(style.token, owner)
    }
    return report
  }

  /** @param {string} destination @param {AcceptanceReceipt} receipt @param {number} started @param {'pass' | 'fail'} status @param {string} [error] @returns {Promise<void>} */
  static async #receipt(destination, receipt, started, status, error) {
    const result = {
      ...receipt,
      status,
      error,
      scope: 'Supplemental retained website browser gate',
      elapsedMilliseconds: performance.now() - started,
      runtimeReplaced: false,
      stylesInjected: false,
      preferenceFixtureOnly: true,
      paintObservation: 'requestAnimationFrame snapshots and root-attribute mutation chronology compared with browser paint timestamps; no pixel proof',
      scriptBudgetKiB: WebsiteMigrationAcceptance.#scriptBudget,
      releaseBudgetCheckedByThisGate: false,
      budgetAcceptance: 'Require the ordinary release bundle to pass BudgetCheck independently: emitted Brotli bytes, or raw bytes when no compressed copy exists. Browser cache/headers are not the compiler budget.',
    }
    await writeFile(destination, JSON.stringify(result, null, 2) + '\n')
  }
}

await WebsiteMigrationAcceptance.run()

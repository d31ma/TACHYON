#!/usr/bin/env node
// @ts-check
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { lstat, mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { withServer } from '../../scripts/release/server-probe.mjs'

/** @typedef {{question: string, answer: string}} FaqEntry */
/** @typedef {{package: string, version: string, license: string, sha256: string}} VendorMetadata */
/** @typedef {{tac: {scripts: string[], scriptSources: Record<string, VendorMetadata>}}} SiteManifest */
/** @typedef {{tag: string, commit: string, sourceInputs: {path: string, upstreamPath: string, sha256: string}[]}} InputManifest */
/** @typedef {{group: string, panel: string, header: string, body: string, wrapper: string, bar: string, barBody: string}} Selectors */
/** @typedef {{question: string, answer: string, open: boolean, expanded: string | null, controls: string, bodyId: string, role: string | null, buttonType: string, tabIndex: number, disabled: boolean, bodyDisplay: string, bodyHeight: number, moving: boolean, shape: string}} PanelState */
/** @typedef {{selected: string | null, single: boolean, multiple: boolean, panels: PanelState[]}} FaqState */
/** @typedef {{root: string | null, main: string | null, vendor: string | undefined, background: string, customElements: {name: string, defined: boolean}[]}} ThemeState */
/** @typedef {{scrolled: boolean, elevated: boolean, offset: number, top: number, position: string, shadow: string, backdrop: string, behavior: string | null, sticky: boolean}} BarState */
/** @typedef {{theme: string, reducedMotion: 'reduce' | 'no-preference', requestedOffset: number, state: BarState}} BarObservation */
/** @typedef {{theme: string, reducedMotion: 'reduce' | 'no-preference', initial: FaqState, selection: FaqState[], keyboard: {key: string, focusedIndex: number}[], bar: BarState[], appearance: ThemeState}} ContextProof */
/** @typedef {{sourceCommit: string, compilerSha256: string, clientSha256: string, output: string, gateSha256: string, inputManifestSha256: string, browserVersion?: string, vendors: Record<string, unknown>[], contexts: ContextProof[], barObservations: BarObservation[], errors: string[]}} VendorReceipt */

class VendorComponentAcceptance {
  /** @type {number} */
  static #timeout = 45_000
  /** @type {string} */
  static #website = fileURLToPath(new URL('..', import.meta.url))
  /** @type {string} */
  static #version = '26.35.01'
  /** @type {string} */
  static #upstreamCommit = '17aba01d5da9f4ceeb9b6373ebb99157a06ed44f'
  /** @type {string} */
  static #wc = '/shared/assets/duvay/duvay-wc.min.js'
  /** @type {string} */
  static #themeScript = '/shared/assets/duvay/duvay.min.js'
  /** @type {string} */
  static #themeSha = '50ca0fd49f2232e3520f9e3d1fe8efa1fec1948a69e1076d43ed649a09b80d5b'
  /** @type {string} */
  static #cssSha = 'dd0b9a6841313bc3b4bf30da476dc1aa88821a58220d662b679f83873b92c768'
  /** @type {string[]} */
  static #elements = ['w-app-bar', 'w-btn', 'w-container', 'w-divider', 'w-expansion-panel', 'w-expansion-panels']
  /** @type {Record<string, string>} */
  static #pinned = {
    'src/components/app-bar.js': '3b19aa6253738234f4b216308b605bcca2271b5c78e3419d669557cb2dc06353',
    'src/components/expansion-panel.js': 'c0e7a72b7e58006ab012811db7cc475e2dabb5159f43b6c4bfb122a6942e22fa',
    'src/components/expansion-panels.js': '72bf65127add10933083bc9a8af87e4fa3f4800da3cd22e6bb294bad07c785a9',
  }
  /** @type {Selectors} */
  static #selectors = {
    group: 'section[aria-labelledby="faq-heading"] w-expansion-panels',
    panel: 'w-expansion-panel',
    header: 'button.w-expand-header',
    body: '.w-expand-body',
    wrapper: '.w-expand',
    bar: 'w-app-bar[sticky][scroll-behavior="elevate"]',
    barBody: 'header.w-app-bar',
  }

  /** @returns {Promise<void>} */
  static async run() {
    const binary = path.resolve(VendorComponentAcceptance.#required('TAC_BIN'))
    const output = await VendorComponentAcceptance.#directory('TAC_WEBSITE_OUTPUT')
    const destination = path.resolve(VendorComponentAcceptance.#required('TAC_VENDOR_COMPONENTS_RECEIPT'))
    VendorComponentAcceptance.#externalReceipt(destination)
    const inputPath = path.resolve(VendorComponentAcceptance.#required('TAC_VENDOR_INPUT_MANIFEST'))
    const sourceCommit = VendorComponentAcceptance.#required('TAC_WEBSITE_SOURCE_COMMIT')
    assert.match(sourceCommit, /^[0-9a-f]{40}$/)
    const inputManifestSha256 = await VendorComponentAcceptance.#inputs(inputPath)
    const manifest = /** @type {SiteManifest} */ (JSON.parse(await readFile(path.join(VendorComponentAcceptance.#website, 'client/manifest.json'), 'utf8')))
    const faq = /** @type {FaqEntry[]} */ (JSON.parse(await readFile(path.join(VendorComponentAcceptance.#website, 'client/shared/data/faq.json'), 'utf8')))
    assert.ok(faq.length >= 2, 'Actual FAQ data supplies distinct sibling panels')
    /** @type {VendorReceipt} */
    const receipt = {
      sourceCommit, output, inputManifestSha256,
      compilerSha256: VendorComponentAcceptance.#sha(await readFile(binary)),
      clientSha256: await VendorComponentAcceptance.#clientHash(),
      gateSha256: VendorComponentAcceptance.#sha(await readFile(fileURLToPath(import.meta.url))),
      vendors: [], contexts: [], barObservations: [], errors: [],
    }
    const started = performance.now()
    try {
      await withServer(binary, ['preview', VendorComponentAcceptance.#website, '--static', '--out-dir', output, '--port', '0'], async request => {
        const home = await request('/')
        assert.equal(home.status, 200)
        const origin = new URL(home.url).origin
        receipt.vendors = await VendorComponentAcceptance.#vendors(request, manifest, output)
        await VendorComponentAcceptance.#browser(origin, faq, receipt)
      })
      assert.equal(await VendorComponentAcceptance.#clientHash(), receipt.clientSha256, 'Authored website bytes remained frozen')
      assert.equal(VendorComponentAcceptance.#sha(await readFile(binary)), receipt.compilerSha256, 'Compiler bytes remained frozen')
      assert.deepEqual(receipt.errors, [])
      await VendorComponentAcceptance.#receipt(destination, receipt, started, 'pass')
      console.log(JSON.stringify({ status: 'pass', receipt: destination, compilerSha256: receipt.compilerSha256, clientSha256: receipt.clientSha256 }))
    } catch (error) {
      await VendorComponentAcceptance.#receipt(destination, receipt, started, 'fail', String(error))
      throw error
    }
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} must identify an explicit qualification input`)
    return value
  }

  /** @param {string} name @returns {Promise<string>} */
  static async #directory(name) {
    const directory = path.resolve(VendorComponentAcceptance.#required(name))
    const metadata = await lstat(directory)
    assert.ok(metadata.isDirectory() && !metadata.isSymbolicLink(), `${name} must retain a real existing directory`)
    return directory
  }

  /** @param {string} destination @returns {void} */
  static #externalReceipt(destination) {
    const relative = path.relative(VendorComponentAcceptance.#website, destination)
    assert.ok(relative.startsWith('..' + path.sep) || path.isAbsolute(relative), 'Receipt must remain outside synced website source')
  }

  /** @param {Uint8Array | string} bytes @returns {string} */
  static #sha(bytes) {
    return createHash('sha256').update(bytes).digest('hex')
  }

  /** @returns {Promise<string>} */
  static async #clientHash() {
    const root = path.join(VendorComponentAcceptance.#website, 'client')
    const names = (await readdir(root, { recursive: true })).sort()
    const hash = createHash('sha256')
    for (const name of names) {
      const file = path.join(root, name)
      const metadata = await lstat(file)
      assert.ok(!metadata.isSymbolicLink(), `Authored bytes must not redirect through symlinks: ${name}`)
      if (metadata.isFile()) hash.update(name).update('\0').update(VendorComponentAcceptance.#sha(await readFile(file))).update('\n')
    }
    return hash.digest('hex')
  }

  /** @param {string} inputPath @returns {Promise<string>} */
  static async #inputs(inputPath) {
    const bytes = await readFile(inputPath)
    const input = /** @type {InputManifest} */ (JSON.parse(bytes.toString('utf8')))
    assert.equal(input.tag, 'v' + VendorComponentAcceptance.#version)
    assert.equal(input.commit, VendorComponentAcceptance.#upstreamCommit)
    for (const [upstreamPath, sha256] of Object.entries(VendorComponentAcceptance.#pinned)) {
      assert.equal(input.sourceInputs.find(item => item.upstreamPath === upstreamPath)?.sha256, sha256, upstreamPath)
    }
    for (const item of input.sourceInputs) {
      const file = path.resolve(path.dirname(inputPath), item.path)
      assert.ok(file.startsWith(path.dirname(inputPath) + path.sep), 'Pinned inputs remain in their retained source closure')
      assert.ok(!(await lstat(file)).isSymbolicLink(), item.path)
      assert.equal(VendorComponentAcceptance.#sha(await readFile(file)), item.sha256, item.upstreamPath)
    }
    return VendorComponentAcceptance.#sha(bytes)
  }

  /** @param {import('../../scripts/release/server-probe.mjs').ProbeRequest} request @param {SiteManifest} manifest @param {string} output @returns {Promise<Record<string, unknown>[]>} */
  static async #vendors(request, manifest, output) {
    assert.ok(manifest.tac.scripts.includes(VendorComponentAcceptance.#wc))
    assert.ok(manifest.tac.scripts.includes(VendorComponentAcceptance.#themeScript))
    const assets = []
    for (const resource of [VendorComponentAcceptance.#wc, VendorComponentAcceptance.#themeScript]) {
      const metadata = manifest.tac.scriptSources[resource]
      assert.ok(metadata, `${resource} has explicit pinned metadata`)
      assert.equal(metadata.package, 'duvay')
      assert.equal(metadata.version, VendorComponentAcceptance.#version)
      assert.equal(metadata.license, 'MIT')
      const source = await readFile(path.join(VendorComponentAcceptance.#website, 'client', resource.slice(1)))
      const emitted = await readFile(path.join(output, resource.slice(1)))
      const response = await request(resource)
      assert.equal(response.status, 200, resource)
      const served = new Uint8Array(await response.arrayBuffer())
      const sha256 = VendorComponentAcceptance.#sha(served)
      assert.equal(sha256, metadata.sha256, `${resource} serves exactly its declaration`)
      assert.equal(sha256, VendorComponentAcceptance.#sha(source), `${resource} preserves captured source bytes`)
      assert.equal(sha256, VendorComponentAcceptance.#sha(emitted), `${resource} preserves retained output bytes`)
      assets.push({ resource, sha256, decodedBytes: served.length })
    }
    assert.equal(manifest.tac.scriptSources[VendorComponentAcceptance.#themeScript].sha256, VendorComponentAcceptance.#themeSha, 'Separate pinned WTheme implementation is preserved')
    const css = await request('/shared/assets/duvay/duvay.min.css')
    assert.equal(css.status, 200)
    assert.equal(VendorComponentAcceptance.#sha(new Uint8Array(await css.arrayBuffer())), VendorComponentAcceptance.#cssSha, 'Pinned vendor styles remain unchanged')
    return assets
  }

  /** @param {string} origin @param {FaqEntry[]} faq @param {VendorReceipt} receipt @returns {Promise<void>} */
  static async #browser(origin, faq, receipt) {
    const load = createRequire(import.meta.url)
    const dependencies = path.resolve(VendorComponentAcceptance.#required('TAC_BROWSER_DEPENDENCIES'))
    const { chromium } = /** @type {typeof import('playwright')} */ (load(path.join(dependencies, 'playwright')))
    const browser = await chromium.launch({ timeout: VendorComponentAcceptance.#timeout })
    receipt.browserVersion = browser.version()
    try {
      for (const theme of ['light', 'dark']) {
        for (const motion of /** @type {('reduce' | 'no-preference')[]} */ (['no-preference', 'reduce'])) {
          receipt.contexts.push(await VendorComponentAcceptance.#context(browser, origin, faq, { theme, motion }, receipt))
        }
      }
      const shape = receipt.contexts[0].initial.panels.map(panel => panel.shape)
      for (const proof of receipt.contexts) assert.deepEqual(proof.initial.panels.map(panel => panel.shape), shape, 'FAQ markup shape is unchanged across theme and motion contexts')
      assert.notEqual(receipt.contexts[0].appearance.background, receipt.contexts[2].appearance.background, 'Light and dark CSS themes both affect the real page')
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {FaqEntry[]} faq @param {{theme: string, motion: 'reduce' | 'no-preference'}} settings @param {VendorReceipt} receipt @returns {Promise<ContextProof>} */
  static async #context(browser, origin, faq, settings, receipt) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: settings.theme === 'dark' ? 'dark' : 'light', reducedMotion: settings.motion })
    try {
      await context.addInitScript((/** @type {string} */ theme) => localStorage.setItem('w-theme', theme), settings.theme)
      const page = await context.newPage()
      page.setDefaultTimeout(VendorComponentAcceptance.#timeout)
      page.on('pageerror', error => receipt.errors.push(String(error)))
      page.on('response', response => { if (response.status() >= 400) receipt.errors.push(`${response.status()} ${new URL(response.url()).pathname}`) })
      const response = await page.goto(origin + '/', { waitUntil: 'networkidle' })
      assert.equal(response?.status(), 200)
      await page.locator(VendorComponentAcceptance.#selectors.group).locator(VendorComponentAcceptance.#selectors.header).first().waitFor({ state: 'visible' })
      await page.waitForFunction(theme => document.documentElement.getAttribute('w-theme') === theme && document.querySelector('#main')?.getAttribute('data-theme') === theme, settings.theme)
      const appearance = await page.evaluate(VendorComponentAcceptance.appearance, VendorComponentAcceptance.#elements)
      VendorComponentAcceptance.#theme(appearance, settings.theme)
      const initial = await VendorComponentAcceptance.#selection(page, faq, -1)
      const bar = await VendorComponentAcceptance.#scroll(page, settings, receipt.barObservations)
      const selection = await VendorComponentAcceptance.#mouse(page, faq)
      const keyboard = await VendorComponentAcceptance.#keyboard(page, faq)
      return { theme: settings.theme, reducedMotion: settings.motion, initial, appearance, bar, selection, keyboard }
    } finally {
      await context.close()
    }
  }

  /** @param {string[]} names @returns {ThemeState} */
  static appearance(names) {
    const globals = /** @type {typeof globalThis & {WTheme?: {get(): string}}} */ (globalThis)
    return {
      root: document.documentElement.getAttribute('w-theme'),
      main: document.querySelector('#main')?.getAttribute('data-theme') ?? null,
      vendor: globals.WTheme?.get(),
      background: getComputedStyle(document.body).backgroundColor,
      customElements: names.map(name => ({ name, defined: Boolean(customElements.get(name)) })),
    }
  }

  /** @param {ThemeState} state @param {string} theme @returns {void} */
  static #theme(state, theme) {
    assert.equal(state.root, theme)
    assert.equal(state.main, theme)
    assert.equal(state.vendor, theme)
    assert.ok(state.customElements.every(element => element.defined), 'All six authored DuVay elements are registered')
  }

  /** @param {Selectors} selectors @returns {FaqState} */
  static faqState(selectors) {
    const group = document.querySelector(selectors.group)
    if (!group) throw new Error('Actual FAQ group is missing')
    const panels = [...group.querySelectorAll(selectors.panel)].map(panel => {
      const button = /** @type {HTMLButtonElement | null} */ (panel.querySelector(selectors.header))
      const body = /** @type {HTMLElement | null} */ (panel.querySelector(selectors.body))
      const wrapper = panel.querySelector(selectors.wrapper)
      if (!button || !body || !wrapper) throw new Error('Pinned expansion markup is incomplete')
      return {
        question: panel.getAttribute('header') ?? '', answer: body.textContent?.replace(/\s+/g, ' ').trim() ?? '',
        open: panel.hasAttribute('open'), expanded: button.getAttribute('aria-expanded'),
        controls: button.getAttribute('aria-controls') ?? '', bodyId: body.id, role: body.getAttribute('role'),
        buttonType: button.type, tabIndex: button.tabIndex, disabled: button.disabled,
        bodyDisplay: getComputedStyle(body).display, bodyHeight: body.getBoundingClientRect().height,
        moving: body.classList.contains('w-motion-expanding'),
        shape: panel.innerHTML.replaceAll(body.id, 'PANEL_BODY'),
      }
    })
    return { selected: group.getAttribute('value'), single: group.hasAttribute('single'), multiple: group.hasAttribute('multiple'), panels }
  }

  /** @param {import('playwright').Page} page @param {FaqEntry[]} faq @param {number} selected @returns {Promise<FaqState>} */
  static async #selection(page, faq, selected) {
    await page.waitForFunction(({ selectors, selected }) => {
      const panels = [...document.querySelectorAll(selectors.group + ' ' + selectors.panel)]
      return panels.every((panel, index) => {
        const body = panel.querySelector(selectors.body)
        const wrapper = panel.querySelector(selectors.wrapper)
        return panel.hasAttribute('open') === (index === selected) && wrapper?.classList.contains('open') === (index === selected) && body && !body.classList.contains('w-motion-expanding')
      })
    }, { selectors: VendorComponentAcceptance.#selectors, selected })
    const state = await page.evaluate(VendorComponentAcceptance.faqState, VendorComponentAcceptance.#selectors)
    assert.equal(state.single, true)
    assert.equal(state.multiple, false)
    assert.equal(state.selected, selected < 0 ? null : String(selected))
    assert.equal(state.panels.length, faq.length)
    assert.equal(new Set(state.panels.map(panel => panel.controls)).size, faq.length, 'Each panel owns a distinct controlled region')
    for (const [index, panel] of state.panels.entries()) {
      assert.equal(panel.question, faq[index].question)
      assert.equal(panel.answer, faq[index].answer.replaceAll('`', '').replace(/\s+/g, ' ').trim())
      assert.equal(panel.open, index === selected)
      assert.equal(panel.expanded, String(index === selected))
      assert.ok(panel.controls)
      assert.equal(panel.controls, panel.bodyId)
      assert.equal(panel.role, 'region')
      assert.equal(panel.buttonType, 'button')
      assert.equal(panel.tabIndex, 0)
      assert.equal(panel.disabled, false)
      assert.equal(panel.bodyDisplay === 'none', index !== selected)
      assert.equal(panel.bodyHeight > 0, index === selected)
    }
    return state
  }

  /** @param {import('playwright').Page} page @param {FaqEntry[]} faq @returns {Promise<FaqState[]>} */
  static async #mouse(page, faq) {
    const headers = page.locator(VendorComponentAcceptance.#selectors.group).locator(VendorComponentAcceptance.#selectors.header)
    await headers.nth(0).click()
    const first = await VendorComponentAcceptance.#selection(page, faq, 0)
    await headers.nth(1).click()
    const second = await VendorComponentAcceptance.#selection(page, faq, 1)
    await headers.nth(1).click()
    const closed = await VendorComponentAcceptance.#selection(page, faq, -1)
    return [first, second, closed]
  }

  /** @param {import('playwright').Page} page @param {FaqEntry[]} faq @returns {Promise<{key: string, focusedIndex: number}[]>} */
  static async #keyboard(page, faq) {
    const headers = page.locator(VendorComponentAcceptance.#selectors.group).locator(VendorComponentAcceptance.#selectors.header)
    await headers.nth(0).focus()
    const keys = [['ArrowUp', faq.length - 1], ['ArrowDown', 0], ['ArrowRight', 1], ['ArrowLeft', 0], ['End', faq.length - 1], ['Home', 0]]
    const proof = []
    for (const [key, index] of keys) {
      await page.keyboard.press(String(key))
      assert.equal(await headers.nth(Number(index)).evaluate(element => element === document.activeElement), true, String(key))
      proof.push({ key: String(key), focusedIndex: Number(index) })
    }
    await page.keyboard.press('Enter')
    await VendorComponentAcceptance.#selection(page, faq, 0)
    await page.keyboard.press('ArrowDown')
    await page.keyboard.press('Space')
    await VendorComponentAcceptance.#selection(page, faq, 1)
    await page.keyboard.press('Space')
    await VendorComponentAcceptance.#selection(page, faq, -1)
    return proof
  }

  /** @param {Selectors} selectors @returns {BarState} */
  static barState(selectors) {
    const host = document.querySelector(selectors.bar)
    const bar = host?.querySelector(selectors.barBody)
    if (!host || !bar) throw new Error('Actual sticky elevation app bar is missing')
    const computed = getComputedStyle(bar)
    return {
      scrolled: host.hasAttribute('data-scrolled'), elevated: bar.classList.contains('w-app-bar--scrolled'),
      offset: window.scrollY, top: host.getBoundingClientRect().top, position: getComputedStyle(host).position,
      shadow: computed.boxShadow, backdrop: computed.backdropFilter, behavior: host.getAttribute('scroll-behavior'), sticky: host.hasAttribute('sticky'),
    }
  }

  /** @param {import('playwright').Page} page @param {{theme: string, motion: 'reduce' | 'no-preference'}} settings @param {BarObservation[]} observations @returns {Promise<BarState[]>} */
  static async #scroll(page, settings, observations) {
    const states = []
    for (const offset of [0, 200, 0]) {
      await page.evaluate(value => window.scrollTo(0, value), offset)
      await page.waitForFunction(({ selectors, scrolled }) => {
        const host = document.querySelector(selectors.bar)
        const bar = host?.querySelector(selectors.barBody)
        return host?.hasAttribute('data-scrolled') === scrolled && bar?.classList.contains('w-app-bar--scrolled') === scrolled && (scrolled ? window.scrollY > 4 : window.scrollY === 0)
      }, { selectors: VendorComponentAcceptance.#selectors, scrolled: offset > 0 })
      const state = await page.evaluate(VendorComponentAcceptance.barState, VendorComponentAcceptance.#selectors)
      observations.push({ theme: settings.theme, reducedMotion: settings.motion, requestedOffset: offset, state })
      assert.equal(state.behavior, 'elevate')
      assert.equal(state.sticky, true)
      assert.equal(state.position, 'sticky')
      assert.ok(Math.abs(state.top) <= 1, 'App bar remains at the viewport top without injected layout')
      assert.equal(state.scrolled, offset > 0)
      assert.equal(state.elevated, offset > 0)
      if (offset > 0) {
        assert.notEqual(state.shadow, 'none', 'Pinned elevation styles apply')
        assert.notEqual(state.backdrop, 'none', 'Pinned sticky frost styles apply')
      }
      states.push(state)
    }
    return states
  }

  /** @param {string} destination @param {VendorReceipt} receipt @param {number} started @param {'pass' | 'fail'} status @param {string} [error] @returns {Promise<void>} */
  static async #receipt(destination, receipt, started, status, error) {
    await mkdir(path.dirname(destination), { recursive: true })
    await writeFile(destination, JSON.stringify({
      ...receipt, status, error, elapsedMilliseconds: performance.now() - started,
      scope: 'Actual retained website FAQ and app-bar contracts from pinned DuVay26.35.01 source',
      sourceCommitBasis: 'Explicit caller-supplied checkpoint; actual client/compiler/vendor byte hashes recorded',
      upstreamCommit: VendorComponentAcceptance.#upstreamCommit,
      runtimeReplaced: false, stylesInjected: false, rebuildPerformed: false,
      preferenceFixtureOnly: true, currentFullVendorRuntimeBaselineCompared: false,
      runtimeEquivalenceLimit: 'No current full-vendor website output exists under the ordinary budget; these tests establish specific preserved source contracts, not complete runtime or pixel equivalence',
      releaseBudgetCheckedByThisGate: false,
    }, null, 2) + '\n')
  }
}

await VendorComponentAcceptance.run()

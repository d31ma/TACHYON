#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { SecureTestOrigin } from './secure-test-origin.mjs'
import assert from 'node:assert/strict'
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'

/** @typedef {import('playwright').BrowserType} BrowserType */
/** @typedef {import('playwright').BrowserContext} BrowserContext */
/** @typedef {import('playwright').Page} Page */
/** @typedef {{identity: string, count: number, recalled: string}} ServiceState */
/** @typedef {{loaded: string[], cached: string[]}} KeptFiles */
/** @typedef {{language: string, browser: string, files: number, services: string[], offlineAttempts: string[]}} LanguageResult */

class WasmOfflineGate {
  static fixtures = fileURLToPath(new URL('./page-program/', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static manifest = { tac: { duplication: { check: false } } }
  static engines = ['chromium', 'firefox', 'webkit']
  /** @type {Record<string, BrowserType>} */
  static launchers = { chromium, firefox, webkit }
  static selected = (process.env.TAC_WASM_OFFLINE_BROWSERS ?? WasmOfflineGate.engines.join(',')).split(',')
  static emulatedOffline = ['chromium', 'firefox']
  static offlineNote = /^Offline: .*$/gm
  static browserRequests = ['/favicon.ico']
  static revalidated = ['/', '/tachyon-sw.js']
  static services = '/.tachyon/services/'
  static remembered = 'kept offline'
  static stepMilliseconds = 20_000
  static settleStep = 100

  /** @returns {string[]} */
  static languages() {
    const chosen = process.env.TAC_WASM_OFFLINE_LANGUAGES?.split(',')
    const present = readdirSync(WasmOfflineGate.fixtures).filter(name => existsSync(path.join(WasmOfflineGate.fixtures, name, 'expected.json')))
    const unknown = (chosen ?? []).filter(name => !present.includes(name))
    if (unknown.length) throw new Error(`no page program fixture for ${unknown.join(', ')}; the fixtures are ${present.join(', ')}`)
    return chosen ?? present.filter(name => !JSON.parse(readFileSync(path.join(WasmOfflineGate.fixtures, name, 'expected.json'), 'utf8')).owed)
  }

  /** @param {string} language @returns {string} */
  static project(language) {
    const project = mkdtempSync(path.join(tmpdir(), `ty-wasm-offline-${language}-`))
    cpSync(path.join(WasmOfflineGate.fixtures, 'shared'), project, { recursive: true })
    cpSync(path.join(WasmOfflineGate.fixtures, language), project, { recursive: true })
    rmSync(path.join(project, 'expected.json'))
    writeFileSync(path.join(project, 'client/manifest.json'), `${JSON.stringify(WasmOfflineGate.manifest)}\n`)
    return project
  }

  /** @returns {Promise<void>} */
  static async run() {
    const unknown = WasmOfflineGate.selected.filter(name => !WasmOfflineGate.engines.includes(name))
    if (unknown.length) throw new Error(`unknown browser ${unknown.join(', ')}; nothing was run`)
    /** @type {LanguageResult[]} */
    const results = []
    for (const language of WasmOfflineGate.languages()) results.push(...await WasmOfflineGate.language(language))
    console.table(results.map(result => ({ ...result, services: result.services.join(' '), offlineAttempts: result.offlineAttempts.join(' ') })))
    console.log('PASS: WebAssembly service languages offline browser gate')
  }

  /** @param {string} language @returns {Promise<LanguageResult[]>} */
  static async language(language) {
    const project = WasmOfflineGate.project(language)
    /** @type {SecureTestOrigin|undefined} */
    let origin
    try {
      const output = await SecureTestOrigin.bundle(WasmOfflineGate.binary, project)
      const notes = output.match(WasmOfflineGate.offlineNote) ?? []
      assert.deepEqual(notes, [], `${language}: the build keeps every file within the service worker's limit`)
      origin = await SecureTestOrigin.open(WasmOfflineGate.binary, project)
      /** @type {LanguageResult[]} */
      const results = []
      for (const browser of WasmOfflineGate.selected) results.push(await WasmOfflineGate.probe(language, browser, project, origin))
      return results
    } finally {
      await origin?.close()
      rmSync(project, { recursive: true, force: true })
    }
  }

  /** @param {string} language @param {string} browser @param {string} project @param {SecureTestOrigin} origin @returns {Promise<LanguageResult>} */
  static async probe(language, browser, project, origin) {
    const label = `${browser} ${language}`
    const context = await WasmOfflineGate.launchers[browser].launchPersistentContext(mkdtempSync(path.join(project, `${browser}-profile-`)), {
      ...origin.launchOptions(browser),
      headless: true,
    })
    context.setDefaultTimeout(WasmOfflineGate.stepMilliseconds)
    context.setDefaultNavigationTimeout(WasmOfflineGate.stepMilliseconds)
    try {
      const page = context.pages()[0] ?? await context.newPage()
      /** @type {string[]} */
      const errors = []
      page.on('pageerror', error => errors.push(String(error)))
      await page.goto(`${SecureTestOrigin.origin}/`, { waitUntil: 'load' })
      assert.equal(await SecureTestOrigin.controlled(page, WasmOfflineGate.stepMilliseconds / 2), 'ready', `${label}: the Tachyon service worker controls the page`)
      await page.reload({ waitUntil: 'load' })
      const online = await WasmOfflineGate.opened(page, label)
      await WasmOfflineGate.call(page, 'writer', 'remember', WasmOfflineGate.remembered)
      await WasmOfflineGate.call(page, 'reader', 'recall')
      const recalled = await WasmOfflineGate.settled(page, state => state.recalled === WasmOfflineGate.remembered, 'reader')
      assert.equal(recalled.recalled, WasmOfflineGate.remembered, `${label}: the reader service finds the writer's record online`)
      await page.evaluate(() => globalThis[Symbol.for('tachyon.tac.storage')].settled())
      const kept = await WasmOfflineGate.kept(page, label)
      const offlineAttempts = await WasmOfflineGate.offline(context, page, label, origin, online)
      assert.equal(errors.join('\n'), '', `${label}: the page threw no error`)
      return { language, browser, files: kept.loaded.length, services: kept.loaded.filter(file => file.startsWith(WasmOfflineGate.services)), offlineAttempts }
    } finally {
      await context.close()
    }
  }

  /** Every same-origin file the page loaded must be in the worker cache before the network goes away.
   * @param {Page} page @param {string} label @returns {Promise<KeptFiles>} */
  static async kept(page, label) {
    /** @type {KeptFiles} */
    let kept = { loaded: [], cached: [] }
    const deadline = Date.now() + WasmOfflineGate.stepMilliseconds
    while (Date.now() < deadline) {
      kept = await page.evaluate(WasmOfflineGate.readCache, WasmOfflineGate.browserRequests)
      if (kept.loaded.every(file => kept.cached.includes(file))) break
      await page.waitForTimeout(WasmOfflineGate.settleStep)
    }
    const missing = kept.loaded.filter(file => !kept.cached.includes(file))
    assert.deepEqual(missing, [], `${label}: the worker keeps every file the page loaded (cached: ${kept.cached.join(' ')})`)
    return kept
  }

  /** @param {string[]} ignored @returns {Promise<KeptFiles>} */
  static async readCache(ignored) {
    const loaded = performance.getEntriesByType('resource')
      .map(entry => new URL(entry.name))
      .filter(url => url.origin === location.origin && url.search === '' && !url.pathname.startsWith('/tachyon-sw.js') && !ignored.includes(url.pathname))
      .map(url => url.pathname)
    const name = (await caches.keys()).find(value => value.startsWith('tachyon-static-'))
    const cached = name ? (await (await caches.open(name)).keys()).map(entry => new URL(entry.url).pathname) : []
    return { loaded: [...new Set(loaded)].sort(), cached: cached.sort() }
  }

  /** @param {BrowserContext} context @param {Page} page @param {string} label @param {SecureTestOrigin} origin @param {ServiceState} online @returns {Promise<string[]>} */
  static async offline(context, page, label, origin, online) {
    origin.offlineAttempts = []
    origin.networkDown = true
    const emulated = WasmOfflineGate.emulatedOffline.includes(label.split(' ')[0])
    if (emulated) await context.setOffline(true)
    try {
      await page.reload({ waitUntil: 'load' })
      const reopened = await WasmOfflineGate.opened(page, label)
      assert.notEqual(reopened.identity, online.identity, `${label}: the services started again from the cache after the offline reload`)
      await WasmOfflineGate.call(page, 'writer', 'increment')
      const counted = await WasmOfflineGate.settled(page, state => state.count === reopened.count + 1, 'writer')
      assert.equal(counted.count, reopened.count + 1, `${label}: the writer service runs offline`)
      await WasmOfflineGate.call(page, 'reader', 'recall')
      const recalled = await WasmOfflineGate.settled(page, state => state.recalled === WasmOfflineGate.remembered, 'reader')
      assert.equal(recalled.recalled, WasmOfflineGate.remembered, `${label}: the reader service finds the record saved before the network went away`)
      const attempts = [...new Set(origin.offlineAttempts)]
      const reached = attempts.filter(file => ![...WasmOfflineGate.revalidated, ...WasmOfflineGate.browserRequests].includes(file))
      assert.deepEqual(reached, [], `${label}: every file came from the worker cache, not the network`)
      console.log(`ok - ${label}: the page and its services ran offline`)
      return attempts
    } finally {
      if (emulated) await context.setOffline(false)
      origin.networkDown = false
    }
  }

  /** @param {Page} page @param {string} label @returns {Promise<ServiceState>} */
  static async opened(page, label) {
    await WasmOfflineGate.call(page, 'writer', 'inspect')
    const state = await WasmOfflineGate.settled(page, value => value.identity !== '', 'writer')
    assert.notEqual(state.identity, '', `${label}: the writer service answers through its client`)
    return state
  }

  /** @param {Page} page @param {string} role @param {string} method @param {...string} values @returns {Promise<void>} */
  static async call(page, role, method, ...values) {
    const deadline = Date.now() + WasmOfflineGate.stepMilliseconds
    let failure = `the ${role} service never offered ${method}`
    while (Date.now() < deadline) {
      const called = await page.evaluate(async ([name, member, given]) => {
        const service = globalThis.__tachyonTac?.service(name)
        if (!service || typeof service[member] !== 'function') return 'not started'
        try {
          await service[member](...given)
          return ''
        } catch (error) {
          return String(error)
        }
      }, /** @type {[string, string, string[]]} */ ([role, method, values]))
      if (called === '') return
      failure = `the ${role} service could not run ${method}: ${called}`
      await page.waitForTimeout(WasmOfflineGate.settleStep)
    }
    throw new Error(failure)
  }

  /** @param {Page} page @param {(state: ServiceState) => boolean} done @param {string} role @returns {Promise<ServiceState>} */
  static async settled(page, done, role) {
    const read = () => page.evaluate((name) => {
      const service = globalThis.__tachyonTac?.service(name)
      return { identity: service?.identity ?? '', count: service?.count ?? 0, recalled: service?.recalled ?? '' }
    }, role)
    const deadline = Date.now() + WasmOfflineGate.stepMilliseconds
    let state = await read()
    while (!done(state) && Date.now() < deadline) {
      await page.waitForTimeout(WasmOfflineGate.settleStep)
      state = await read()
    }
    return state
  }
}

await WasmOfflineGate.run()

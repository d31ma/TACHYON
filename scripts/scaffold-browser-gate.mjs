#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'
import { SupervisedCommand } from './supervised-command.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} ScaffoldEngine */
/** @typedef {{name: string, options: string[], shellService: boolean, greeting: {tag: string, before: string, after: string} | null}} ScaffoldCase */
/** @typedef {{scaffold: string, engine: ScaffoldEngine, phase: string, errors: string[], failedRequests: string[], externalRequests: string[]}} ScaffoldDiagnostics */

class ScaffoldBrowserGate {
  static _timeout = 30_000
  static _serverTimeout = 300_000
  static _engines = ['chromium', 'firefox', 'webkit']
  static _foundationBackground = 'rgb(15, 23, 42)'
  static _foundationText = 'rgb(226, 232, 240)'
  static _greetingMargin = '24px'
  /** @type {ScaffoldCase[]} */
  static _cases = [
    { name: 'js', options: [], shellService: true, greeting: null },
    {
      name: 'ts',
      options: ['--tac', 'ts', '--yon', 'ts'],
      shellService: false,
      greeting: { tag: 'greeting-typescript', before: 'Hello from TypeScript', after: 'TypeScript says hello again.' },
    },
  ]

  /** @returns {Promise<void>} */
  static async run() {
    const binary = ScaffoldBrowserGate._binary()
    const engines = ScaffoldBrowserGate._selectedEngines()
    const workspace = await mkdtemp(path.join(tmpdir(), 'tac-scaffold-'))
    try {
      for (const scaffold of ScaffoldBrowserGate._cases) {
        const project = await ScaffoldBrowserGate._initialize(binary, workspace, scaffold)
        await ScaffoldBrowserGate._serve(binary, project, scaffold, engines)
      }
    } finally {
      await rm(workspace, { recursive: true, force: true })
    }
  }

  /** @returns {string} */
  static _binary() {
    const binary = process.env.TAC_BIN ?? CargoArtifacts.path('release', 'ty')
    return path.resolve(binary)
  }

  /** @returns {ScaffoldEngine[]} */
  static _selectedEngines() {
    const requested = process.env.TAC_SCAFFOLD_BROWSERS
    if (!requested) return /** @type {ScaffoldEngine[]} */ (ScaffoldBrowserGate._engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(ScaffoldBrowserGate._engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {ScaffoldEngine[]} */ (engines)
  }

  /** @param {string} binary @param {string} workspace @param {ScaffoldCase} scaffold @returns {Promise<string>} */
  static async _initialize(binary, workspace, scaffold) {
    const project = path.join(workspace, scaffold.name)
    const args = ['init', project, '--name', `Scaffold ${scaffold.name}`, '--skip-tools', ...scaffold.options]
    const result = await SupervisedCommand.run(binary, args, { timeoutMilliseconds: ScaffoldBrowserGate._timeout })
    assert.equal(result.status, 0, `ty init ${scaffold.name} failed: ${result.stderr}`)
    return project
  }

  /** @param {string} binary @param {string} project @param {ScaffoldCase} scaffold @param {ScaffoldEngine[]} engines @returns {Promise<void>} */
  static async _serve(binary, project, scaffold, engines) {
    const options = { timeoutMilliseconds: ScaffoldBrowserGate._serverTimeout, readinessTimeoutMilliseconds: ScaffoldBrowserGate._timeout }
    await withServer(binary, ['preview', project, '--port', '0'], async (request) => {
      const origin = new URL((await request('/')).url).origin
      for (const engine of engines) await ScaffoldBrowserGate._runEngine(engine, origin, scaffold)
    }, options)
  }

  /** @param {ScaffoldEngine} engine @returns {import('playwright').BrowserType} */
  static _browserType(engine) {
    if (engine === 'firefox') return firefox
    if (engine === 'webkit') return webkit
    return chromium
  }

  /** @param {ScaffoldEngine} engine @param {string} origin @param {ScaffoldCase} scaffold @returns {Promise<void>} */
  static async _runEngine(engine, origin, scaffold) {
    /** @type {ScaffoldDiagnostics} */
    const diagnostics = { scaffold: scaffold.name, engine, phase: 'starting', errors: [], failedRequests: [], externalRequests: [] }
    const browser = await ScaffoldBrowserGate._browserType(engine).launch({ timeout: ScaffoldBrowserGate._timeout })
    try {
      const page = await ScaffoldBrowserGate._open(browser, origin, diagnostics)
      diagnostics.phase = 'foundation-styles'
      await ScaffoldBrowserGate._checkFoundation(page)
      diagnostics.phase = 'shell'
      await ScaffoldBrowserGate._checkShell(page, scaffold)
      if (scaffold.greeting) {
        diagnostics.phase = 'greeting'
        await ScaffoldBrowserGate._checkGreeting(page, scaffold.greeting)
      }
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      console.log(JSON.stringify({ status: 'pass', scaffold: scaffold.name, engine, browserVersion: browser.version() }))
    } catch (error) {
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @param {ScaffoldDiagnostics} diagnostics @returns {Promise<import('playwright').Page>} */
  static async _open(browser, origin, diagnostics) {
    const page = await browser.newPage()
    page.on('pageerror', (error) => diagnostics.errors.push(error.message))
    page.on('console', (message) => {
      if (message.type() === 'error') diagnostics.errors.push(message.text())
    })
    page.on('requestfailed', (request) => diagnostics.failedRequests.push(request.url()))
    page.on('request', (request) => {
      const url = new URL(request.url())
      if (url.protocol.startsWith('http') && url.origin !== origin) diagnostics.externalRequests.push(request.url())
    })
    await page.goto(`${origin}/`, { waitUntil: 'load', timeout: ScaffoldBrowserGate._timeout })
    await page.locator('[data-tachyon-active="true"]').first().waitFor({ timeout: ScaffoldBrowserGate._timeout })
    return page
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkFoundation(page) {
    const styles = await page.evaluate(() => {
      const body = getComputedStyle(document.body)
      const root = getComputedStyle(document.documentElement)
      return { background: body.backgroundColor, text: body.color, margin: body.marginTop, colorScheme: root.colorScheme }
    })
    assert.deepEqual(styles, {
      background: ScaffoldBrowserGate._foundationBackground,
      text: ScaffoldBrowserGate._foundationText,
      margin: '0px',
      colorScheme: 'dark',
    })
  }

  /** @param {import('playwright').Page} page @param {ScaffoldCase} scaffold @returns {Promise<void>} */
  static async _checkShell(page, scaffold) {
    await page.locator('.hero h1').waitFor({ timeout: ScaffoldBrowserGate._timeout })
    const brand = await page.locator('.shell .brand strong').textContent()
    assert.equal(brand, 'Tachyon')
    const heroBackground = await page.locator('.hero').evaluate((hero) => getComputedStyle(hero).backgroundImage)
    assert.match(heroBackground, /linear-gradient/)
    if (!scaffold.shellService) return
    const title = `Scaffold ${scaffold.name}`
    await page.waitForFunction((expected) => document.title === expected, title, { timeout: ScaffoldBrowserGate._timeout })
  }

  /** @param {import('playwright').Page} page @param {{tag: string, before: string, after: string}} greeting @returns {Promise<void>} */
  static async _checkGreeting(page, greeting) {
    const section = page.locator(`[data-tachyon-component="${greeting.tag}"] .greeting`)
    const message = section.locator('p')
    await message.waitFor({ timeout: ScaffoldBrowserGate._timeout })
    assert.equal(await message.textContent(), greeting.before)
    const spacing = await section.evaluate((element) => {
      const hero = document.querySelector('.hero')
      const gap = element.getBoundingClientRect().top - (hero?.getBoundingClientRect().bottom ?? 0)
      return { margin: getComputedStyle(element).marginTop, gap: Math.round(gap) }
    })
    assert.deepEqual(spacing, { margin: ScaffoldBrowserGate._greetingMargin, gap: 24 })
    await section.locator('button').click()
    await page.waitForFunction(
      ([tag, expected]) => document.querySelector(`[data-tachyon-component="${tag}"] .greeting p`)?.textContent === expected,
      [greeting.tag, greeting.after],
      { timeout: ScaffoldBrowserGate._timeout },
    )
  }
}

await ScaffoldBrowserGate.run()

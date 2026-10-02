#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash, randomUUID } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {{message: string}} HttpAnswer */
/** @typedef {{last: string, read: () => Promise<HttpAnswer>, slow: () => Promise<HttpAnswer>, invalidPath: () => Promise<unknown>, __tachyonDispose: () => void, __tachyonStoredChanged?: () => void}} HttpFacade */
/** @typedef {{new(): HttpFacade}} HttpConstructor */
/** @typedef {{message: string, code?: string}} HttpRefusal */
/** @typedef {{path: string, error?: string}} FailedRequest */
/** @typedef {'chromium' | 'firefox' | 'webkit'} HttpBrowserEngine */
/** @typedef {{phase: string, sourceCommit: string, compilerSha256: string, browserEngine: HttpBrowserEngine, browserVersion?: string, receipt: string, errors: string[], failedRequests: FailedRequest[], externalRequests: string[], requests: string[], chromiumVersion?: string}} HttpDiagnostics */
/** @typedef {{root: string, binary: string, diagnostics: HttpDiagnostics}} HttpFixture */
/** @typedef {{ordinary: HttpAnswer, invalidPath: HttpRefusal, cancelled: HttpRefusal, capturedRefusal: HttpRefusal, replacement: HttpAnswer, retiredChanges: number}} HttpProof */

class DotnetHttpBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    const fixture = await DotnetHttpBrowserGate.#read()
    await DotnetHttpBrowserGate.#execute(fixture)
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} must identify this explicit fixture`)
    return value
  }

  /** @returns {HttpBrowserEngine} */
  static #engine() {
    const engine = process.env.TAC_DOTNET_HTTP_BROWSER ?? 'chromium'
    assert.ok(engine === 'chromium' || engine === 'firefox' || engine === 'webkit', 'TAC_DOTNET_HTTP_BROWSER must be chromium, firefox or webkit')
    return engine
  }

  /** @param {HttpBrowserEngine} engine @returns {import('playwright').BrowserType} */
  static #browserType(engine) {
    switch (engine) {
      case 'chromium': return chromium
      case 'firefox': return firefox
      case 'webkit': return webkit
    }
  }

  /** @returns {Promise<HttpFixture>} */
  static async #read() {
    const browserEngine = DotnetHttpBrowserGate.#engine()
    const root = path.resolve(DotnetHttpBrowserGate.#required('TAC_DOTNET_HTTP_FIXTURE_OUT'))
    const binary = path.resolve(DotnetHttpBrowserGate.#required('TAC_BIN'))
    const sourceCommit = DotnetHttpBrowserGate.#required('TAC_DOTNET_HTTP_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const receipt = `dotnet-http-browser-${browserEngine}-${randomUUID()}.json`
    return { root, binary, diagnostics: { phase: 'starting', sourceCommit, compilerSha256, browserEngine, receipt, errors: [], failedRequests: [], externalRequests: [], requests: [] } }
  }

  /** @param {HttpFixture} fixture @returns {Promise<void>} */
  static async #execute(fixture) {
    const started = performance.now()
    try {
      const proof = await withServer(fixture.binary, ['serve', fixture.root, '--host', '127.0.0.1', '--port', '0', '--out-dir', 'dist', '--no-watch', '--no-self-restart', '--no-bundle'], async request => {
        const origin = new URL((await request('/')).url).origin
        return DotnetHttpBrowserGate.#probe(origin, fixture.diagnostics)
      })
      await DotnetHttpBrowserGate.#receipt(fixture, { status: 'pass', proof, elapsedMilliseconds: performance.now() - started })
      console.log(JSON.stringify({ browserEngine: fixture.diagnostics.browserEngine, receipt: fixture.diagnostics.receipt, proof }))
    } catch (error) {
      await DotnetHttpBrowserGate.#receipt(fixture, { status: 'fail', error: String(error).slice(0, 2_000), elapsedMilliseconds: performance.now() - started })
      throw error
    }
  }

  /** @param {HttpFixture} fixture @param {Record<string, unknown>} result @returns {Promise<void>} */
  static async #receipt(fixture, result) {
    await writeFile(path.join(fixture.root, fixture.diagnostics.receipt), JSON.stringify({ ...fixture.diagnostics, ...result, scope: 'ordinary C#/.NET Wasm default HTTP with genuine Yon, explicit Setup override and captured-owner refusal', browserHostReplaced: false, cacheRevalidationQualified: false, taskDtoQualified: false, asyncLifecycleQualified: false }, null, 2) + '\n', { flag: 'wx' })
  }

  /** @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<HttpProof>} */
  static async #probe(origin, diagnostics) {
    diagnostics.phase = 'browser-launch'
    const browser = await DotnetHttpBrowserGate.#browserType(diagnostics.browserEngine).launch({ timeout: 10_000 })
    const deadline = setTimeout(() => browser.close(), 45_000)
    diagnostics.browserVersion = browser.version()
    if (diagnostics.browserEngine === 'chromium') diagnostics.chromiumVersion = browser.version()
    try {
      const context = await browser.newContext()
      await context.addCookies([{ name: 'private_session', value: 'must-not-travel', url: origin }])
      const page = await context.newPage()
      page.setDefaultTimeout(10_000)
      DotnetHttpBrowserGate.#observe(page, origin, diagnostics)
      await DotnetHttpBrowserGate.#view(page, origin, diagnostics)
      diagnostics.phase = 'captured-owner-transport'
      const proof = await DotnetHttpBrowserGate.#owners(page)
      DotnetHttpBrowserGate.#check(proof, diagnostics)
      diagnostics.phase = 'complete'
      return proof
    } finally {
      clearTimeout(deadline)
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<void>} */
  static async #view(page, origin, diagnostics) {
    diagnostics.phase = 'ordinary-view'
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
    await page.waitForFunction(() => document.querySelector('#http-answer')?.textContent === 'ready')
    const responsePromise = page.waitForResponse(response => new URL(response.url()).pathname === '/api/probe')
    await page.locator('#http-read').click()
    const response = await responsePromise
    assert.equal(response.status(), 200)
    assert.equal((await response.request().allHeaders()).cookie ?? null, null, 'same-origin credentials are omitted')
    await page.waitForFunction(() => document.querySelector('#http-answer')?.textContent === 'actual Yon answer')
    assert.deepEqual(diagnostics.requests, ['/api/probe'], 'one authored click makes one real Yon request')
    await page.locator('#http-custom').click()
    await page.waitForFunction(() => document.querySelector('#http-override-answer')?.textContent === 'custom adapter')
    assert.deepEqual(diagnostics.requests, ['/api/probe'], 'explicit Setup adapter makes no browser request')
  }

  /** @param {import('playwright').Page} page @returns {Promise<HttpProof>} */
  static async #owners(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/HttpProbeService.js'
      /** @type {HttpConstructor} */
      const Service = (await import(modulePath)).default
      /** @param {unknown} error @returns {HttpRefusal} */
      const refusal = error => {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error), ...(typeof code === 'string' ? { code } : {}) }
      }
      const owner = new Service()
      const ordinary = await owner.read()
      const invalidPath = await owner.invalidPath().then(() => { throw new Error('Unsafe path succeeded.') }, refusal)
      let retiredChanges = 0
      owner.__tachyonStoredChanged = () => { retiredChanges++ }
      const pending = owner.slow().then(() => { throw new Error('Retired HTTP succeeded.') }, refusal)
      await new Promise(resolve => setTimeout(resolve, 75))
      const captured = owner.read
      owner.__tachyonDispose()
      /** @type {HttpRefusal} */
      let capturedRefusal
      try { await captured(); throw new Error('Retired captured method succeeded.') } catch (error) { capturedRefusal = refusal(error) }
      const cancelled = await pending
      const replacementOwner = new Service()
      const replacement = await replacementOwner.read()
      replacementOwner.__tachyonDispose()
      await new Promise(resolve => setTimeout(resolve, 550))
      return { ordinary, invalidPath, cancelled, capturedRefusal, replacement, retiredChanges }
    })
  }

  /** @param {HttpProof} proof @param {HttpDiagnostics} diagnostics @returns {void} */
  static #check(proof, diagnostics) {
    assert.deepEqual(proof.ordinary, { message: 'actual Yon answer' })
    assert.deepEqual(proof.replacement, { message: 'actual Yon answer' })
    assert.equal(proof.invalidPath.code, 'HTTP_PATH')
    assert.equal(proof.cancelled.code, 'CANCELLED')
    assert.equal(proof.capturedRefusal.code, 'CANCELLED')
    assert.equal(proof.retiredChanges, 0)
    assert.deepEqual(diagnostics.requests, ['/api/probe', '/api/probe', '/api/slow', '/api/probe'])
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    assert.ok(diagnostics.failedRequests.every(request => request.path === '/api/slow'), 'only deliberately cancelled HTTP may fail')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {void} */
  static #observe(page, origin, diagnostics) {
    page.on('pageerror', error => DotnetHttpBrowserGate.#record(diagnostics.errors, error.message.slice(0, 2_000)))
    page.on('request', request => {
      const url = new URL(request.url())
      if (url.pathname.startsWith('/api/')) DotnetHttpBrowserGate.#record(diagnostics.requests, url.pathname)
      if (url.protocol.startsWith('http') && url.origin !== origin) DotnetHttpBrowserGate.#record(diagnostics.externalRequests, url.pathname.slice(0, 240))
    })
    page.on('requestfailed', request => DotnetHttpBrowserGate.#record(diagnostics.failedRequests, { path: new URL(request.url()).pathname.slice(0, 240), error: request.failure()?.errorText }))
    page.on('response', response => {
      if (response.status() >= 400) DotnetHttpBrowserGate.#record(diagnostics.errors, `HTTP ${response.status()} ${new URL(response.url()).pathname.slice(0, 240)}`)
    })
  }

  /** @template T @param {T[]} values @param {T} value @returns {void} */
  static #record(values, value) {
    if (values.length < 32) values.push(value)
  }
}

await DotnetHttpBrowserGate.run()

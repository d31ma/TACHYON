#!/usr/bin/env node
import assert from 'node:assert/strict'
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'

/** @typedef {{message: string}} RegisteredGreeting */
/** @typedef {{message: string | null}} RegisteredNullableGreeting */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpRefusal} ModelRefusal */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpDiagnostics} HttpDiagnostics */
/** @typedef {{path: string, method: string, credentials: RequestCredentials, authorization: boolean}} ModelRequest */
/** @typedef {{message: string, ask: () => Promise<RegisteredGreeting>, nullable: () => Promise<RegisteredNullableGreeting>, missing: () => Promise<RegisteredGreeting>, wrongType: () => Promise<RegisteredGreeting>, extra: () => Promise<RegisteredGreeting>, slow: () => Promise<RegisteredGreeting>, __tachyonDispose: () => void, __tachyonStoredChanged?: () => void}} ModelOwner */
/** @typedef {{default: new() => ModelOwner}} ModelServiceModule */
/** @typedef {{owner: ModelOwner, captured: () => Promise<RegisteredGreeting>, changes: number, disposed: boolean}} ModelOwnerState */
/** @typedef {{nullable: RegisteredNullableGreeting, missing: ModelRefusal, wrongType: ModelRefusal, extra: ModelRefusal, privateProvider: boolean, unchangedMessage: string}} ModelDecodingProof */
/** @typedef {{cancelled: ModelRefusal, capturedRefusal: ModelRefusal, retiredChanges: number, replacement: RegisteredGreeting}} ModelLifetimeProof */

class DotnetTaskModelBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(DotnetTaskModelBrowserGate.#probe, 'ordinary C# Task of registered closed scalar Greeting/NullableGreeting records through genuine Yon and activation-owned HTTPClient; arbitrary Task DTO and service-worker cache unqualified')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(page, origin, diagnostics) {
    await DotnetTaskModelBrowserGate.#observe(page)
    diagnostics.phase = 'ordinary-registered-model-view'
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
    await page.waitForFunction(() => document.querySelector('#greeting-answer')?.textContent === '')
    const response = page.waitForResponse(answer => new URL(answer.url()).pathname === '/api/greeting')
    await page.locator('#greeting-ask').click()
    const answer = await response
    assert.equal(answer.status(), 200)
    assert.deepEqual(await answer.json(), { message: 'actual registered Yon answer' })
    const headers = await answer.request().allHeaders()
    assert.equal(headers.cookie ?? null, null)
    assert.equal(headers.authorization ?? null, null)
    await page.waitForFunction(() => document.querySelector('#greeting-answer')?.textContent === 'actual registered Yon answer')
    diagnostics.phase = 'registered-record-response-controls'
    const decoding = await DotnetTaskModelBrowserGate.#decoding(page)
    DotnetTaskModelBrowserGate.#checkDecoding(decoding)
    diagnostics.phase = 'registered-client-child-disposal'
    const lifetime = await DotnetTaskModelBrowserGate.#lifetime(page)
    const requests = await DotnetTaskModelBrowserGate.#requests(page)
    DotnetTaskModelBrowserGate.#checkRequests(requests)
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    assert.equal(diagnostics.failedRequests.length, 1, 'only the deliberately cancelled child request fails')
    assert.equal(diagnostics.failedRequests[0]?.path, '/api/greeting/slow')
    assert.deepEqual(diagnostics.requests, requests.map(request => request.path))
    diagnostics.phase = 'complete'
    return { decoding, lifetime, requests, registeredScalarTaskModelsQualified: true, arbitraryTaskDtoQualified: false, serviceWorkerQualified: false, browserHostReplaced: false }
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static #observe(page) {
    return page.addInitScript(() => {
      const original = globalThis.fetch.bind(globalThis)
      /** @type {ModelRequest[]} */
      const requests = []
      Reflect.set(globalThis, '__taskModelRequests', requests)
      globalThis.fetch = (input, options) => {
        const source = input instanceof Request ? input : undefined
        const url = new URL(source?.url ?? String(input), location.href)
        if (url.pathname.startsWith('/api/greeting') && requests.length < 12) {
          const headers = new Headers(options?.headers ?? source?.headers)
          requests.push({ path: url.pathname, method: options?.method ?? source?.method ?? 'GET', credentials: options?.credentials ?? source?.credentials ?? 'same-origin', authorization: headers.has('authorization') })
        }
        return original(input, options)
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<ModelDecodingProof>} */
  static #decoding(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/GreetingService.js'
      /** @type {ModelServiceModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      /** @param {unknown} error @returns {ModelRefusal} */
      const refusal = error => {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error).slice(0, 2_000), ...(typeof code === 'string' ? { code } : {}) }
      }
      try {
        const privateProvider = !Reflect.has(owner, '_greetingClient') && !Reflect.has(owner, 'greetingClient')
        const nullable = await owner.nullable()
        const missing = await owner.missing().then(() => ({ code: 'UNEXPECTED_SUCCESS', message: 'Missing required field accepted' }), refusal)
        const wrongType = await owner.wrongType().then(() => ({ code: 'UNEXPECTED_SUCCESS', message: 'Wrong scalar type accepted' }), refusal)
        const extra = await owner.extra().then(() => ({ code: 'UNEXPECTED_SUCCESS', message: 'Undeclared field accepted' }), refusal)
        return { nullable, missing, wrongType, extra, privateProvider, unchangedMessage: owner.message }
      } finally {
        owner.__tachyonDispose()
      }
    })
  }

  /** @param {ModelDecodingProof} proof @returns {void} */
  static #checkDecoding(proof) {
    assert.equal(proof.privateProvider, true, 'both spellings of the private typed Client are absent')
    assert.deepEqual(proof.nullable, { message: null })
    assert.equal(proof.missing.code, 'HTTP_RESPONSE')
    assert.equal(proof.wrongType.code, 'HTTP_RESPONSE')
    assert.equal(proof.extra.code, 'HTTP_RESPONSE')
    assert.equal(proof.unchangedMessage, '', 'failed response decoding cannot publish Service state')
  }

  /** @param {import('playwright').Page} page @returns {Promise<ModelLifetimeProof>} */
  static async #lifetime(page) {
    const requested = page.waitForRequest(request => new URL(request.url()).pathname === '/api/greeting/slow')
    const pending = DotnetTaskModelBrowserGate.#start(page).catch(error => ({ code: 'PROBE_FAILED', message: String(error).slice(0, 2_000) }))
    try {
      const request = await requested
      assert.equal(request.method(), 'GET')
      const failed = page.waitForEvent('requestfailed', { predicate: request => new URL(request.url()).pathname === '/api/greeting/slow' }).catch(() => null)
      const capturedRefusal = await DotnetTaskModelBrowserGate.#dispose(page)
      const [cancelled, aborted, headers] = await Promise.all([pending, failed, request.allHeaders()])
      assert.ok(aborted, 'actual child HTTP request must abort when its Service is disposed')
      assert.equal(headers.cookie ?? null, null)
      assert.equal(headers.authorization ?? null, null)
      assert.equal(cancelled.code, 'CANCELLED')
      assert.equal(capturedRefusal.code, 'CANCELLED')
      const replacement = await DotnetTaskModelBrowserGate.#replacement(page)
      assert.deepEqual(replacement, { message: 'actual registered Yon answer' })
      const retiredChanges = await DotnetTaskModelBrowserGate.#retiredChanges(page)
      assert.equal(retiredChanges, 0)
      return { cancelled, capturedRefusal, replacement, retiredChanges }
    } finally {
      await DotnetTaskModelBrowserGate.#cleanup(page)
    }
  }

  /** @param {import('playwright').Page} page @returns {Promise<ModelRefusal>} */
  static #start(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/GreetingService.js'
      /** @type {ModelServiceModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      const state = { owner, captured: owner.ask, changes: 0, disposed: false }
      owner.__tachyonStoredChanged = () => { state.changes++ }
      Reflect.set(globalThis, '__taskModelOwner', state)
      try {
        await owner.slow()
        return { code: 'UNEXPECTED_SUCCESS', message: 'Disposed child request succeeded' }
      } catch (error) {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error).slice(0, 2_000), ...(typeof code === 'string' ? { code } : {}) }
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<ModelRefusal>} */
  static #dispose(page) {
    return page.evaluate(async () => {
      /** @type {ModelOwnerState} */
      const state = Reflect.get(globalThis, '__taskModelOwner')
      state.disposed = true
      state.owner.__tachyonDispose()
      try {
        await state.captured()
        return { code: 'UNEXPECTED_SUCCESS', message: 'Captured disposed Service call succeeded' }
      } catch (error) {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error).slice(0, 2_000), ...(typeof code === 'string' ? { code } : {}) }
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<RegisteredGreeting>} */
  static #replacement(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/GreetingService.js'
      /** @type {ModelServiceModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      try { return await owner.ask() } finally { owner.__tachyonDispose() }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<number>} */
  static #retiredChanges(page) {
    return page.evaluate(async () => {
      await new Promise(resolve => setTimeout(resolve, 650))
      /** @type {ModelOwnerState} */
      const state = Reflect.get(globalThis, '__taskModelOwner')
      return state.changes
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static #cleanup(page) {
    return page.evaluate(() => {
      /** @type {ModelOwnerState | undefined} */
      const state = Reflect.get(globalThis, '__taskModelOwner')
      try {
        if (state && !state.disposed) {
          state.disposed = true
          state.owner.__tachyonDispose()
        }
      } finally {
        Reflect.deleteProperty(globalThis, '__taskModelOwner')
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<ModelRequest[]>} */
  static #requests(page) {
    return page.evaluate(() => Reflect.get(globalThis, '__taskModelRequests'))
  }

  /** @param {ModelRequest[]} requests @returns {void} */
  static #checkRequests(requests) {
    const paths = ['/api/greeting', '/api/greeting/nullable', '/api/greeting/missing', '/api/greeting/wrong-type', '/api/greeting/extra', '/api/greeting/slow', '/api/greeting']
    assert.deepEqual(requests, paths.map(path => ({ path, method: 'GET', credentials: 'omit', authorization: false })))
  }
}

await DotnetTaskModelBrowserGate.run()

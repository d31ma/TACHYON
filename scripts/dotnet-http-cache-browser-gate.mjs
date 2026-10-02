#!/usr/bin/env node
import assert from 'node:assert/strict'
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'

/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpDiagnostics} HttpDiagnostics */
/** @typedef {{path: string, method: string, cache: RequestCache, credentials: RequestCredentials, authorization: boolean}} CacheRequest */
/** @typedef {{default: new() => {__tachyonDispose: () => void}}} CachedFacadeModule */

class DotnetHttpCacheBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(DotnetHttpCacheBrowserGate.#probe, 'ordinary C# cached HTTPClient with real public Yon, per-context answer cache and real no-cache request options; service-worker CacheStorage unqualified')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(page, origin, diagnostics) {
    await DotnetHttpCacheBrowserGate.#observeFetch(page)
    diagnostics.phase = 'ordinary-cached-view'
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
    await page.waitForFunction(() => document.querySelector('#cached-answer')?.textContent === 'ready')
    const headers = await DotnetHttpCacheBrowserGate.#firstRead(page)
    await DotnetHttpCacheBrowserGate.#readAgain(page)
    await DotnetHttpCacheBrowserGate.#write(page)
    await DotnetHttpCacheBrowserGate.#readAfterWrite(page)
    const requests = await DotnetHttpCacheBrowserGate.#requests(page)
    DotnetHttpCacheBrowserGate.#checkRequests(requests)
    const privateProvider = await DotnetHttpCacheBrowserGate.#privateProvider(page)
    assert.equal(privateProvider, true, 'Needs-injected client is absent from the public facade')
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.failedRequests, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    assert.deepEqual(diagnostics.requests, ['/api/cached-probe', '/api/cached-probe', '/api/cached-probe'])
    diagnostics.phase = 'complete'
    return { requests, responseHeaders: headers, privateProvider, requestRevalidationQualified: true, serviceWorkerQualified: false, responseEligibleForServiceWorker: !headers.vary && !/\b(?:private|no-store)\b/i.test(headers['cache-control'] ?? '') }
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #observeFetch(page) {
    await page.addInitScript(() => {
      const original = globalThis.fetch.bind(globalThis)
      /** @type {CacheRequest[]} */
      const requests = []
      Reflect.set(globalThis, '__dotnetCacheRequests', requests)
      globalThis.fetch = (input, options) => {
        const source = input instanceof Request ? input : undefined
        const url = new URL(source?.url ?? String(input), location.href)
        if (url.pathname === '/api/cached-probe' && requests.length < 8) {
          const headers = new Headers(options?.headers ?? source?.headers)
          requests.push({ path: url.pathname, method: options?.method ?? source?.method ?? 'GET', cache: options?.cache ?? source?.cache ?? 'default', credentials: options?.credentials ?? source?.credentials ?? 'same-origin', authorization: headers.has('authorization') })
        }
        return original(input, options)
      }
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<Record<string, string>>} */
  static async #firstRead(page) {
    const responsePromise = page.waitForResponse(response => new URL(response.url()).pathname === '/api/cached-probe')
    await page.locator('#cached-read').click()
    const response = await responsePromise
    assert.equal(response.status(), 200)
    assert.equal((await response.request().allHeaders()).cookie ?? null, null)
    assert.deepEqual(await response.json(), { message: 'actual public Yon answer' })
    const headers = await response.allHeaders()
    assert.match(headers['cache-control'] ?? '', /\bpublic\b/i)
    assert.doesNotMatch(headers['cache-control'] ?? '', /\b(?:private|no-store)\b/i)
    await page.waitForFunction(() => document.querySelector('#cached-read-count')?.textContent === '1')
    assert.equal((await DotnetHttpCacheBrowserGate.#requests(page)).length, 1)
    return headers
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #readAgain(page) {
    await page.locator('#cached-read').click()
    await page.waitForFunction(() => document.querySelector('#cached-read-count')?.textContent === '2')
    assert.equal(await page.locator('#cached-answer').textContent(), 'actual public Yon answer')
    assert.equal((await DotnetHttpCacheBrowserGate.#requests(page)).length, 1, 'second read answers without calling fetch')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #write(page) {
    const responsePromise = page.waitForResponse(response => new URL(response.url()).pathname === '/api/cached-probe' && response.request().method() === 'POST')
    await page.locator('#cached-save').click()
    const response = await responsePromise
    assert.equal(response.status(), 200)
    assert.deepEqual(await response.json(), { message: 'actual Yon write' })
    await page.waitForFunction(() => document.querySelector('#cached-save-count')?.textContent === '1')
    assert.equal(await page.locator('#cached-answer').textContent(), 'actual Yon write')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async #readAfterWrite(page) {
    const responsePromise = page.waitForResponse(response => new URL(response.url()).pathname === '/api/cached-probe' && response.request().method() === 'GET')
    await page.locator('#cached-read').click()
    const response = await responsePromise
    assert.equal(response.status(), 200)
    assert.deepEqual(await response.json(), { message: 'actual public Yon answer' })
    await page.waitForFunction(() => document.querySelector('#cached-read-count')?.textContent === '3')
    assert.equal(await page.locator('#cached-answer').textContent(), 'actual public Yon answer')
  }

  /** @param {import('playwright').Page} page @returns {Promise<CacheRequest[]>} */
  static async #requests(page) {
    return page.evaluate(() => Reflect.get(globalThis, '__dotnetCacheRequests'))
  }

  /** @param {CacheRequest[]} requests @returns {void} */
  static #checkRequests(requests) {
    assert.deepEqual(requests, [
      { path: '/api/cached-probe', method: 'GET', cache: 'default', credentials: 'omit', authorization: false },
      { path: '/api/cached-probe', method: 'POST', cache: 'no-store', credentials: 'omit', authorization: false },
      { path: '/api/cached-probe', method: 'GET', cache: 'no-cache', credentials: 'omit', authorization: false },
    ])
  }

  /** @param {import('playwright').Page} page @returns {Promise<boolean>} */
  static async #privateProvider(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/CachedHttpService.js'
      /** @type {CachedFacadeModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      try {
        return !Reflect.has(owner, 'cachedProbeClient')
      } finally {
        owner.__tachyonDispose()
      }
    })
  }
}

await DotnetHttpCacheBrowserGate.run()

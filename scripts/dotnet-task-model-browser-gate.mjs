#!/usr/bin/env node
import assert from 'node:assert/strict'
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'
import { DotnetTaskModelOwnerProof } from './dotnet-task-model-owner-proof.mjs'
import { DotnetTaskModelRequests } from './dotnet-task-model-requests.mjs'

/** @typedef {{message: string}} RegisteredGreeting */
/** @typedef {{message: string | null}} RegisteredNullableGreeting */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpRefusal} ModelRefusal */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpDiagnostics} HttpDiagnostics */
/** @typedef {{path: string, method: string, credentials: RequestCredentials, authorization: boolean}} ModelRequest */
/** @typedef {{message: string, ask: () => Promise<RegisteredGreeting>, nullable: () => Promise<RegisteredNullableGreeting>, missing: () => Promise<RegisteredGreeting>, wrongType: () => Promise<RegisteredGreeting>, extra: () => Promise<RegisteredGreeting>, slow: () => Promise<RegisteredGreeting>, __tachyonDispose: () => void, __tachyonStoredChanged?: () => void}} ModelOwner */
/** @typedef {{default: new() => ModelOwner}} ModelServiceModule */
/** @typedef {{nullable: RegisteredNullableGreeting, missing: ModelRefusal, wrongType: ModelRefusal, extra: ModelRefusal, privateProvider: boolean, unchangedMessage: string}} ModelDecodingProof */

class DotnetTaskModelBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(DotnetTaskModelBrowserGate.#probe, 'ordinary C# Task of registered closed scalar Greeting/NullableGreeting records through genuine Yon and activation-owned HTTPClient; arbitrary Task DTO and service-worker cache unqualified')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(page, origin, diagnostics) {
    await DotnetTaskModelRequests.observe(page, '/api/greeting', 12)
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
    const lifetime = await DotnetTaskModelOwnerProof.run(page, { modulePath: '/.tachyon/services/GreetingService.js', slowPath: '/api/greeting/slow' })
    assert.deepEqual(lifetime.replacement, { message: 'actual registered Yon answer' })
    const requests = await DotnetTaskModelRequests.read(page)
    DotnetTaskModelBrowserGate.#checkRequests(requests)
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    assert.equal(diagnostics.failedRequests.length, 1, 'only the deliberately cancelled child request fails')
    assert.equal(diagnostics.failedRequests[0]?.path, '/api/greeting/slow')
    assert.deepEqual(diagnostics.requests, requests.map(request => request.path))
    diagnostics.phase = 'complete'
    return { decoding, lifetime, requests, registeredScalarTaskModelsQualified: true, arbitraryTaskDtoQualified: false, serviceWorkerQualified: false, browserHostReplaced: false }
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

  /** @param {ModelRequest[]} requests @returns {void} */
  static #checkRequests(requests) {
    const paths = ['/api/greeting', '/api/greeting/nullable', '/api/greeting/missing', '/api/greeting/wrong-type', '/api/greeting/extra', '/api/greeting/slow', '/api/greeting']
    assert.deepEqual(requests, paths.map(path => ({ path, method: 'GET', credentials: 'omit', authorization: false })))
  }
}

await DotnetTaskModelBrowserGate.run()

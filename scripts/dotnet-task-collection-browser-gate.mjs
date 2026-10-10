#!/usr/bin/env node
import assert from 'node:assert/strict'
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'
import { DotnetTaskModelOwnerProof } from './dotnet-task-model-owner-proof.mjs'
import { DotnetTaskModelRequests } from './dotnet-task-model-requests.mjs'

/** @typedef {{message: string, note: string | null}} MessageItem */
/** @typedef {{items: MessageItem[], optionalItems: (MessageItem | null)[] | null}} MessagePage */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpRefusal} CollectionRefusal */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpDiagnostics} HttpDiagnostics */
/** @typedef {import('./dotnet-task-model-requests.mjs').TaskModelRequest} CollectionRequest */
/** @typedef {'missing' | 'wrongArray' | 'missingMember' | 'wrongMember' | 'extraMember' | 'nullMember' | 'wrongNullableLeaf'} CollectionInvalidMethod */
/** @typedef {{first: string, ask: () => Promise<MessagePage>, empty: () => Promise<MessagePage>, nullable: () => Promise<MessagePage>, nullList: () => Promise<MessagePage>, nullableLeaf: () => Promise<MessagePage>, wrongNullableLeaf: () => Promise<MessagePage>, missing: () => Promise<MessagePage>, wrongArray: () => Promise<MessagePage>, missingMember: () => Promise<MessagePage>, wrongMember: () => Promise<MessagePage>, extraMember: () => Promise<MessagePage>, nullMember: () => Promise<MessagePage>, slow: () => Promise<MessagePage>, __tachyonDispose: () => void}} CollectionOwner */
/** @typedef {{default: new() => CollectionOwner}} CollectionServiceModule */
/** @typedef {{empty: MessagePage, nullable: MessagePage, nullList: MessagePage, nullableLeaf: MessagePage, refused: {method: CollectionInvalidMethod, error: CollectionRefusal}[], privateProvider: boolean, unchangedFirst: string}} CollectionDecodingProof */

class DotnetTaskCollectionBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(DotnetTaskCollectionBrowserGate.#probe, 'C# Task registered closed record with one-level scalar-record list fields through real Yon; optional/nullable list semantics, malformed member refusal and captured child disposal; top-level collections, recursive/nested collections, native async and service-worker cache unqualified')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(page, origin, diagnostics) {
    await DotnetTaskModelRequests.observe(page, '/api/collections', 16)
    diagnostics.phase = 'ordered-registered-collection-view'
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
    await page.waitForFunction(() => document.querySelector('#collection-answer')?.textContent === '')
    const response = page.waitForResponse(answer => new URL(answer.url()).pathname === '/api/collections')
    await page.locator('#collection-ask').click()
    const answer = await response
    assert.equal(answer.status(), 200)
    assert.deepEqual(await answer.json(), { items: [{ message: 'first' }, { message: 'second' }] })
    const headers = await answer.request().allHeaders()
    assert.equal(headers.cookie ?? null, null)
    assert.equal(headers.authorization ?? null, null)
    await page.waitForFunction(() => document.querySelector('#collection-answer')?.textContent === 'first')
    await page.locator('#collection-published').waitFor({ state: 'visible' })
    diagnostics.phase = 'closed-collection-member-response-controls'
    const decoding = await DotnetTaskCollectionBrowserGate.#decoding(page)
    DotnetTaskCollectionBrowserGate.#checkDecoding(decoding)
    diagnostics.phase = 'collection-child-disposal-before-publication'
    const lifetime = await DotnetTaskModelOwnerProof.run(page, { modulePath: '/.tachyon/services/CollectionService.js', slowPath: '/api/collections/slow' })
    assert.deepEqual(lifetime.replacement, { items: [{ message: 'first', note: null }, { message: 'second', note: null }], optionalItems: null })
    const requests = await DotnetTaskModelRequests.read(page)
    DotnetTaskCollectionBrowserGate.#checkRequests(requests)
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    assert.equal(diagnostics.failedRequests.length, 1, 'only the deliberately cancelled collection request fails')
    assert.equal(diagnostics.failedRequests[0]?.path, '/api/collections/slow')
    assert.deepEqual(diagnostics.requests, requests.map(request => request.path))
    diagnostics.phase = 'complete'
    return { decoding, lifetime, requests, registeredCollectionFieldsQualified: true, topLevelTaskCollectionQualified: false, recursiveModelQualified: false, serviceWorkerQualified: false, browserHostReplaced: false }
  }

  /** @param {import('playwright').Page} page @returns {Promise<CollectionDecodingProof>} */
  static async #decoding(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/CollectionService.js'
      /** @type {CollectionServiceModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      /** @type {CollectionInvalidMethod[]} */
      const methods = ['missing', 'wrongArray', 'missingMember', 'wrongMember', 'extraMember', 'nullMember', 'wrongNullableLeaf']
      /** @type {{method: CollectionInvalidMethod, error: CollectionRefusal}[]} */
      const refused = []
      /** @param {unknown} error @returns {CollectionRefusal} */
      const refusal = error => {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error).slice(0, 2_000), ...(typeof code === 'string' ? { code } : {}) }
      }
      try {
        const privateProvider = !Reflect.has(owner, '_collectionClient') && !Reflect.has(owner, 'collectionClient')
        const empty = await owner.empty()
        const nullable = await owner.nullable()
        const nullList = await owner.nullList()
        const nullableLeaf = await owner.nullableLeaf()
        for (const method of methods) {
          const error = await owner[method]().then(() => ({ code: 'UNEXPECTED_SUCCESS', message: 'Malformed collection accepted' }), refusal)
          refused.push({ method, error })
        }
        return { empty, nullable, nullList, nullableLeaf, refused, privateProvider, unchangedFirst: owner.first }
      } finally {
        owner.__tachyonDispose()
      }
    })
  }

  /** @param {CollectionDecodingProof} proof @returns {void} */
  static #checkDecoding(proof) {
    assert.equal(proof.privateProvider, true)
    assert.deepEqual(proof.empty, { items: [], optionalItems: null })
    assert.deepEqual(proof.nullable, { items: [], optionalItems: [null, { message: 'nullable item', note: null }] })
    assert.deepEqual(proof.nullList, { items: [], optionalItems: null })
    assert.deepEqual(proof.nullableLeaf, { items: [{ message: 'absent note', note: null }, { message: 'null note', note: null }, { message: 'text note', note: 'present' }], optionalItems: null })
    assert.deepEqual(proof.refused.map(result => result.method), ['missing', 'wrongArray', 'missingMember', 'wrongMember', 'extraMember', 'nullMember', 'wrongNullableLeaf'])
    assert.deepEqual(proof.refused.map(result => result.error.code), Array(7).fill('HTTP_RESPONSE'))
    assert.equal(proof.unchangedFirst, '', 'no malformed collection can publish Service state')
  }

  /** @param {CollectionRequest[]} requests @returns {void} */
  static #checkRequests(requests) {
    const suffixes = ['', '/empty', '/nullable', '/null-list', '/nullable-leaf', '/missing', '/wrong-array', '/missing-member', '/wrong-member', '/extra-member', '/null-member', '/wrong-nullable-leaf', '/slow', '']
    assert.deepEqual(requests, suffixes.map(suffix => ({ path: `/api/collections${suffix}`, method: 'GET', credentials: 'omit', authorization: false })))
  }
}

await DotnetTaskCollectionBrowserGate.run()

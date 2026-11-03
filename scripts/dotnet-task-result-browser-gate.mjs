#!/usr/bin/env node
import assert from 'node:assert/strict'
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'
import { DotnetTaskModelRequests } from './dotnet-task-model-requests.mjs'

/** @typedef {{name: string}} ShelfItem */
/** @typedef {{label: string, detail: {note: string, count: number}}} Shelf */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpRefusal} ResultRefusal */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpDiagnostics} HttpDiagnostics */
/** @typedef {import('./dotnet-task-model-requests.mjs').TaskModelRequest} ResultRequest */
/** @typedef {'wrongItems' | 'wrongCount' | 'wrongShelf'} ResultInvalidMethod */
/** @typedef {{summary: string, items: () => Promise<ShelfItem[]>, array: () => Promise<ShelfItem[]>, count: () => Promise<number>, label: () => Promise<string>, ready: () => Promise<boolean>, ratio: () => Promise<number>, numbers: () => Promise<number[]>, shelf: () => Promise<Shelf>, first: () => Promise<ShelfItem>, wrongItems: () => Promise<ShelfItem[]>, wrongCount: () => Promise<number>, wrongShelf: () => Promise<Shelf>, __tachyonDispose: () => void}} ResultOwner */
/** @typedef {{default: new() => ResultOwner}} ResultServiceModule */
/** @typedef {{items: ShelfItem[], array: ShelfItem[], count: number, label: string, ready: boolean, ratio: number, numbers: number[], first: ShelfItem, refused: {method: ResultInvalidMethod, error: ResultRefusal}[], unchangedSummary: string}} ResultProof */

class DotnetTaskResultBrowserGate {
  static #modulePath = '/.tachyon/services/ResultService.js'
  /** @type {ResultInvalidMethod[]} */
  static #invalid = ['wrongItems', 'wrongCount', 'wrongShelf']

  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(DotnetTaskResultBrowserGate.#probe, 'C# browser HTTPClient and Service results of Task and ValueTask: lists and arrays of registered records and plain values, plain values and one nested record through real Yon, with HTTP_RESPONSE for wrong shapes')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {HttpDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(page, origin, diagnostics) {
    await DotnetTaskModelRequests.observe(page, '/api/results', 16)
    diagnostics.phase = 'nested-record-view'
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
    await page.waitForFunction(() => document.querySelector('#result-summary')?.textContent === '')
    const response = page.waitForResponse(answer => new URL(answer.url()).pathname === '/api/results/shelf')
    await page.locator('#result-shelf').click()
    const answer = await response
    assert.equal(answer.status(), 200)
    assert.deepEqual(await answer.json(), { label: 'Yon shelf', detail: { note: 'nested note', count: 3 } })
    await page.waitForFunction(() => document.querySelector('#result-summary')?.textContent === 'Yon shelf: nested note')
    diagnostics.phase = 'list-value-and-value-task-results'
    const proof = await DotnetTaskResultBrowserGate.#results(page)
    DotnetTaskResultBrowserGate.#check(proof)
    const requests = await DotnetTaskModelRequests.read(page)
    DotnetTaskResultBrowserGate.#checkRequests(requests)
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    assert.deepEqual(diagnostics.failedRequests, [])
    assert.deepEqual(diagnostics.requests, requests.map(request => request.path))
    diagnostics.phase = 'complete'
    return { proof, requests, topLevelListsQualified: true, plainValuesQualified: true, nestedRecordQualified: true, valueTaskQualified: true, mapsQualified: false, nestedListsQualified: false }
  }

  /** @param {import('playwright').Page} page @returns {Promise<ResultProof>} */
  static async #results(page) {
    return page.evaluate(async ({ modulePath, invalid }) => {
      /** @type {ResultServiceModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      /** @param {unknown} error @returns {ResultRefusal} */
      const refusal = error => {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error).slice(0, 2_000), ...(typeof code === 'string' ? { code } : {}) }
      }
      try {
        const items = await owner.items()
        const array = await owner.array()
        const count = await owner.count()
        const label = await owner.label()
        const ready = await owner.ready()
        const ratio = await owner.ratio()
        const numbers = await owner.numbers()
        const first = await owner.first()
        /** @type {{method: ResultInvalidMethod, error: ResultRefusal}[]} */
        const refused = []
        for (const method of invalid) {
          const error = await owner[method]().then(() => ({ code: 'UNEXPECTED_SUCCESS', message: 'Wrong result shape accepted' }), refusal)
          refused.push({ method, error })
        }
        return { items, array, count, label, ready, ratio, numbers, first, refused, unchangedSummary: owner.summary }
      } finally {
        owner.__tachyonDispose()
      }
    }, { modulePath: DotnetTaskResultBrowserGate.#modulePath, invalid: DotnetTaskResultBrowserGate.#invalid })
  }

  /** @param {ResultProof} proof @returns {void} */
  static #check(proof) {
    assert.deepEqual(proof.items, [{ name: 'first item' }, { name: 'second item' }])
    assert.deepEqual(proof.array, [{ name: 'array item' }])
    assert.equal(proof.count, 42)
    assert.equal(proof.label, 'Yon label')
    assert.equal(proof.ready, true)
    assert.equal(proof.ratio, 0.25)
    assert.deepEqual(proof.numbers, [5, 8, 13])
    assert.deepEqual(proof.first, { name: 'value task item' })
    assert.deepEqual(proof.refused.map(result => result.method), DotnetTaskResultBrowserGate.#invalid)
    assert.deepEqual(proof.refused.map(result => result.error.code), Array(DotnetTaskResultBrowserGate.#invalid.length).fill('HTTP_RESPONSE'))
    assert.equal(proof.unchangedSummary, '', 'a wrong nested record cannot publish Service state')
  }

  /** @param {ResultRequest[]} requests @returns {void} */
  static #checkRequests(requests) {
    const suffixes = ['/shelf', '/items', '/array', '/count', '/label', '/ready', '/ratio', '/numbers', '/first', '/wrong-items', '/wrong-count', '/wrong-shelf']
    assert.deepEqual(requests, suffixes.map(suffix => ({ path: `/api/results${suffix}`, method: 'GET', credentials: 'omit', authorization: false })))
  }
}

await DotnetTaskResultBrowserGate.run()

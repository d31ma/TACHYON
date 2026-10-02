#!/usr/bin/env node
import assert from 'node:assert/strict'
import { DotnetHttpBrowserGate } from './dotnet-http-browser-gate.mjs'

/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpDiagnostics} TutorialDiagnostics */
/** @typedef {{__tachyonDispose: () => void}} CounterOwner */
/** @typedef {{default: new() => CounterOwner}} CounterModule */

class FirstAppCsharpBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    await DotnetHttpBrowserGate.run(FirstAppCsharpBrowserGate.#probe, 'unlisted cumulative first-app C# Step4 candidate: exact published preceding JavaScript Yon endpoint, registered scalar Task<Models.Greeting>, private Needs client, real count/ask/message UI; Step5 SDK proof is separate; arbitrary Task DTO and service-worker cache unqualified')
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {TutorialDiagnostics} diagnostics @returns {Promise<Record<string, unknown>>} */
  static async #probe(page, origin, diagnostics) {
    diagnostics.phase = 'cumulative-csharp-counter'
    await page.goto(`${origin}/hello/`, { waitUntil: 'networkidle', timeout: 15_000 })
    await page.waitForFunction(() => document.querySelector('section p')?.textContent?.trim() === 'You clicked 0 times.')
    await page.getByRole('button', { name: 'Click me', exact: true }).click()
    await page.waitForFunction(() => document.querySelector('section p')?.textContent?.trim() === 'You clicked 1 times.')
    diagnostics.phase = 'published-javascript-yon-answer'
    const pending = page.waitForResponse(response => new URL(response.url()).pathname === '/api/greeting')
    await page.getByRole('button', { name: 'Ask the server', exact: true }).click()
    const answer = await pending
    assert.equal(answer.status(), 200)
    assert.equal(answer.request().method(), 'GET')
    const greeting = await answer.json()
    assert.deepEqual(greeting, { message: 'Hello from Yon' })
    const headers = await answer.request().allHeaders()
    assert.equal(headers.cookie ?? null, null, 'the runner seeds a same-origin cookie which must not reach Yon')
    assert.equal(headers.authorization ?? null, null)
    await page.waitForFunction(() => document.querySelectorAll('section p')[1]?.textContent?.trim() === 'Hello from Yon')
    await page.getByRole('button', { name: 'Click me', exact: true }).click()
    await page.waitForFunction(() => document.querySelector('section p')?.textContent?.trim() === 'You clicked 2 times.')
    diagnostics.phase = 'private-csharp-client-facade'
    const privateClient = await FirstAppCsharpBrowserGate.#privateClient(page)
    assert.equal(privateClient, true, 'neither spelling of the private Needs provider is a public facade field')
    assert.deepEqual(diagnostics.requests, ['/api/greeting'])
    assert.deepEqual(diagnostics.errors, [])
    assert.deepEqual(diagnostics.failedRequests, [])
    assert.deepEqual(diagnostics.externalRequests, [])
    diagnostics.phase = 'complete'
    return { counts: [0, 1, 2], greeting, privateClient, publishedJavascriptYon: true, registeredScalarTaskModelQualified: true, stepFiveSdkQualified: false, arbitraryTaskDtoQualified: false, serviceWorkerQualified: false, browserHostReplaced: false }
  }

  /** @param {import('playwright').Page} page @returns {Promise<boolean>} */
  static #privateClient(page) {
    return page.evaluate(async () => {
      const modulePath = '/.tachyon/services/CounterService.js'
      /** @type {CounterModule} */
      const Service = await import(modulePath)
      const counter = new Service.default()
      try { return !Reflect.has(counter, '_greetingClient') && !Reflect.has(counter, 'greetingClient') }
      finally { counter.__tachyonDispose() }
    })
  }
}

await FirstAppCsharpBrowserGate.run()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {{key: string, calls: number}} TaskAnswer */
/** @typedef {{calls: number, last: string, load: (key: string, delay: number) => Promise<TaskAnswer>, forget: () => Promise<null>, failure: () => Promise<unknown>, __tachyonDispose: () => void, __tachyonStoredChanged?: () => void}} TaskFacade */
/** @typedef {{service: TaskFacade, disposed: boolean}} OwnedFacade */
/** @typedef {{message: string, code?: string}} Refusal */
/** @typedef {{phase: string, sourceCommit: string, fixtureSourceCommit: string, compilerSha256: string, errors: string[], failedRequests: {path: string, error?: string}[], externalRequests: string[], chromiumVersion?: string}} Diagnostics */
/** @typedef {{root: string, binary: string, diagnostics: Diagnostics}} Fixture */
/** @typedef {{first: TaskAnswer, second: TaskAnswer, order: string[], timerTicks: number, elapsedMilliseconds: number, voidResult: null, failure: Refusal, disposal: Refusal, retiredCall: Refusal, replacement: TaskAnswer, retiredChanges: number, oldGetterRefused: boolean}} ContinuationProof */

class DotnetTaskBrowserGate {
  /** @returns {Promise<void>} */
  static async run() {
    const fixture = await DotnetTaskBrowserGate.#read()
    await DotnetTaskBrowserGate.#execute(fixture)
  }

  /** @param {string} name @returns {string} */
  static #required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} must identify the explicit retained fixture`)
    return value
  }

  /** @returns {Promise<Fixture>} */
  static async #read() {
    const root = path.resolve(DotnetTaskBrowserGate.#required('TAC_DOTNET_TASK_FIXTURE_OUT'))
    const binary = path.resolve(DotnetTaskBrowserGate.#required('TAC_BIN'))
    const sourceCommit = DotnetTaskBrowserGate.#required('TAC_DOTNET_TASK_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    return { root, binary, diagnostics: { phase: 'starting-preview', sourceCommit, compilerSha256, fixtureSourceCommit: process.env.TAC_DOTNET_TASK_FIXTURE_SOURCE_COMMIT ?? 'not supplied', errors: [], failedRequests: [], externalRequests: [] } }
  }

  /** @param {Fixture} fixture @returns {Promise<void>} */
  static async #execute(fixture) {
    const started = performance.now()
    try {
      const proof = await withServer(fixture.binary, ['preview', fixture.root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return DotnetTaskBrowserGate.#probe(origin, fixture.diagnostics)
      })
      await DotnetTaskBrowserGate.#receipt(fixture, { status: 'pass', proof, elapsedMilliseconds: performance.now() - started })
      console.log(JSON.stringify(proof))
    } catch (error) {
      await DotnetTaskBrowserGate.#receipt(fixture, { status: 'fail', error: String(error).slice(0, 2_000), elapsedMilliseconds: performance.now() - started })
      throw error
    }
  }

  /** @param {Fixture} fixture @param {Record<string, unknown>} result @returns {Promise<void>} */
  static async #receipt(fixture, result) {
    await writeFile(path.join(fixture.root, 'dotnet-task-browser.json'), JSON.stringify({ ...fixture.diagnostics, ...result, scope: 'ordinary production C#/.NET Wasm Task facade and Chromium continuation settlement', adapterReplaced: false, defaultHttpInstalled: false, asyncLifecycleQualified: false, taskDtoQualified: false, sameUuidReuse: 'not exposed by public facade; separate .NET ownership fixture', sameTargetRemount: 'separate bridge unit; this browser gate creates a genuine replacement instance', browsersQualified: result.status === 'pass' ? ['Chromium'] : [] }, null, 2) + '\n')
  }

  /** @param {string} origin @param {Diagnostics} diagnostics @returns {Promise<ContinuationProof>} */
  static async #probe(origin, diagnostics) {
    const browser = await chromium.launch({ timeout: 10_000 })
    diagnostics.chromiumVersion = browser.version()
    try {
      const page = await browser.newPage()
      DotnetTaskBrowserGate.#observe(page, origin, diagnostics)
      diagnostics.phase = 'ordinary-view-bindings'
      await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
      await page.waitForFunction(() => document.querySelector('#task-ready')?.textContent === 'ready', undefined, { timeout: 10_000 })
      await page.locator('#task-load').click()
      await page.waitForFunction(() => document.querySelector('#task-ready')?.textContent === 'view', undefined, { timeout: 5_000 })
      assert.equal(await page.locator('#task-calls').textContent(), '1')
      await page.locator('#task-forget').click()
      await page.waitForFunction(() => document.querySelector('#task-ready')?.textContent === 'void', undefined, { timeout: 5_000 })
      assert.equal(await page.locator('#task-calls').textContent(), '2')
      diagnostics.phase = 'actual-task-owners'
      const proof = await DotnetTaskBrowserGate.#continuations(page)
      DotnetTaskBrowserGate.#check(proof)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      return proof
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} origin @param {Diagnostics} diagnostics @returns {void} */
  static #observe(page, origin, diagnostics) {
    page.on('pageerror', error => DotnetTaskBrowserGate.#record(diagnostics.errors, error.message.slice(0, 2_000)))
    page.on('requestfailed', request => DotnetTaskBrowserGate.#record(diagnostics.failedRequests, { path: new URL(request.url()).pathname.slice(0, 240), error: request.failure()?.errorText }))
    page.on('request', request => {
      const url = new URL(request.url())
      if (url.protocol.startsWith('http') && url.origin !== origin) DotnetTaskBrowserGate.#record(diagnostics.externalRequests, url.pathname.slice(0, 240))
    })
    page.on('response', response => {
      if (response.status() >= 400) DotnetTaskBrowserGate.#record(diagnostics.errors, `HTTP ${response.status()} ${new URL(response.url()).pathname.slice(0, 240)}`)
    })
  }

  /** @template T @param {T[]} entries @param {T} value @returns {void} */
  static #record(entries, value) {
    if (entries.length < 20) entries.push(value)
  }

  /** @param {import('playwright').Page} page @returns {Promise<ContinuationProof>} */
  static async #continuations(page) {
    return page.evaluate(async () => {
      /** @type {new (properties: Record<string, unknown>) => TaskFacade} */
      const Service = (await import('/.tachyon/services/DelayTaskService.js')).default
      /** @type {OwnedFacade[]} */
      const owned = []
      /** @returns {OwnedFacade} */
      const create = () => {
        const entry = { service: new Service({}), disposed: false }
        owned.push(entry)
        return entry
      }
      /** @param {OwnedFacade} entry @returns {void} */
      const dispose = entry => {
        if (entry.disposed) return
        entry.disposed = true
        entry.service.__tachyonDispose()
      }
      /** @param {() => Promise<unknown>} work @returns {Promise<Refusal>} */
      const refusal = async work => {
        try { await work() } catch (error) {
          /** @type {Error & {code?: string}} */
          const failure = error
          return { message: String(failure).slice(0, 512), code: failure.code }
        }
        throw new Error('Disposed or failing Task was accepted')
      }
      /** @type {ReturnType<typeof setInterval> | undefined} */
      let timer
      try {
        const first = create()
        const second = create()
        /** @type {string[]} */
        const order = []
        /** @type {number} */
        let ticks = 0
        /** @type {boolean} */
        let settled = false
        const started = performance.now()
        timer = setInterval(() => { if (!settled) ticks++ }, 10)
        const waiting = first.service.load('first', 250).then(value => { settled = true; order.push('first'); return value })
        if (!(waiting instanceof Promise)) throw new Error('Task facade did not return a Promise')
        if (second.service.calls !== 0) throw new Error('Activation state was shared')
        const other = await second.service.load('second', 20)
        order.push('second')
        if (settled) throw new Error('Second owner was blocked behind the first Task')
        const answer = await waiting
        clearInterval(timer)
        timer = undefined
        const elapsedMilliseconds = performance.now() - started
        if (first.service.calls !== 1 || second.service.calls !== 1) throw new Error('Task invocation was replayed')
        const voidResult = await second.service.forget()
        const failure = await refusal(() => second.service.failure())
        /** @type {number} */
        let retiredChanges = 0
        first.service.__tachyonStoredChanged = () => { retiredChanges++ }
        const retained = first.service.load
        const obsolete = first.service.load('obsolete', 300)
        dispose(first)
        const cancellation = await refusal(() => obsolete)
        const retiredCall = await refusal(() => retained('retired', 10))
        const replacement = create()
        const fresh = await replacement.service.load('replacement', 20)
        await new Promise(resolve => setTimeout(resolve, 350))
        if (replacement.service.calls !== 1 || replacement.service.last !== 'replacement') throw new Error('Retired continuation changed the replacement owner')
        /** @type {boolean} */
        let oldGetterRefused = false
        try { void first.service.calls } catch { oldGetterRefused = true }
        return { first: answer, second: other, order, timerTicks: ticks, elapsedMilliseconds, voidResult, failure, disposal: cancellation, retiredCall, replacement: fresh, retiredChanges, oldGetterRefused }
      } finally {
        clearInterval(timer)
        for (const entry of owned) dispose(entry)
      }
    })
  }

  /** @param {ContinuationProof} proof @returns {void} */
  static #check(proof) {
    assert.deepEqual(proof.first, { key: 'first', calls: 1 })
    assert.deepEqual(proof.second, { key: 'second', calls: 1 })
    assert.deepEqual(proof.order, ['second', 'first'])
    assert.ok(proof.timerTicks > 0, 'browser timer must progress during the real .NET Task wait')
    assert.equal(proof.voidResult, null)
    assert.match(proof.failure.message, /Web service task failed/)
    assert.ok(!proof.failure.message.includes('private fixture failure detail'))
    assert.equal(proof.disposal.code, 'CANCELLED')
    assert.equal(proof.retiredCall.code, 'CANCELLED')
    assert.deepEqual(proof.replacement, { key: 'replacement', calls: 1 })
    assert.equal(proof.retiredChanges, 0)
    assert.equal(proof.oldGetterRefused, true)
  }
}

await DotnetTaskBrowserGate.run()

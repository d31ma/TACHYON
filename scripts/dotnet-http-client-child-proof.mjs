import assert from 'node:assert/strict'

/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpAnswer} HttpAnswer */
/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpRefusal} HttpRefusal */
/** @typedef {{read: () => Promise<HttpAnswer>, slow: () => Promise<HttpAnswer>, workerRead: () => Promise<HttpAnswer>, __tachyonDispose: () => void, __tachyonStoredChanged?: () => void}} ChildClientOwner */
/** @typedef {{default: new() => ChildClientOwner}} ChildClientModule */
/** @typedef {{owner: ChildClientOwner, captured: () => Promise<HttpAnswer>, changes: number}} ChildClientState */
/** @typedef {{cancelled: HttpRefusal, capturedRefusal: HttpRefusal, retiredChanges: number, replacement: HttpAnswer}} ChildClientProof */

export class DotnetHttpClientChildProof {
  /** @type {import('playwright').Page} */
  #page

  /** @param {import('playwright').Page} page */
  constructor(page) {
    this.#page = page
  }

  /** @returns {Promise<ChildClientProof>} */
  async run() {
    return this.#exercise()
  }

  /** @returns {Promise<ChildClientProof>} */
  async #exercise() {
    const requested = this.#page.waitForRequest(request => new URL(request.url()).pathname === '/api/cached-probe/slow')
    const pending = this.#start().catch(error => ({ code: 'PROBE_FAILED', message: String(error).slice(0, 2_000) }))
    try {
      const request = await requested
      assert.equal(request.method(), 'GET')
      assert.equal((await request.allHeaders()).cookie ?? null, null)
      const aborted = this.#page.waitForEvent('requestfailed', { predicate: request => new URL(request.url()).pathname === '/api/cached-probe/slow' }).catch(() => null)
      const capturedRefusal = await this.#dispose()
      const [cancelled, failedRequest] = await Promise.all([pending, aborted])
      assert.ok(failedRequest, 'disposing the Service must abort its real pending private Client GET')
      assert.equal(cancelled.code, 'CANCELLED', 'disposing the Service cancels its actual private Client context')
      assert.equal(capturedRefusal.code, 'CANCELLED')
      const replacement = await this.#replacement()
      assert.deepEqual(replacement, { message: 'actual public Yon answer' })
      const retiredChanges = await this.#retiredChanges()
      assert.equal(retiredChanges, 0)
      return { cancelled, capturedRefusal, retiredChanges, replacement }
    } finally {
      await this.#cleanup()
    }
  }

  /** @returns {Promise<HttpRefusal>} */
  #start() {
    return this.#page.evaluate(async () => {
      const modulePath = '/.tachyon/services/CachedHttpService.js'
      /** @type {ChildClientModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      const state = { owner, captured: owner.read, changes: 0 }
      owner.__tachyonStoredChanged = () => { state.changes++ }
      Reflect.set(globalThis, '__dotnetChildOwner', state)
      try {
        await owner.slow()
        return { code: 'UNEXPECTED_SUCCESS', message: 'Retired private Client request succeeded' }
      } catch (error) {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error), ...(typeof code === 'string' ? { code } : {}) }
      }
    })
  }

  /** @returns {Promise<HttpRefusal>} */
  #dispose() {
    return this.#page.evaluate(async () => {
      /** @type {ChildClientState} */
      const state = Reflect.get(globalThis, '__dotnetChildOwner')
      state.owner.__tachyonDispose()
      try {
        await state.captured()
        return { code: 'UNEXPECTED_SUCCESS', message: 'Retired captured Service method succeeded' }
      } catch (error) {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error), ...(typeof code === 'string' ? { code } : {}) }
      }
    })
  }

  /** @returns {Promise<HttpAnswer>} */
  #replacement() {
    return this.#page.evaluate(async () => {
      const modulePath = '/.tachyon/services/CachedHttpService.js'
      /** @type {ChildClientModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      try { return await owner.workerRead() } finally { owner.__tachyonDispose() }
    })
  }

  /** @returns {Promise<number>} */
  #retiredChanges() {
    return this.#page.evaluate(async () => {
      await new Promise(resolve => setTimeout(resolve, 650))
      /** @type {ChildClientState} */
      const state = Reflect.get(globalThis, '__dotnetChildOwner')
      return state.changes
    })
  }

  /** @returns {Promise<void>} */
  #cleanup() {
    return this.#page.evaluate(() => {
      /** @type {ChildClientState | undefined} */
      const state = Reflect.get(globalThis, '__dotnetChildOwner')
      state?.owner.__tachyonDispose()
      Reflect.deleteProperty(globalThis, '__dotnetChildOwner')
    })
  }
}

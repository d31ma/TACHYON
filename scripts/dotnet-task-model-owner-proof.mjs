import assert from 'node:assert/strict'

/** @typedef {import('./dotnet-http-browser-gate.mjs').HttpRefusal} TaskRefusal */
/** @typedef {{modulePath: string, slowPath: string}} TaskOwnerProfile */
/** @typedef {{ask: () => Promise<unknown>, slow: () => Promise<unknown>, __tachyonDispose: () => void, __tachyonStoredChanged?: () => void}} TaskOwner */
/** @typedef {{default: new() => TaskOwner}} TaskOwnerModule */
/** @typedef {{owner: TaskOwner, captured: () => Promise<unknown>, changes: number, disposed: boolean}} TaskOwnerState */
/** @typedef {{cancelled: TaskRefusal, capturedRefusal: TaskRefusal, retiredChanges: number, replacement: unknown}} TaskOwnerProof */

export class DotnetTaskModelOwnerProof {
  /** @param {import('playwright').Page} page @param {TaskOwnerProfile} profile @returns {Promise<TaskOwnerProof>} */
  static async run(page, profile) {
    return DotnetTaskModelOwnerProof.#exercise(page, profile)
  }

  /** @param {import('playwright').Page} page @param {TaskOwnerProfile} profile @returns {Promise<TaskOwnerProof>} */
  static async #exercise(page, profile) {
    const requested = page.waitForRequest(request => new URL(request.url()).pathname === profile.slowPath)
    const pending = DotnetTaskModelOwnerProof.#start(page, profile).catch(error => ({ code: 'PROBE_FAILED', message: String(error).slice(0, 2_000) }))
    try {
      const request = await requested
      assert.equal(request.method(), 'GET')
      const failed = page.waitForEvent('requestfailed', { predicate: request => new URL(request.url()).pathname === profile.slowPath }).catch(() => null)
      const capturedRefusal = await DotnetTaskModelOwnerProof.#dispose(page)
      const [cancelled, aborted, headers] = await Promise.all([pending, failed, request.allHeaders()])
      assert.ok(aborted, 'actual child HTTP request must abort when its Service is disposed')
      assert.equal(headers.cookie ?? null, null)
      assert.equal(headers.authorization ?? null, null)
      assert.equal(cancelled.code, 'CANCELLED')
      assert.equal(capturedRefusal.code, 'CANCELLED')
      const replacement = await DotnetTaskModelOwnerProof.#replacement(page, profile)
      const retiredChanges = await DotnetTaskModelOwnerProof.#retiredChanges(page)
      assert.equal(retiredChanges, 0)
      return { cancelled, capturedRefusal, replacement, retiredChanges }
    } finally {
      await DotnetTaskModelOwnerProof.#cleanup(page)
    }
  }

  /** @param {import('playwright').Page} page @param {TaskOwnerProfile} profile @returns {Promise<TaskRefusal>} */
  static #start(page, profile) {
    return page.evaluate(async ({ modulePath }) => {
      /** @type {TaskOwnerModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      const state = { owner, captured: owner.ask, changes: 0, disposed: false }
      owner.__tachyonStoredChanged = () => { state.changes++ }
      Reflect.set(globalThis, '__taskDtoOwner', state)
      try {
        await owner.slow()
        return { code: 'UNEXPECTED_SUCCESS', message: 'Disposed child request succeeded' }
      } catch (error) {
        const code = error !== null && typeof error === 'object' ? Reflect.get(error, 'code') : undefined
        return { message: String(error).slice(0, 2_000), ...(typeof code === 'string' ? { code } : {}) }
      }
    }, profile)
  }

  /** @param {import('playwright').Page} page @returns {Promise<TaskRefusal>} */
  static #dispose(page) {
    return page.evaluate(async () => {
      /** @type {TaskOwnerState} */
      const state = Reflect.get(globalThis, '__taskDtoOwner')
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

  /** @param {import('playwright').Page} page @param {TaskOwnerProfile} profile @returns {Promise<unknown>} */
  static #replacement(page, profile) {
    return page.evaluate(async ({ modulePath }) => {
      /** @type {TaskOwnerModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      try { return await owner.ask() } finally { owner.__tachyonDispose() }
    }, profile)
  }

  /** @param {import('playwright').Page} page @returns {Promise<number>} */
  static #retiredChanges(page) {
    return page.evaluate(async () => {
      await new Promise(resolve => setTimeout(resolve, 650))
      /** @type {TaskOwnerState} */
      const state = Reflect.get(globalThis, '__taskDtoOwner')
      return state.changes
    })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static #cleanup(page) {
    return page.evaluate(() => {
      /** @type {TaskOwnerState | undefined} */
      const state = Reflect.get(globalThis, '__taskDtoOwner')
      try {
        if (state && !state.disposed) {
          state.disposed = true
          state.owner.__tachyonDispose()
        }
      } finally {
        Reflect.deleteProperty(globalThis, '__taskDtoOwner')
      }
    })
  }

}

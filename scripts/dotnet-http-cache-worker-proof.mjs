import assert from 'node:assert/strict'

/** @typedef {{read: () => Promise<{message: string}>, __tachyonDispose: () => void}} CachedOwner */
/** @typedef {{default: new() => CachedOwner}} CachedOwnerModule */
/** @typedef {{status: number, policy: string | null, vary: string | null, bytes: number, answer: {message: string}}} CachedWorkerEntry */

export class DotnetHttpCacheWorkerProof {
  /** @type {import('playwright').Page} */
  #page
  /** @type {string} */
  #origin
  /** @type {import('./dotnet-http-cache-test-origin.mjs').DotnetHttpCacheTestOrigin} */
  #network

  /** @param {import('playwright').Page} page @param {string} origin @param {import('./dotnet-http-cache-test-origin.mjs').DotnetHttpCacheTestOrigin} network */
  constructor(page, origin, network) {
    this.#page = page
    this.#origin = origin
    this.#network = network
  }

  /** @returns {Promise<Record<string, unknown>>} */
  async registered() {
    return this.#registration()
  }

  /** @returns {Promise<void>} */
  async filled() {
    this.#requirePublic(await this.#cache())
    this.#requireNetwork(['GET'])
  }

  /** @returns {Promise<void>} */
  async newOwner() {
    assert.deepEqual(await this.#readOwner(), { message: 'actual public Yon answer' })
    this.#requireNetwork(['GET'])
  }

  /** @returns {Promise<void>} */
  async evicted() {
    assert.equal(await this.#cache(), null, 'the real POST removes the worker GET entry')
    this.#requireNetwork(['GET', 'POST'])
  }

  /** @returns {Promise<void>} */
  async refill() {
    await this.#refill()
    this.#requirePublic(await this.#cache())
    this.#requireNetwork(['GET', 'POST', 'GET'])
  }

  /** @returns {Promise<void>} */
  async revalidated() {
    this.#requirePublic(await this.#cache())
    this.#requireNetwork(['GET', 'POST', 'GET', 'GET'])
  }

  /** @returns {Promise<void>} */
  async offline() {
    await this.#offline()
    this.#requireNetwork(['GET', 'POST', 'GET', 'GET'])
  }

  /** @returns {Promise<Record<string, unknown>>} */
  async #registration() {
    const origin = await this.#page.evaluate(() => ({ origin: location.origin, secure: isSecureContext, workerAvailable: 'serviceWorker' in navigator }))
    assert.equal(origin.origin, this.#origin)
    assert.equal(origin.secure, true, 'the named TLS test origin is a secure context')
    assert.equal(origin.workerAvailable, true)
    await this.#page.waitForFunction(() => navigator.serviceWorker.controller !== null)
    const worker = await this.#page.evaluate(async () => {
      const registration = await navigator.serviceWorker.ready
      return { scope: new URL(registration.scope).pathname, script: new URL(registration.active?.scriptURL ?? '').pathname }
    })
    assert.deepEqual(worker, { scope: '/', script: '/tachyon-sw.js' })
    return { ...origin, ...worker, localTestCertificate: true, systemTrustChanged: false, responseHeadersRewritten: false }
  }

  /** @returns {Promise<CachedWorkerEntry | null>} */
  async #cache() {
    const names = await this.#page.evaluate(async () => (await caches.keys()).filter(name => name.startsWith('tachyon-static-')))
    assert.equal(names.length, 1, 'the controlled fixture must have one current worker cache')
    return this.#page.evaluate(async ({ name, origin }) => {
      const cache = await caches.open(name)
      const response = await cache.match(`${origin}/api/cached-probe`)
      if (!response) return null
      return { status: response.status, policy: response.headers.get('cache-control'), vary: response.headers.get('vary'), bytes: Number(response.headers.get('x-tachyon-cache-bytes')), answer: await response.json() }
    }, { name: names[0], origin: this.#origin })
  }

  /** @param {CachedWorkerEntry | null} entry @returns {void} */
  #requirePublic(entry) {
    assert.ok(entry, 'the genuine Yon answer is stored in CacheStorage')
    assert.equal(entry.status, 200)
    assert.match(entry.policy ?? '', /\bpublic\b/i)
    assert.doesNotMatch(entry.policy ?? '', /\b(?:private|no-store)\b/i)
    assert.equal(entry.vary, null)
    assert.ok(entry.bytes > 0 && entry.bytes <= 256 * 1024)
    assert.deepEqual(entry.answer, { message: 'actual public Yon answer' })
  }

  /** @param {string[]} methods @returns {void} */
  #requireNetwork(methods) {
    assert.deepEqual(this.#network.requests(), methods.map(method => ({ method, path: '/api/cached-probe', cookie: false, authorization: false })), 'count genuine forwarded Yon requests, including service-worker hits that never reach Yon')
    assert.deepEqual(this.#network.failures(), [])
  }

  /** @returns {Promise<{message: string}>} */
  #readOwner() {
    return this.#page.evaluate(async () => {
      const modulePath = '/.tachyon/services/CachedHttpService.js'
      /** @type {CachedOwnerModule} */
      const Service = await import(modulePath)
      const owner = new Service.default()
      try { return await owner.read() } finally { owner.__tachyonDispose() }
    })
  }

  /** @returns {Promise<void>} */
  async #refill() {
    const response = await this.#page.evaluate(async () => {
      const answer = await fetch('/api/cached-probe', { credentials: 'omit' })
      return { status: answer.status, answer: await answer.json() }
    })
    assert.deepEqual(response, { status: 200, answer: { message: 'actual public Yon answer' } })
  }

  /** @returns {Promise<void>} */
  async #offline() {
    const context = this.#page.context()
    await context.setOffline(true)
    try {
      assert.deepEqual(await this.#readOwner(), { message: 'actual public Yon answer' }, 'a fresh C# owner obtains the worker answer offline')
    } finally {
      await context.setOffline(false)
    }
  }
}

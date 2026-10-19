class TacYonReads {
  static key = Symbol.for('tachyon.tac.yon-reads')
  static rules = (__CACHE_RULES__).map((rule) => ({ ...rule, match: new RegExp(rule.pattern) }))
  static cacheName = 'tachyon-yon-reads'
  static originTag = 'meta[name="tachyon-yon-origin"]'
  static mostEntries = 256
  static mostBytes = 32 * 1024 * 1024
  static mostEntryBytes = 256 * 1024
  static mostAge = 24 * 60 * 60 * 1000
  static mostWait = 2000
  static mostPendingWrites = 16
  static acceptedVary = ['accept-encoding', 'origin']
  static writes = Promise.resolve()
  static pendingWrites = 0
  static installed = TacYonReads.install()

  static install() {
    Object.defineProperty(globalThis, TacYonReads.key, {
      configurable: true,
      value: Object.freeze({ fetch: (url, init) => TacYonReads.fetch(url, init) }),
    })
    return true
  }

  static yonOrigin() {
    const declared = globalThis.document?.querySelector?.(TacYonReads.originTag)?.content
    return declared ? new URL(declared).origin : null
  }

  static async fetch(url, init = {}) {
    const request = new Request(url, init)
    const target = new URL(request.url)
    if (!globalThis.caches || target.origin !== TacYonReads.yonOrigin() || request.method === 'HEAD') return fetch(request)
    if (request.method !== 'GET') return TacYonReads.#writeThrough(request, target.pathname)
    const rule = TacYonReads.rules.find((candidate) => candidate.match.test(target.pathname))
    if (!rule) return fetch(request)
    if (request.cache === 'no-store' || request.headers.has('authorization') || request.headers.has('range')
        || rule.policy === 'no-store' || request.credentials !== 'omit') {
      await TacYonReads.#evict(request)
      return fetch(request)
    }
    const cacheFirst = request.cache !== 'no-cache' && rule.policy === 'cache-first'
    return cacheFirst ? TacYonReads.#fromCache(request) : TacYonReads.#fromNetwork(request)
  }

  static async #bounded(work) {
    let timer
    try { return await Promise.race([work, new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('cache unavailable')), TacYonReads.mostWait) })]) }
    finally { clearTimeout(timer) }
  }

  static #storable(response) {
    const vary = (response.headers.get('vary') || '').split(',').map((name) => name.trim().toLowerCase())
    return response.ok && !response.redirected && response.type !== 'opaque' && response.type !== 'opaqueredirect'
      && !/(?:^|,)\s*(?:private|no-store)\b/i.test(response.headers.get('cache-control') || '')
      && vary.every((name) => !name || TacYonReads.acceptedVary.includes(name))
  }

  static async #bodyBytes(response) {
    const reader = response.body?.getReader()
    if (!reader) return new Uint8Array()
    const chunks = []
    let length = 0
    const deadline = Date.now() + TacYonReads.mostWait
    try {
      while (true) {
        if (Date.now() >= deadline) throw new Error('cache body deadline')
        const { value, done } = await TacYonReads.#bounded(reader.read())
        if (done) break
        length += value.byteLength
        if (length > TacYonReads.mostEntryBytes) throw new Error('cache body limit')
        chunks.push(value)
      }
    } finally { void reader.cancel().catch(() => {}) }
    const bytes = new Uint8Array(length)
    let offset = 0
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength }
    return bytes
  }

  static async #evict(request) {
    try {
      const cache = await TacYonReads.#bounded(caches.open(TacYonReads.cacheName))
      await TacYonReads.#bounded(cache.delete(request))
    } catch {}
  }

  static async #writeThrough(request, path) {
    const response = await fetch(request)
    if (!response.ok) return response
    try {
      const cache = await TacYonReads.#bounded(caches.open(TacYonReads.cacheName))
      for (const entry of await TacYonReads.#bounded(cache.keys())) {
        const stored = new URL(entry.url).pathname
        if (stored === path || path.startsWith(stored + '/')) await TacYonReads.#bounded(cache.delete(entry))
      }
    } catch {}
    return response
  }

  static async #read(request) {
    try {
      const cache = await TacYonReads.#bounded(caches.open(TacYonReads.cacheName))
      const response = await TacYonReads.#bounded(cache.match(request))
      if (!response) return null
      const created = Number(response.headers.get('x-tachyon-cached-at'))
      if (!TacYonReads.#storable(response) || !created || Date.now() - created > TacYonReads.mostAge) {
        await TacYonReads.#bounded(cache.delete(request))
        return null
      }
      return response
    } catch { return null }
  }

  static async #write(request, response) {
    if (!TacYonReads.#storable(response)) { await TacYonReads.#evict(request); return }
    if (TacYonReads.pendingWrites >= TacYonReads.mostPendingWrites) return
    TacYonReads.pendingWrites += 1
    try {
      const bytes = await TacYonReads.#bodyBytes(response.clone())
      const headers = new Headers(response.headers)
      headers.delete('content-encoding')
      headers.set('content-length', String(bytes.byteLength))
      headers.set('x-tachyon-cache-bytes', String(bytes.byteLength))
      headers.set('x-tachyon-cached-at', String(Date.now()))
      const stored = new Response(bytes, { status: response.status, statusText: response.statusText, headers })
      TacYonReads.writes = TacYonReads.writes.catch(() => {}).then(() => TacYonReads.#put(request, stored, bytes.byteLength))
      await TacYonReads.writes
    } catch { await TacYonReads.#evict(request) }
    finally { TacYonReads.pendingWrites -= 1 }
  }

  static async #put(request, stored, length) {
    const cache = await TacYonReads.#bounded(caches.open(TacYonReads.cacheName))
    const entries = await TacYonReads.#bounded(cache.keys())
    let size = length
    for (const entry of entries) {
      const existing = await TacYonReads.#bounded(cache.match(entry))
      size += Number(existing?.headers.get('x-tachyon-cache-bytes')) || TacYonReads.mostBytes
    }
    if (entries.length >= TacYonReads.mostEntries || size > TacYonReads.mostBytes)
      for (const entry of entries) await TacYonReads.#bounded(cache.delete(entry))
    await TacYonReads.#bounded(cache.put(request, stored))
  }

  static async #fromCache(request) {
    return (await TacYonReads.#read(request)) || TacYonReads.#fromNetwork(request)
  }

  static async #fromNetwork(request) {
    let response
    try {
      response = await fetch(new Request(request, { cache: 'no-store' }))
    } catch (error) {
      if (request.signal.aborted) throw error
      const cached = await TacYonReads.#read(request)
      if (cached) return cached
      throw error
    }
    await TacYonReads.#bounded(TacYonReads.#write(request, response)).catch(() => {})
    return response
  }
}

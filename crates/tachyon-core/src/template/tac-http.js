class TacBrowserHttp {
  static key = Symbol.for('tachyon.tac.http')
  static mostRequests = 64
  static mostBytes = 1048576
  static requestBytes = 1048576
  static pending = 0
  static yonOriginTag = 'meta[name="tachyon-yon-origin"]'
  static installed = TacBrowserHttp.install()

  static install() {
    Object.defineProperty(globalThis, TacBrowserHttp.key, {
      configurable: true,
      value: Object.freeze({ start: (text) => TacBrowserHttp.start(text) }),
    })
  }

  static start(text) {
    const controller = new AbortController()
    return Object.freeze({ answer: TacBrowserHttp.#answer(text, controller), cancel: () => controller.abort('CANCELLED') })
  }

  static async #answer(text, controller) {
    try {
      const request = TacBrowserHttp.#request(text)
      const value = await TacBrowserHttp.#exchange(request, controller)
      const answer = JSON.stringify({ value })
      if (new TextEncoder().encode(answer).length > TacBrowserHttp.mostBytes) TacBrowserHttp.#fail('HTTP_LIMIT')
      return answer
    } catch (error) {
      const code = TacBrowserHttp.#code(error, controller)
      return JSON.stringify({ error: code === 'HTTP_STATUS' ? { code, status: error.status } : { code } })
    }
  }

  static #code(error, controller) {
    if (controller.signal.aborted) return controller.signal.reason === 'DEADLINE' ? 'DEADLINE' : 'CANCELLED'
    const allowed = ['HTTP_REQUEST', 'HTTP_ORIGIN', 'HTTP_HEADER', 'HTTP_QUERY', 'HTTP_BODY', 'HTTP_LIMIT', 'HTTP_RESPONSE', 'HTTP_STATUS', 'HTTP_CAPACITY']
    return allowed.includes(error?.code) ? error.code : 'HTTP_NETWORK'
  }

  static #fail(code, status) { throw Object.assign(new Error('Browser HTTP failed.'), { code, status }) }

  static #request(text) {
    if (typeof text !== 'string' || new TextEncoder().encode(text).length > TacBrowserHttp.requestBytes) TacBrowserHttp.#fail('HTTP_REQUEST')
    let request
    try { request = JSON.parse(text) } catch { TacBrowserHttp.#fail('HTTP_REQUEST') }
    if (!request || Array.isArray(request) || typeof request !== 'object') TacBrowserHttp.#fail('HTTP_REQUEST')
    const allowed = ['method', 'path', 'query', 'headers', 'body', 'redirect', 'credentials', 'deadline', 'maxResponseBytes', 'publicRead', 'revalidate']
    if (Object.keys(request).some((key) => !allowed.includes(key))) TacBrowserHttp.#fail('HTTP_REQUEST')
    if (!['GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE'].includes(request.method)) TacBrowserHttp.#fail('HTTP_REQUEST')
    if (request.redirect !== 'error' || request.credentials !== 'omit') TacBrowserHttp.#fail('HTTP_REQUEST')
    if (!Number.isSafeInteger(request.deadline) || request.deadline > Date.now() + 60000) TacBrowserHttp.#fail('HTTP_REQUEST')
    if (!Number.isInteger(request.maxResponseBytes) || request.maxResponseBytes < 1 || request.maxResponseBytes > TacBrowserHttp.mostBytes) TacBrowserHttp.#fail('HTTP_LIMIT')
    if (request.publicRead !== undefined && typeof request.publicRead !== 'boolean') TacBrowserHttp.#fail('HTTP_REQUEST')
    if (request.revalidate !== undefined && typeof request.revalidate !== 'boolean') TacBrowserHttp.#fail('HTTP_REQUEST')
    const url = TacBrowserHttp.#url(request)
    const headers = TacBrowserHttp.#headers(request.headers)
    const body = TacBrowserHttp.#body(request, headers)
    const cached = request.method === 'GET' && request.publicRead === true && !headers.has('authorization')
    return { url, headers, body, method: request.method, deadline: request.deadline, limit: request.maxResponseBytes,
      mode: new URL(url).origin === location.origin ? 'same-origin' : 'cors', cache: cached ? request.revalidate === true ? 'no-cache' : 'default' : 'no-store' }
  }

  static #url(request) {
    const path = request.path
    if (typeof path !== 'string' || !path.startsWith('/') || path.startsWith('//') || /[\s\\?#%]/.test(path) || path.split('/').some((part) => part === '.' || part === '..') || new TextEncoder().encode(path).length > 2048) TacBrowserHttp.#fail('HTTP_ORIGIN')
    const yonOrigin = TacBrowserHttp.#yonOrigin()
    const url = new URL(path, yonOrigin ?? location.href)
    if (![location.origin, yonOrigin].includes(url.origin)) TacBrowserHttp.#fail('HTTP_ORIGIN')
    const query = request.query ?? {}
    if (!query || typeof query !== 'object' || Array.isArray(query)) TacBrowserHttp.#fail('HTTP_QUERY')
    for (const [key, value] of Object.entries(query)) {
      if (key.length > 256 || !['string', 'number', 'boolean'].includes(typeof value) || new TextEncoder().encode(JSON.stringify(value)).length > 8192) TacBrowserHttp.#fail('HTTP_QUERY')
      TacBrowserHttp.#json(value)
      url.searchParams.set(key, String(value))
    }
    if (new TextEncoder().encode(url.href).length > 8192) TacBrowserHttp.#fail('HTTP_QUERY')
    return url.href
  }

  static #yonOrigin() {
    const declared = globalThis.document?.querySelector?.(TacBrowserHttp.yonOriginTag)?.content
    return declared ? new URL(declared).origin : null
  }

  static #headers(values = {}) {
    if (!values || typeof values !== 'object' || Array.isArray(values) || Object.keys(values).length > 128) TacBrowserHttp.#fail('HTTP_HEADER')
    const headers = new Headers()
    const refused = ['host', 'connection', 'content-length', 'transfer-encoding', 'cookie', 'origin', 'referer']
    for (const [key, value] of Object.entries(values)) {
      if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}$/.test(key) || refused.includes(key.toLowerCase()) || key.toLowerCase().startsWith('sec-') || key.toLowerCase().startsWith('proxy-') || typeof value !== 'string' || /[\r\n\0]/.test(value) || new TextEncoder().encode(value).length > 8192) TacBrowserHttp.#fail('HTTP_HEADER')
      headers.set(key, value)
    }
    return headers
  }

  static #body(request, headers) {
    if (!Object.hasOwn(request, 'body')) return undefined
    if (request.method === 'GET' || request.method === 'HEAD') TacBrowserHttp.#fail('HTTP_BODY')
    TacBrowserHttp.#json(request.body)
    headers.set('content-type', 'application/json')
    return JSON.stringify(request.body)
  }

  static #json(value, depth = 0) {
    if (depth > 32) TacBrowserHttp.#fail('HTTP_RESPONSE')
    if (typeof value === 'number' && (!Number.isFinite(value) || Math.abs(value) > Number.MAX_SAFE_INTEGER)) TacBrowserHttp.#fail('HTTP_RESPONSE')
    if (value && typeof value === 'object') {
      for (const item of Object.values(value)) TacBrowserHttp.#json(item, depth + 1)
    }
  }

  static async #exchange(request, controller) {
    if (TacBrowserHttp.pending >= TacBrowserHttp.mostRequests) TacBrowserHttp.#fail('HTTP_CAPACITY')
    TacBrowserHttp.pending += 1
    const remaining = request.deadline - Date.now()
    const timer = setTimeout(() => controller.abort('DEADLINE'), Math.max(0, remaining))
    try {
      if (remaining <= 0) { controller.abort('DEADLINE'); TacBrowserHttp.#fail('HTTP_REQUEST') }
      const response = await fetch(request.url, { method: request.method, headers: request.headers, body: request.body,
        credentials: 'omit', redirect: 'error', mode: request.mode, cache: request.cache, signal: controller.signal })
      if (!response.ok) TacBrowserHttp.#fail('HTTP_STATUS', response.status)
      const text = await TacBrowserHttp.#read(response, request.limit)
      if (controller.signal.aborted) TacBrowserHttp.#fail('HTTP_REQUEST')
      if (!text.trim()) return null
      let value
      try { value = JSON.parse(text) } catch { TacBrowserHttp.#fail('HTTP_RESPONSE') }
      TacBrowserHttp.#json(value)
      return value
    } finally { clearTimeout(timer); TacBrowserHttp.pending -= 1 }
  }

  static async #read(response, limit) {
    if (!response.body) return ''
    const bytes = TacBrowserHttp.#declared(response, limit)
      ? new Uint8Array(await response.arrayBuffer())
      : await TacBrowserHttp.#streamed(response.body.getReader(), limit)
    if (bytes.byteLength > limit) TacBrowserHttp.#fail('HTTP_LIMIT')
    try { return new TextDecoder('utf-8', { fatal: true }).decode(bytes) } catch { TacBrowserHttp.#fail('HTTP_RESPONSE') }
  }

  static #declared(response, limit) {
    const headers = response.headers
    const length = Number(headers?.get('content-length') ?? Number.NaN)
    if (length > limit) TacBrowserHttp.#fail('HTTP_LIMIT')
    const identity = (headers?.get('content-encoding') ?? 'identity') === 'identity'
    return Number.isSafeInteger(length) && identity
  }

  static async #streamed(reader, limit) {
    const chunks = []
    let length = 0
    try {
      while (true) {
        const { done, value } = await reader.read()
        if (done) break
        length += value.byteLength
        if (length > limit) TacBrowserHttp.#fail('HTTP_LIMIT')
        chunks.push(value)
      }
    } catch (error) {
      await reader.cancel().catch(() => {})
      throw error
    } finally { reader.releaseLock() }
    const bytes = new Uint8Array(length)
    let offset = 0
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength }
    return bytes
  }
}

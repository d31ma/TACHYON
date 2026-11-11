class __tachyon_ClientConfiguration {
  static bindTransport(transport) {
    if (transport === undefined) return
    if (typeof transport !== 'function' || (this.transport && this.transport !== transport)) throw new TypeError('Invalid Tac Client transport owner.')
    Object.defineProperty(this, 'transport', {value:transport})
  }
  static yonOrigin() {
    const declared = globalThis.document?.querySelector?.('meta[name="tachyon-yon-origin"]')?.content
    return declared ? new URL(declared).origin : null
  }
  static namedOrigin(path) {
    const [, name, rest] = /^\/\.tachyon\/origin\/([a-z][a-z0-9-]{0,31})(\/(?!\/).*)?$/.exec(path) ?? []
    if (!name) return null
    let origins = {}
    try { origins = JSON.parse(globalThis.document?.querySelector?.('meta[name="tachyon-yon-origins"]')?.content ?? '{}') ?? {} } catch {}
    if (!Object.hasOwn(origins, name)) throw new TypeError('Tac Client names an origin that is not in tac.yon.origins.')
    return { origin: new URL(origins[name]).origin, path: rest ?? '/' }
  }
  static fetch(url, init) {
    const reads = globalThis[Symbol.for('tachyon.tac.yon-reads')]
    return reads ? reads.fetch(url, init) : fetch(url, init)
  }
  static configure() {
    return { http: Object.fromEntries(this.adapters.map(name =>
      [name, {exchange: request => __tachyon_client_exchange__(request, this.transport)}])) }
  }
}
async function __tachyon_client_exchange__(request, transport) {
  const yonOrigin = __tachyon_ClientConfiguration.yonOrigin()
  const named = __tachyon_ClientConfiguration.namedOrigin(request.path)
  const url = new URL(named?.path ?? request.path, named?.origin ?? yonOrigin ?? location.href)
  if (!['http:', 'https:'].includes(url.protocol) || ![location.origin, yonOrigin, named?.origin].includes(url.origin) || url.username || url.password) {
    throw new TypeError('Tac Client requires a same-origin HTTP endpoint, the tac.yon.origin endpoint or a tac.yon.origins endpoint.')
  }
  for (const [name, value] of Object.entries(request.query)) url.searchParams.set(name, String(value))
  const headers = new Headers(request.headers)
  const publicRead = request.publicRead === true && request.method === 'GET' && !headers.has('authorization')
  const revalidate = publicRead && request.revalidate === true
  const publicCache = revalidate ? 'no-cache' : 'default'
  headers.set('accept', 'application/json')
  const body = Object.hasOwn(request, 'body') ? JSON.stringify(request.body) : undefined
  if (body !== undefined) headers.set('content-type', 'application/json')
  const init = {
    method: request.method, headers, body, signal: request.signal,
    redirect: 'error', credentials: 'omit', cache: publicRead ? publicCache : 'no-store',
  }
  const response = transport ? await transport(url, init, {publicRead, revalidate}) : await __tachyon_ClientConfiguration.fetch(url, init)
  if (!response.ok) throw await __tachyon_ClientAnswer.failure(response, request.maxResponseBytes)
  if (request.method === 'HEAD' || response.status === 204) { await response.body?.cancel(); return null }
  const type = __tachyon_ClientAnswer.mediaType(response)
  if (type !== 'application/json' && !/^application\/[a-z0-9!#$&^_.+-]+\+json$/.test(type)) {
    await response.body?.cancel(); throw new TypeError('Tac Client requires a JSON response.')
  }
  return __tachyon_ClientAnswer.read(response, request.maxResponseBytes)
}
class __tachyon_ClientAnswer {
  static problemTypes = ['application/json', 'application/problem+json']
  static mediaType(response) {
    return (response.headers.get('content-type') || '').split(';')[0].trim().toLowerCase()
  }
  static async failure(response, limit) {
    const body = await __tachyon_ClientAnswer.problem(response, limit)
    return Object.assign(new Error('Tac Client HTTP request failed.'), { code: 'HTTP_STATUS', status: response.status, body })
  }
  static async problem(response, limit) {
    if (!__tachyon_ClientAnswer.problemTypes.includes(__tachyon_ClientAnswer.mediaType(response))) {
      await response.body?.cancel(); return undefined
    }
    return __tachyon_ClientAnswer.read(response, limit).catch(() => undefined)
  }
  static async read(response, limit) {
    const reader = response.body?.getReader()
    if (!reader) throw new TypeError('Tac Client response has no body.')
    let size = 0
    const bytes = new Uint8Array(limit)
    try {
      while (true) {
        const { value, done } = await reader.read()
        if (done) break
        if (value.byteLength > bytes.byteLength - size) throw new RangeError('Tac Client response exceeds its byte limit.')
        bytes.set(value, size)
        size += value.byteLength
      }
    } catch (error) { await reader.cancel(); throw error }
    finally { reader.releaseLock() }
    return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(0, size)))
  }
}

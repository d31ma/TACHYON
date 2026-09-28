class __tachyon_ClientConfiguration {
  static bindTransport(transport) {
    if (transport === undefined) {
      if (Object.values(this.adapters).some(policy => policy !== 'no-store')) throw new TypeError('Cached Client requires its mounted Tac owner.')
      return
    }
    if (typeof transport !== 'function' || (this.transport && this.transport !== transport)) throw new TypeError('Invalid Tac Client transport owner.')
    Object.defineProperty(this, 'transport', {value:transport})
  }
  static configure() {
    return { http: Object.fromEntries(Object.entries(this.adapters).map(([name, policy]) =>
      [name, {exchange: request => __tachyon_client_exchange__(request, policy, this.transport)}])) }
  }
}
async function __tachyon_client_exchange__(request, policy, transport) {
  const url = new URL(request.path, location.href)
  if (!['http:', 'https:'].includes(url.protocol) || url.origin !== location.origin || url.username || url.password) {
    throw new TypeError('Tac Client requires a same-origin HTTP endpoint.')
  }
  for (const [name, value] of Object.entries(request.query)) url.searchParams.set(name, String(value))
  const headers = new Headers(request.headers)
  headers.set('accept', 'application/json')
  const body = Object.hasOwn(request, 'body') ? JSON.stringify(request.body) : undefined
  if (body !== undefined) headers.set('content-type', 'application/json')
  const init = {
    method: request.method, headers, body, signal: request.signal,
    redirect: 'error', credentials: 'omit', cache: policy === 'no-store' ? 'no-store' : 'default',
  }
  const response = transport ? await transport(url, init, {cache:policy}) : await fetch(url, {...init, cache:'no-store'})
  if (!response.ok) { await response.body?.cancel(); throw new Error('Tac Client HTTP request failed.') }
  if (request.method === 'HEAD' || response.status === 204) { await response.body?.cancel(); return null }
  const type = (response.headers.get('content-type') || '').split(';')[0].trim().toLowerCase()
  if (type !== 'application/json' && !/^application\/[a-z0-9!#$&^_.+-]+\+json$/.test(type)) {
    await response.body?.cancel(); throw new TypeError('Tac Client requires a JSON response.')
  }
  const reader = response.body?.getReader()
  if (!reader) throw new TypeError('Tac Client response has no body.')
  let size = 0
  const bytes = new Uint8Array(request.maxResponseBytes)
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

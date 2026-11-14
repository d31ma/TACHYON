class TacTransfer {
  static hostsTag = 'meta[name="tachyon-outgoing-hosts"]'
  static originTag = 'meta[name="tachyon-yon-origin"]'
  static mostBody = 8192
  static mostHeaders = 32
  static mostHeaderBytes = 8192
  static deadline = 900000
  static methods = ['PUT', 'POST']
  static namedHeaders = ['authorization', 'content-type', 'content-md5', 'content-disposition', 'content-encoding', 'cache-control']
  static headerPrefixes = ['x-amz-', 'x-goog-', 'x-ms-']
  static headerName = /^[A-Za-z0-9!#$%&'*+.^_|~-]+$/
  static unsafeValue = /[\r\n\0]/
  static localHosts = ['localhost', '127.0.0.1', '[::1]']
  static anySubdomain = '*.'
  static uploadOrigin = 'UPLOAD_ORIGIN: An upload goes to a path such as "/uploads", or to an https address whose host is in tac.outgoing.hosts in client/manifest.json.'
  static downloadOrigin = 'DOWNLOAD_ORIGIN: A download comes from a path such as "/attachments/1", or from an https address whose host is in tac.outgoing.hosts in client/manifest.json.'
  static uploads = new Map()

  static upload(load, { method, address, headers }, reply, answer) {
    const init = { method: TacTransfer.method(method), headers: TacTransfer.headers(headers, 'UPLOAD_HEADER: An upload') }
    const url = TacTransfer.target(address, TacTransfer.uploadOrigin)
    const upload = TacRandomId.next()
    const control = new AbortController()
    const timer = setTimeout(() => control.abort('DEADLINE'), TacTransfer.deadline)
    TacTransfer.uploads.set(upload, control)
    const sent = (loaded) => TacTransfer.send(url, { ...init, body: loaded }, control.signal)
      .then((response) => TacTransfer.finished(upload, response), () => TacTransfer.broken(upload, control.signal))
      .then(answer.finished)
    const body = load()
    const sending = typeof body?.then === 'function' ? body.then(sent, answer.unread) : sent(body)
    sending.then((event) => TacTransfer.settle(upload, timer, reply, event))
    return upload
  }

  static download(address, headers, signal) {
    const init = { method: 'GET', headers: TacTransfer.headers(headers, 'DOWNLOAD_HEADER: A download') }
    const url = TacTransfer.target(address, TacTransfer.downloadOrigin)
    return TacTransfer.send(url, init, signal)
  }

  static send(url, init, signal) {
    return fetch(url, { ...init, credentials: 'omit', redirect: 'error', cache: 'no-store', referrerPolicy: 'no-referrer', signal })
  }

  static method(method) {
    const named = String(method || TacTransfer.methods[0]).toUpperCase()
    if (!TacTransfer.methods.includes(named)) throw new Error(`UPLOAD_REQUEST: An upload uses ${TacTransfer.methods.join(' or ')}.`)
    return named
  }

  static headers(lines, refusal) {
    const entries = String(lines ?? '').split('\n').filter((line) => line.trim()).map(TacTransfer.header)
    const bytes = entries.reduce((total, [name, value]) => total + String(name).length + String(value).length, 0)
    if (entries.length > TacTransfer.mostHeaders || bytes > TacTransfer.mostHeaderBytes || !entries.every(TacTransfer.allowedHeader)) {
      throw new Error(`${refusal} sends at most ${TacTransfer.mostHeaders} headers in ${TacTransfer.mostHeaderBytes} bytes: ${TacTransfer.namedHeaders.join(', ')}, or names that start with ${TacTransfer.headerPrefixes.join(', ')}.`)
    }
    return Object.fromEntries(entries.map(([name, value]) => [name.toLowerCase(), value]))
  }

  static header(line) {
    const colon = line.indexOf(':')
    return colon < 1 ? [line, null] : [line.slice(0, colon).trim(), line.slice(colon + 1).trim()]
  }

  static allowedHeader([name, value]) {
    const lower = String(name).toLowerCase()
    const known = TacTransfer.namedHeaders.includes(lower) || TacTransfer.headerPrefixes.some((prefix) => lower.startsWith(prefix))
    return known && TacTransfer.headerName.test(name) && typeof value === 'string' && !TacTransfer.unsafeValue.test(value)
  }

  static target(given, refusal) {
    const text = String(given ?? '')
    const base = TacTransfer.base()
    const url = TacTransfer.parsed(text, base)
    const sameOrigin = text.startsWith('/') && url?.origin === new URL(base).origin
    if (!url || url.username || url.password || !(sameOrigin || TacTransfer.declared(url))) {
      throw new Error(`${refusal} '${text}' is neither.`)
    }
    return url.href
  }

  static parsed(text, base) {
    return URL.canParse(text, base) ? new URL(text, base) : null
  }

  static base() {
    return globalThis.document?.querySelector?.(TacTransfer.originTag)?.content || globalThis.location.origin
  }

  static declared(url) {
    const local = url.protocol === 'http:' && TacTransfer.localHosts.includes(url.hostname)
    const secure = url.protocol === 'https:' || local
    return secure && TacTransfer.hosts().some((host) => TacTransfer.matches(host, url.hostname))
  }

  static hosts() {
    return (globalThis.document?.querySelector?.(TacTransfer.hostsTag)?.content ?? '').split(' ').filter(Boolean)
  }

  static matches(host, hostname) {
    const below = host.startsWith(TacTransfer.anySubdomain) && hostname.endsWith(host.slice(1))
    return host === hostname || below
  }

  static async finished(upload, response) {
    const text = await response.text()
    const body = text.slice(0, TacTransfer.mostBody)
    return { upload, status: response.status, body, truncated: body.length < text.length, error: '' }
  }

  static broken(upload, signal) {
    const error = signal.aborted ? String(signal.reason) : 'UPLOAD_NETWORK'
    return { upload, status: 0, body: '', truncated: false, error }
  }

  static settle(upload, timer, reply, event) {
    clearTimeout(timer)
    TacTransfer.uploads.delete(upload)
    reply(event)
  }

  static cancel(upload) {
    TacTransfer.uploads.get(String(upload))?.abort('CANCELLED')
    return null
  }
}

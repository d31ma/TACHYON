class TacFiles {
  static key = Symbol.for('tachyon.tac.files')
  static webKey = Symbol.for('tachyon.tac.web')
  static grants = 'meta[name="tachyon-browser-apis"]'
  static hostsTag = 'meta[name="tachyon-outgoing-hosts"]'
  static originTag = 'meta[name="tachyon-yon-origin"]'
  static zoneAttribute = 'data-tac-drop-zone'
  static overAttribute = 'data-tac-drag-over'
  static mostHeld = 256
  static mostChosen = 32
  static mostAccepted = 32
  static mostChunk = 32768
  static mostText = 255
  static mostBody = 8192
  static mostHeaders = 32
  static mostHeaderBytes = 8192
  static uploadDeadline = 900000
  static hashChunk = 1048576
  static methods = ['PUT', 'POST']
  static namedHeaders = ['authorization', 'content-type', 'content-md5', 'content-disposition', 'content-encoding', 'cache-control']
  static headerPrefixes = ['x-amz-', 'x-goog-', 'x-ms-']
  static headerName = /^[A-Za-z0-9!#$%&'*+.^_|~-]+$/
  static unsafeValue = /[\r\n\0]/
  static acceptedType = /^(\.[A-Za-z0-9._+-]{1,64}|[A-Za-z0-9.+-]{1,64}\/(\*|[A-Za-z0-9.+-]{1,64}))$/
  static localHosts = ['localhost', '127.0.0.1', '[::1]']
  static anySubdomain = '*.'
  static held = new Map()
  static uploads = new Map()
  static picking = null
  static installed = TacFiles.install()

  static install() {
    Object.defineProperty(globalThis, TacFiles.key, {
      configurable: true,
      value: Object.freeze({
        pick: (reply, accept, multiple) => TacFiles.pick(reply, accept, multiple),
        read: (reply, id, offset, count) => TacFiles.read(reply, id, offset, count),
        release: (reply, id) => TacFiles.release(id),
        upload: (reply, id, method, url, headers) => TacFiles.upload(reply, id, method, url, headers),
        cancel: (reply, upload) => TacFiles.cancel(upload),
      }),
    })
    globalThis.document?.addEventListener?.('dragover', (event) => TacFiles.over(event))
    globalThis.document?.addEventListener?.('dragleave', (event) => TacFiles.zone(event)?.removeAttribute(TacFiles.overAttribute))
    globalThis.document?.addEventListener?.('drop', (event) => TacFiles.drop(event))
    return true
  }

  static pick(reply, accept, multiple) {
    const input = document.createElement('input')
    input.type = 'file'
    input.multiple = multiple === true
    input.accept = TacFiles.accepted(accept)
    input.addEventListener('change', () => TacFiles.answer(reply, 'pick', TacFiles.hold(input.files), (files) => TacFiles.events('FilePicked', files, {})))
    input.addEventListener('cancel', () => reply(['FilePickCancelled', {}]))
    TacFiles.picking = input
    input.click()
    return null
  }

  static accepted(accept) {
    const list = String(accept ?? '').split(',').map((type) => type.trim()).filter(Boolean)
    if (list.length > TacFiles.mostAccepted || !list.every((type) => TacFiles.acceptedType.test(type))) {
      throw new RangeError(`Accept is a comma-separated list of at most ${TacFiles.mostAccepted} types, such as ".pdf,image/png,image/*".`)
    }
    return list.join(',')
  }

  static answer(reply, call, work, events) {
    work.then(events, (error) => [TacFiles.failed(call, error)]).then((list) => list.forEach(reply))
  }

  static events(name, files, extra) {
    return files.map((file, index) => [name, { ...extra, ...file, index, count: files.length }])
  }

  static failed(call, error) {
    return ['BrowserCallFailed', { api: 'files', call, message: String(error?.message ?? error) }]
  }

  static async hold(chosen) {
    const files = [...(chosen ?? [])]
    if (files.length > TacFiles.mostChosen || TacFiles.held.size + files.length > TacFiles.mostHeld) {
      throw new RangeError(`A page holds at most ${TacFiles.mostHeld} files, and takes at most ${TacFiles.mostChosen} at once. Release files you no longer need.`)
    }
    const picked = []
    for (const file of files) picked.push(await TacFiles.described(file))
    return picked
  }

  static async described(file) {
    const sha256 = await TacFiles.digest(file)
    const id = TacRandomId.next()
    TacFiles.held.set(id, file)
    return { id, name: String(file.name).slice(0, TacFiles.mostText), size: file.size, media: String(file.type).slice(0, TacFiles.mostText), sha256 }
  }

  static async digest(file) {
    const hash = new TacSha256()
    for (let offset = 0; offset < file.size; offset += TacFiles.hashChunk) {
      hash.update(new Uint8Array(await file.slice(offset, offset + TacFiles.hashChunk).arrayBuffer()))
    }
    return hash.digest()
  }

  static read(reply, id, offset, count) {
    const file = TacFiles.file(id)
    TacFiles.range(offset, count)
    const chunk = file.slice(offset, offset + count).arrayBuffer()
    TacFiles.answer(reply, 'read', chunk, (bytes) => [['FileChunkRead', { id, offset, size: file.size, data: TacFiles.base64(new Uint8Array(bytes)) }]])
    return null
  }

  static file(id) {
    const file = TacFiles.held.get(String(id))
    if (!file) throw new Error(`UPLOAD_FILE: No file has the id '${id}'. Pick or drop it first; a released file is gone.`)
    return file
  }

  static range(offset, count) {
    const whole = [offset, count].every(Number.isSafeInteger)
    if (!whole || offset < 0 || count < 1 || count > TacFiles.mostChunk) {
      throw new RangeError(`Read 1 to ${TacFiles.mostChunk} bytes at an offset of 0 or more.`)
    }
  }

  static base64(bytes) {
    return btoa(String.fromCharCode(...bytes))
  }

  static release(id) {
    TacFiles.held.delete(String(id))
    return null
  }

  static upload(reply, id, method, address, headers) {
    const file = TacFiles.file(id)
    const init = { method: TacFiles.method(method), headers: TacFiles.headers(headers), body: file }
    const url = TacFiles.target(address)
    const upload = TacRandomId.next()
    const control = new AbortController()
    const timer = setTimeout(() => control.abort('DEADLINE'), TacFiles.uploadDeadline)
    TacFiles.uploads.set(upload, control)
    const sent = fetch(url, { ...init, credentials: 'omit', redirect: 'error', cache: 'no-store', referrerPolicy: 'no-referrer', signal: control.signal })
    sent.then((response) => TacFiles.finished(upload, id, response), () => TacFiles.broken(upload, id, control.signal))
      .then((event) => TacFiles.settle(upload, timer, reply, event))
    return upload
  }

  static method(method) {
    const named = String(method || TacFiles.methods[0]).toUpperCase()
    if (!TacFiles.methods.includes(named)) throw new Error(`UPLOAD_REQUEST: An upload uses ${TacFiles.methods.join(' or ')}.`)
    return named
  }

  static headers(lines) {
    const entries = String(lines ?? '').split('\n').filter((line) => line.trim()).map(TacFiles.header)
    const bytes = entries.reduce((total, [name, value]) => total + String(name).length + String(value).length, 0)
    if (entries.length > TacFiles.mostHeaders || bytes > TacFiles.mostHeaderBytes || !entries.every(TacFiles.allowedHeader)) {
      throw new Error(`UPLOAD_HEADER: An upload sends at most ${TacFiles.mostHeaders} headers in ${TacFiles.mostHeaderBytes} bytes: ${TacFiles.namedHeaders.join(', ')}, or names that start with ${TacFiles.headerPrefixes.join(', ')}.`)
    }
    return Object.fromEntries(entries.map(([name, value]) => [name.toLowerCase(), value]))
  }

  static header(line) {
    const colon = line.indexOf(':')
    return colon < 1 ? [line, null] : [line.slice(0, colon).trim(), line.slice(colon + 1).trim()]
  }

  static allowedHeader([name, value]) {
    const lower = String(name).toLowerCase()
    const known = TacFiles.namedHeaders.includes(lower) || TacFiles.headerPrefixes.some((prefix) => lower.startsWith(prefix))
    return known && TacFiles.headerName.test(name) && typeof value === 'string' && !TacFiles.unsafeValue.test(value)
  }

  static target(given) {
    const text = String(given ?? '')
    const base = TacFiles.base()
    const url = TacFiles.parsed(text, base)
    const sameOrigin = text.startsWith('/') && url?.origin === new URL(base).origin
    if (!url || url.username || url.password || !(sameOrigin || TacFiles.declared(url))) {
      throw new Error(`UPLOAD_ORIGIN: An upload goes to a path such as "/uploads", or to an https address whose host is in tac.outgoing.hosts in client/manifest.json. '${text}' is neither.`)
    }
    return url.href
  }

  static parsed(text, base) {
    return URL.canParse(text, base) ? new URL(text, base) : null
  }

  static base() {
    return globalThis.document?.querySelector?.(TacFiles.originTag)?.content || globalThis.location.origin
  }

  static declared(url) {
    const local = url.protocol === 'http:' && TacFiles.localHosts.includes(url.hostname)
    const secure = url.protocol === 'https:' || local
    return secure && TacFiles.hosts().some((host) => TacFiles.matches(host, url.hostname))
  }

  static hosts() {
    return (globalThis.document?.querySelector?.(TacFiles.hostsTag)?.content ?? '').split(' ').filter(Boolean)
  }

  static matches(host, hostname) {
    const below = host.startsWith(TacFiles.anySubdomain) && hostname.endsWith(host.slice(1))
    return host === hostname || below
  }

  static async finished(upload, id, response) {
    const text = await response.text()
    const body = text.slice(0, TacFiles.mostBody)
    return ['UploadFinished', { upload, id, status: response.status, body, truncated: body.length < text.length, error: '' }]
  }

  static broken(upload, id, signal) {
    const error = signal.aborted ? String(signal.reason) : 'UPLOAD_NETWORK'
    return ['UploadFinished', { upload, id, status: 0, body: '', truncated: false, error }]
  }

  static settle(upload, timer, reply, event) {
    clearTimeout(timer)
    TacFiles.uploads.delete(upload)
    reply(event)
  }

  static cancel(upload) {
    TacFiles.uploads.get(String(upload))?.abort('CANCELLED')
    return null
  }

  static zone(event) {
    return event.target?.closest?.(`[${TacFiles.zoneAttribute}]`) ?? null
  }

  static carriesFiles(event) {
    const granted = document.querySelector(TacFiles.grants)?.content.split(' ').includes('files') === true
    return granted && [...(event.dataTransfer?.types ?? [])].includes('Files')
  }

  static over(event) {
    const zone = TacFiles.zone(event)
    if (!zone || !TacFiles.carriesFiles(event)) return
    event.preventDefault()
    zone.setAttribute(TacFiles.overAttribute, '')
  }

  static drop(event) {
    const zone = TacFiles.zone(event)
    if (!zone || !TacFiles.carriesFiles(event)) return
    event.preventDefault()
    zone.removeAttribute(TacFiles.overAttribute)
    const name = zone.getAttribute(TacFiles.zoneAttribute)
    const publish = (dropped) => globalThis[TacFiles.webKey]?.pageEvent(dropped)
    TacFiles.answer(publish, 'drop', TacFiles.hold(event.dataTransfer.files), (files) => TacFiles.events('FileDropped', files, { zone: name }))
  }
}

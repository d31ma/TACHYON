class TacBlobs {
  static key = Symbol.for('tachyon.tac.blobs')
  static filesKey = Symbol.for('tachyon.tac.files')
  static sealKey = Symbol.for('tachyon.tac.blobs.seal')
  static spaceKey = Symbol.for('tachyon.tac.space')
  static limitsTag = 'meta[name="tachyon-blobs"]'
  static database = 'tachyon-blobs'
  static version = 1
  static stores = ['blobs', 'index']
  static mostKeyBytes = 512
  static mostChunk = 32768
  static mostMedia = 255
  static hashChunk = 1048576
  static fallbackLimits = [536870912, 67108864]
  static blobRefusals = ['UnknownError', 'DataCloneError', 'NotSupportedError']
  static forbiddenKey = /[\u0000-\u001f\u007f]|\.\./
  static messages = Object.freeze({
    quota: 'The blobs would use more space than tac.storage.blobs.maxBytes or the browser allows. Remove blobs first.',
    size: 'The blob is larger than tac.storage.blobs.maxBlobBytes.',
    missing: 'No blob has this key.',
    network: 'The download did not reach the server.',
    status: 'The server did not answer the download with a 2xx status.',
    unavailable: 'This page cannot keep blobs: IndexedDB or the seal for this blob is not available.',
    failed: 'The blob store failed.',
  })
  static plain = Object.freeze({
    keyId: '',
    seal: async (blob) => blob,
    open: async (blob, keyId) => keyId === '' ? blob : TacBlobs.refuse('unavailable'),
  })
  static opened = new Map()
  static connection = null
  static installed = TacBlobs.install()

  static install() {
    Object.defineProperty(globalThis, TacBlobs.key, {
      configurable: true,
      value: Object.freeze({
        saveHeld: (reply, id, key) => TacBlobs.saveHeld(reply, id, key),
        download: (reply, url, key, headers) => TacBlobs.download(reply, url, key, headers),
        list: (reply, prefix) => TacBlobs.list(reply, prefix),
        read: (reply, key, offset, count) => TacBlobs.read(reply, key, offset, count),
        remove: (reply, key) => TacBlobs.remove(reply, key),
        url: (reply, key) => TacBlobs.opened.get(TacBlobs.checked(key)) ?? '',
        open: (reply, key) => TacBlobs.open(reply, key),
        close: (reply, key) => TacBlobs.close(TacBlobs.checked(key)),
        upload: (reply, key, method, url, headers) => TacBlobs.upload(reply, key, method, url, headers),
        cancel: (reply, upload) => TacTransfer.cancel(upload),
      }),
    })
    ;(globalThis[TacBlobs.spaceKey] ??= []).push(() => TacBlobs.used())
    return true
  }

  static checked(key) {
    const text = String(key ?? '')
    const bytes = new TextEncoder().encode(text).length
    if (bytes < 1 || bytes > TacBlobs.mostKeyBytes || TacBlobs.forbiddenKey.test(text)) {
      throw new RangeError(`BLOB_KEY: A blob key is 1 to ${TacBlobs.mostKeyBytes} bytes of text with no control character and no "..".`)
    }
    return text
  }

  static limits() {
    const declared = (globalThis.document?.querySelector?.(TacBlobs.limitsTag)?.content ?? '').split(' ').map(Number)
    const [maxBytes, maxBlobBytes] = TacBlobs.fallbackLimits.map((fallback, index) => declared[index] || fallback)
    return { maxBytes, maxBlobBytes }
  }

  static refuse(reason) {
    throw Object.assign(new Error(TacBlobs.messages[reason]), { blobReason: reason })
  }

  static reason(error) {
    if (error?.blobReason) return error.blobReason
    return error?.name === 'QuotaExceededError' ? 'quota' : 'failed'
  }

  static answer(reply, call, key, work) {
    work.then(reply, (error) => reply(TacBlobs.failed(call, key, error)))
    return null
  }

  static failed(call, key, error) {
    const reason = TacBlobs.reason(error)
    return ['BlobFailed', { call, key, reason, message: TacBlobs.messages[reason] }]
  }

  static seal() {
    return globalThis[TacBlobs.sealKey] ?? TacBlobs.plain
  }

  static saveHeld(reply, id, key) {
    const name = TacBlobs.checked(key)
    const file = TacBlobs.held(id)
    return TacBlobs.answer(reply, 'saveHeld', name, TacBlobs.save(name, file))
  }

  static held(id) {
    const files = globalThis[TacBlobs.filesKey]
    if (!files) throw new Error('BLOB_FILE: Saving a held file needs "files" in tac.browserApis as well.')
    return files.held(String(id))
  }

  static download(reply, address, key, headers) {
    const name = TacBlobs.checked(key)
    const control = new AbortController()
    const sent = TacTransfer.download(address, headers, control.signal)
    const timer = setTimeout(() => control.abort('DEADLINE'), TacTransfer.deadline)
    return TacBlobs.answer(reply, 'download', name, TacBlobs.received(name, sent).finally(() => clearTimeout(timer)))
  }

  static async received(key, sent) {
    const response = await sent.catch(() => TacBlobs.refuse('network'))
    if (!response.ok) TacBlobs.refuse('status')
    const media = response.headers.get('content-type') ?? ''
    return TacBlobs.save(key, await TacBlobs.body(response, media))
  }

  static async body(response, media) {
    const most = TacBlobs.limits().maxBlobBytes
    if (Number(response.headers.get('content-length')) > most) TacBlobs.refuse('size')
    const reader = response.body.getReader()
    const parts = []
    let size = 0
    for (let part = await TacBlobs.part(reader); part; part = await TacBlobs.part(reader)) {
      size += part.length
      if (size > most) await TacBlobs.tooLarge(reader)
      parts.push(part)
    }
    return new Blob(parts, { type: media })
  }

  static async tooLarge(reader) {
    await reader.cancel().catch(() => null)
    TacBlobs.refuse('size')
  }

  static async part(reader) {
    const { done, value } = await reader.read().catch(() => TacBlobs.refuse('network'))
    return done ? null : value
  }

  static async save(key, blob) {
    if (blob.size > TacBlobs.limits().maxBlobBytes) TacBlobs.refuse('size')
    const seal = TacBlobs.seal()
    const media = String(blob.type).slice(0, TacBlobs.mostMedia)
    const row = { key, bytes: blob.size, media, sha256: await TacBlobs.digest(blob), saved: Date.now(), keyId: seal.keyId }
    const stored = await seal.seal(blob)
    if (!await TacBlobs.put(row, stored)) TacBlobs.refuse('quota')
    TacBlobs.close(key)
    return ['BlobSaved', { key, bytes: row.bytes, media, sha256: row.sha256 }]
  }

  static async digest(blob) {
    const hash = new TacSha256()
    for (let offset = 0; offset < blob.size; offset += TacBlobs.hashChunk) {
      hash.update(new Uint8Array(await blob.slice(offset, offset + TacBlobs.hashChunk).arrayBuffer()))
    }
    return hash.digest()
  }

  static async put(row, stored) {
    try {
      return await TacBlobs.keep(row, stored)
    } catch (error) {
      if (!TacBlobs.unstorable(error, stored)) throw error
      return TacBlobs.keep(row, await stored.arrayBuffer())
    }
  }

  static unstorable(error, stored) {
    return stored instanceof Blob && TacBlobs.blobRefusals.includes(error?.name)
  }

  static keep(row, stored) {
    return TacBlobs.run('readwrite', (stores, done) => {
      const rows = stores.index.getAll()
      rows.onsuccess = () => done(TacBlobs.fits(rows.result, row) && TacBlobs.write(stores, row, stored))
    })
  }

  static fits(rows, row) {
    const others = rows.reduce((total, other) => total + (other.key === row.key ? 0 : other.bytes), 0)
    return others + row.bytes <= TacBlobs.limits().maxBytes
  }

  static write(stores, row, stored) {
    stores.blobs.put(stored, row.key)
    stores.index.put(row, row.key)
    return true
  }

  static list(reply, prefix) {
    const start = String(prefix ?? '')
    TacBlobs.listed(start).then((events) => events.forEach(reply), (error) => reply(TacBlobs.failed('list', start, error)))
    return null
  }

  static async listed(prefix) {
    const rows = await TacBlobs.rows()
    const found = rows.filter((row) => row.key.startsWith(prefix))
    const events = found.map(({ key, bytes, media, sha256, saved }, index) => ['BlobFound', { key, bytes, media, sha256, saved, index, count: found.length }])
    const listed = { prefix, count: found.length, bytes: TacBlobs.total(found), usedBytes: TacBlobs.total(rows), maxBytes: TacBlobs.limits().maxBytes }
    return [...events, ['BlobsListed', listed]]
  }

  static total(rows) {
    return rows.reduce((sum, row) => sum + row.bytes, 0)
  }

  static rows() {
    return TacBlobs.run('readonly', (stores, done) => {
      const rows = stores.index.getAll()
      rows.onsuccess = () => done(rows.result)
    })
  }

  static async used() {
    return TacBlobs.total(await TacBlobs.rows().catch(() => []))
  }

  static read(reply, key, offset, count) {
    const name = TacBlobs.checked(key)
    const whole = [offset, count].every(Number.isSafeInteger)
    if (!whole || offset < 0 || count < 1 || count > TacBlobs.mostChunk) {
      throw new RangeError(`Read 1 to ${TacBlobs.mostChunk} bytes at an offset of 0 or more.`)
    }
    return TacBlobs.answer(reply, 'read', name, TacBlobs.chunk(name, offset, count))
  }

  static async chunk(key, offset, count) {
    const blob = await TacBlobs.plainBlob(key)
    const bytes = new Uint8Array(await blob.slice(offset, offset + count).arrayBuffer())
    return ['BlobChunkRead', { key, offset, size: blob.size, data: btoa(String.fromCharCode(...bytes)) }]
  }

  static async plainBlob(key) {
    const [stored, row] = await TacBlobs.entry(key)
    if (!stored || !row) TacBlobs.refuse('missing')
    const blob = stored instanceof Blob ? stored : new Blob([stored], { type: row.media })
    return TacBlobs.seal().open(blob, row.keyId)
  }

  static entry(key) {
    return TacBlobs.run('readonly', (stores, done) => {
      const stored = stores.blobs.get(key)
      const row = stores.index.get(key)
      row.onsuccess = () => done([stored.result, row.result])
    })
  }

  static remove(reply, key) {
    const name = TacBlobs.checked(key)
    TacBlobs.close(name)
    return TacBlobs.answer(reply, 'remove', name, TacBlobs.removed(name))
  }

  static async removed(key) {
    const bytes = await TacBlobs.run('readwrite', (stores, done) => {
      const row = stores.index.get(key)
      row.onsuccess = () => done(row.result?.bytes ?? 0)
      stores.blobs.delete(key)
      stores.index.delete(key)
    })
    return ['BlobRemoved', { key, bytes }]
  }

  static open(reply, key) {
    const name = TacBlobs.checked(key)
    return TacBlobs.answer(reply, 'open', name, TacBlobs.shown(name))
  }

  static async shown(key) {
    const url = TacBlobs.opened.get(key) ?? URL.createObjectURL(await TacBlobs.plainBlob(key))
    TacBlobs.opened.set(key, url)
    return ['BlobOpened', { key, url }]
  }

  static close(key) {
    const url = TacBlobs.opened.get(key)
    if (url) URL.revokeObjectURL(url)
    TacBlobs.opened.delete(key)
    return null
  }

  static upload(reply, key, method, address, headers) {
    const name = TacBlobs.checked(key)
    const finished = (answer) => ['BlobUploaded', { upload: answer.upload, key: name, ...answer }]
    const unread = (error) => TacBlobs.failed('upload', name, error)
    return TacTransfer.upload(() => TacBlobs.plainBlob(name), { method, address, headers }, reply, { finished, unread })
  }

  static run(mode, work) {
    return TacBlobs.base().then((base) => new Promise((resolve, reject) => {
      const transaction = base.transaction(TacBlobs.stores, mode)
      const [blobs, index] = TacBlobs.stores.map((name) => transaction.objectStore(name))
      let result
      work({ blobs, index }, (value) => { result = value })
      transaction.oncomplete = () => resolve(result)
      transaction.onabort = () => reject(transaction.error ?? new Error('The blob store stopped a write.'))
    }))
  }

  static base() {
    TacBlobs.connection ??= TacBlobs.connect().catch((error) => {
      TacBlobs.connection = null
      throw error
    })
    return TacBlobs.connection
  }

  static connect() {
    if (typeof globalThis.indexedDB?.open !== 'function') return Promise.reject(Object.assign(new Error('unavailable'), { blobReason: 'unavailable' }))
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(TacBlobs.database, TacBlobs.version)
      request.onupgradeneeded = () => TacBlobs.stores.forEach((name) => request.result.createObjectStore(name))
      request.onsuccess = () => resolve(TacBlobs.watched(request.result))
      request.onerror = () => reject(Object.assign(new Error('unavailable'), { blobReason: 'unavailable' }))
    })
  }

  static watched(base) {
    const forget = () => { TacBlobs.connection = null }
    base.onclose = forget
    base.onversionchange = () => {
      base.close()
      forget()
    }
    return base
  }
}

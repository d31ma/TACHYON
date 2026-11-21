class TacMediaFrames {
  static suite = 0x0004n
  static keyBits = 128
  static saltBits = 96
  static nonceBytes = 12
  static tagBits = 128
  static kidBytes = 8
  static suiteBytes = 2
  static largestCounter = 0xffffffffffffffffn
  static smallest = 8n
  static extended = 8
  static lengthMask = 7
  static keyLabel = 'SFrame 1.0 Secret key '
  static saltLabel = 'SFrame 1.0 Secret salt '
  static videoClearBytes = Object.freeze({ key: 10, delta: 3 })
  static wrapInfo = 'Tachyon SFrame frame key 1'
  static wrapVersion = 'HPKE-v1'
  static curve = Object.freeze({ name: 'ECDH', namedCurve: 'P-256' })
  static hmac = Object.freeze({ name: 'HMAC', hash: 'SHA-256' })
  static kemId = 16
  static kdfId = 1
  static aeadId = 2
  static idBytes = 2
  static hashBytes = 32
  static baseMode = 0
  static aeadKeyBytes = Object.freeze({ 1: 16, 2: 32 })
  static sealedParts = Object.freeze({ key: 0, salt: 1 })
  static pending = new Map()
  static receiving = new Map()
  static sending = null
  static installed = TacMediaFrames.install()

  static install() {
    if (typeof globalThis.WorkerGlobalScope !== 'function') return false
    globalThis.onrtctransform = (event) => TacMediaFrames.attach(event.transformer)
    globalThis.onmessage = (event) => TacMediaFrames.receive(event.data)
    return true
  }

  static receive(message) {
    const kid = BigInt(message.keyId)
    if (message.kind === 'bind') return TacMediaFrames.bind(kid).catch(() => TacMediaFrames.report('refused', kid, ''))
    if (message.kind === 'wrapped') return TacMediaFrames.unwrap(kid, message.wrapped)
    if (message.kind === 'use') return TacMediaFrames.use(kid, message.key)
    if (message.kind === 'send') return TacMediaFrames.send(kid)
    return TacMediaFrames.forget(kid)
  }

  static async use(kid, baseKey) {
    TacMediaFrames.receiving.set(kid, await TacMediaFrames.derive(baseKey, kid))
    TacMediaFrames.report('ready', kid, '')
  }

  static async bind(kid) {
    const pair = await crypto.subtle.generateKey(TacMediaFrames.curve, false, ['deriveBits'])
    TacMediaFrames.pending.set(kid, pair)
    const publicKey = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey))
    globalThis.postMessage?.({ kind: 'bound', keyId: Number(kid), publicKey: TacMediaFrames.base64(publicKey) })
  }

  static async unwrap(kid, wrapped) {
    const pair = TacMediaFrames.pending.get(kid)
    TacMediaFrames.pending.delete(kid)
    const entry = await TacMediaFrames.unwrapped(pair, kid, wrapped).catch(() => null)
    if (entry === null) return TacMediaFrames.report('refused', kid, '')
    TacMediaFrames.receiving.set(kid, entry)
    TacMediaFrames.report('ready', kid, '')
  }

  static async unwrapped(pair, kid, wrapped) {
    const own = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey))
    const kidBytes = TacMediaFrames.bigEndian(kid, TacMediaFrames.kidBytes)
    const info = TacMediaFrames.join([TacMediaFrames.text(TacMediaFrames.wrapInfo), kidBytes])
    const context = await TacMediaFrames.hpke(pair.privateKey, own, TacMediaFrames.fromBase64(wrapped.enc), info, TacMediaFrames.aeadId)
    const key = await TacMediaFrames.unwrapKey(context, kidBytes, TacMediaFrames.fromBase64(wrapped.key))
    const salt = await TacMediaFrames.openSalt(context, kidBytes, TacMediaFrames.fromBase64(wrapped.salt))
    return { kid, key, salt, counter: 0n }
  }

  static unwrapKey(context, kidBytes, sealed) {
    const aead = TacMediaFrames.opening(context, TacMediaFrames.sealedParts.key, TacMediaFrames.partData('key', kidBytes))
    return crypto.subtle.unwrapKey('raw', sealed, context.key, aead, { name: 'AES-GCM', length: TacMediaFrames.keyBits }, false, ['encrypt', 'decrypt'])
  }

  static async openSalt(context, kidBytes, sealed) {
    const aead = TacMediaFrames.opening(context, TacMediaFrames.sealedParts.salt, TacMediaFrames.partData('salt', kidBytes))
    const salt = new Uint8Array(await crypto.subtle.decrypt(aead, context.key, sealed))
    if (salt.length * 8 !== TacMediaFrames.saltBits) throw new Error('The frame salt has the wrong size.')
    return salt
  }

  static partData(part, kidBytes) {
    return TacMediaFrames.join([TacMediaFrames.text(part), kidBytes])
  }

  static async hpke(privateKey, own, enc, info, aead) {
    const kem = TacMediaFrames.join([TacMediaFrames.text('KEM'), TacMediaFrames.suiteId(TacMediaFrames.kemId)])
    const suite = TacMediaFrames.join([TacMediaFrames.text('HPKE'), ...[TacMediaFrames.kemId, TacMediaFrames.kdfId, aead].map(TacMediaFrames.suiteId)])
    const peer = await crypto.subtle.importKey('raw', enc, TacMediaFrames.curve, false, [])
    const dh = new Uint8Array(await crypto.subtle.deriveBits({ name: 'ECDH', public: peer }, privateKey, TacMediaFrames.hashBytes * 8))
    const shared = await TacMediaFrames.labeledBits(new Uint8Array(0), kem, ['eae_prk', dh], ['shared_secret', TacMediaFrames.join([enc, own])], TacMediaFrames.hashBytes)
    const schedule = TacMediaFrames.join([new Uint8Array([TacMediaFrames.baseMode]), await TacMediaFrames.extracted(suite, 'psk_id_hash', new Uint8Array(0)), await TacMediaFrames.extracted(suite, 'info_hash', info)])
    const size = TacMediaFrames.aeadKeyBytes[aead]
    const keyBase = await TacMediaFrames.labeledBase(suite, 'secret', new Uint8Array(0))
    const key = await crypto.subtle.deriveKey(TacMediaFrames.hkdfWith(shared, TacMediaFrames.labeledInfo(suite, 'key', schedule, size)), keyBase, { name: 'AES-GCM', length: size * 8 }, false, ['unwrapKey', 'decrypt'])
    const nonce = await TacMediaFrames.labeledBits(shared, suite, ['secret', new Uint8Array(0)], ['base_nonce', schedule], TacMediaFrames.nonceBytes)
    return { key, nonce }
  }

  static async labeledBits(salt, suite, [extractLabel, input], [expandLabel, context], size) {
    const base = await TacMediaFrames.labeledBase(suite, extractLabel, input)
    const params = TacMediaFrames.hkdfWith(salt, TacMediaFrames.labeledInfo(suite, expandLabel, context, size))
    return new Uint8Array(await crypto.subtle.deriveBits(params, base, size * 8))
  }

  static labeledBase(suite, label, input) {
    return crypto.subtle.importKey('raw', TacMediaFrames.labels(suite, label, input), 'HKDF', false, ['deriveBits', 'deriveKey'])
  }

  static labeledInfo(suite, label, context, size) {
    return TacMediaFrames.join([TacMediaFrames.bigEndian(size, TacMediaFrames.idBytes), TacMediaFrames.labels(suite, label, context)])
  }

  static async extracted(suite, label, value) {
    const zeros = await crypto.subtle.importKey('raw', new Uint8Array(TacMediaFrames.hashBytes), TacMediaFrames.hmac, false, ['sign'])
    return new Uint8Array(await crypto.subtle.sign('HMAC', zeros, TacMediaFrames.labels(suite, label, value)))
  }

  static labels(suite, label, value) {
    return TacMediaFrames.join([TacMediaFrames.text(TacMediaFrames.wrapVersion), suite, TacMediaFrames.text(label), value])
  }

  static suiteId(id) {
    return TacMediaFrames.bigEndian(id, TacMediaFrames.idBytes)
  }

  static opening(context, sequence, additionalData) {
    const iv = TacMediaFrames.nonce(context.nonce, BigInt(sequence))
    return { name: 'AES-GCM', iv, additionalData, tagLength: TacMediaFrames.tagBits }
  }

  static text(value) {
    return new TextEncoder().encode(value)
  }

  static base64(bytes) {
    return btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
  }

  static fromBase64(value) {
    const plain = atob(String(value).replace(/-/g, '+').replace(/_/g, '/'))
    return Uint8Array.from(plain, (letter) => letter.charCodeAt(0))
  }

  static send(kid) {
    const entry = TacMediaFrames.receiving.get(kid)
    if (!entry) return TacMediaFrames.report('missing', kid, '')
    TacMediaFrames.receiving.delete(kid)
    TacMediaFrames.sending = entry
    return entry
  }

  static forget(kid) {
    TacMediaFrames.receiving.delete(kid)
    if (TacMediaFrames.sending?.kid === kid) TacMediaFrames.sending = null
  }

  static attach(transformer) {
    const options = transformer.options ?? {}
    const transform = (frame, controller) => TacMediaFrames.frame(frame, controller, options)
    transformer.readable.pipeThrough(new TransformStream({ transform })).pipeTo(transformer.writable)
  }

  static async frame(frame, controller, options) {
    const bytes = await TacMediaFrames.change(frame, options)
    if (bytes === null) return
    frame.data = bytes.buffer
    controller.enqueue(frame)
  }

  static change(frame, options) {
    const data = new Uint8Array(frame.data)
    const clear = Math.min(TacMediaFrames.clearBytes(frame, options.kind), data.length)
    if (options.side === 'send') return TacMediaFrames.seal(data, clear)
    return TacMediaFrames.open(data, clear, options.track ?? '')
  }

  static clearBytes(frame, kind) {
    if (kind !== 'video') return 0
    return TacMediaFrames.videoClearBytes[frame.type] ?? 0
  }

  static async seal(data, clear) {
    const sender = TacMediaFrames.sending
    if (!sender) return null
    const sealed = await TacMediaFrames.encrypt(sender, data.subarray(0, clear), data.subarray(clear)).catch(() => TacMediaFrames.report('exhausted', sender.kid, ''))
    if (sealed === null) return null
    return TacMediaFrames.join([data.subarray(0, clear), sealed])
  }

  static async open(data, clear, track) {
    const metadata = data.subarray(0, clear)
    const opened = await TacMediaFrames.decrypt(metadata, data.subarray(clear), track).catch(() => TacMediaFrames.report('rejected', 0n, track))
    if (opened === null) return null
    return TacMediaFrames.join([metadata, opened])
  }

  static async decrypt(metadata, ciphertext, track = '') {
    const parsed = TacMediaFrames.parse(ciphertext)
    const entry = TacMediaFrames.receiving.get(parsed.kid)
    if (!entry) return TacMediaFrames.report('missing', parsed.kid, track)
    const header = ciphertext.subarray(0, parsed.length)
    const plain = await crypto.subtle.decrypt(TacMediaFrames.aead(entry.salt, parsed.ctr, header, metadata), entry.key, ciphertext.subarray(parsed.length))
    return new Uint8Array(plain)
  }

  static report(kind, kid, track) {
    globalThis.postMessage?.({ kind, keyId: Number(kid), track })
    return null
  }

  static async encrypt(entry, metadata, plaintext) {
    const ctr = TacMediaFrames.nextCounter(entry)
    const header = TacMediaFrames.header(entry.kid, ctr)
    const sealed = await crypto.subtle.encrypt(TacMediaFrames.aead(entry.salt, ctr, header, metadata), entry.key, plaintext)
    return TacMediaFrames.join([header, new Uint8Array(sealed)])
  }

  static nextCounter(entry) {
    const ctr = entry.counter
    if (ctr > TacMediaFrames.largestCounter) throw new Error('The frame key has used every counter value; send a new key.')
    entry.counter = ctr + 1n
    return ctr
  }

  static aead(salt, ctr, header, metadata) {
    const iv = TacMediaFrames.nonce(salt, ctr)
    return { name: 'AES-GCM', iv, additionalData: TacMediaFrames.join([header, metadata]), tagLength: TacMediaFrames.tagBits }
  }

  static async derive(baseKey, kid) {
    const keyInfo = TacMediaFrames.label(TacMediaFrames.keyLabel, kid)
    const saltInfo = TacMediaFrames.label(TacMediaFrames.saltLabel, kid)
    const key = await crypto.subtle.deriveKey(TacMediaFrames.hkdf(keyInfo), baseKey, { name: 'AES-GCM', length: TacMediaFrames.keyBits }, false, ['encrypt', 'decrypt'])
    const salt = new Uint8Array(await crypto.subtle.deriveBits(TacMediaFrames.hkdf(saltInfo), baseKey, TacMediaFrames.saltBits))
    return { kid, key, salt, counter: 0n }
  }

  static hkdf(info) {
    return TacMediaFrames.hkdfWith(new Uint8Array(0), info)
  }

  static hkdfWith(salt, info) {
    return { name: 'HKDF', hash: 'SHA-256', salt, info }
  }

  static label(text, kid) {
    const kidBytes = TacMediaFrames.bigEndian(kid, TacMediaFrames.kidBytes)
    const suiteBytes = TacMediaFrames.bigEndian(TacMediaFrames.suite, TacMediaFrames.suiteBytes)
    return TacMediaFrames.join([TacMediaFrames.text(text), kidBytes, suiteBytes])
  }

  static nonce(salt, ctr) {
    const counter = TacMediaFrames.bigEndian(ctr, TacMediaFrames.nonceBytes)
    return salt.map((byte, index) => byte ^ counter[index])
  }

  static header(kid, ctr) {
    const kidField = TacMediaFrames.field(kid)
    const ctrField = TacMediaFrames.field(ctr)
    const config = new Uint8Array([(kidField.bits << 4) | ctrField.bits])
    return TacMediaFrames.join([config, kidField.bytes, ctrField.bytes])
  }

  static field(value) {
    if (value < TacMediaFrames.smallest) return { bits: Number(value), bytes: new Uint8Array(0) }
    const bytes = TacMediaFrames.compact(value)
    return { bits: TacMediaFrames.extended | (bytes.length - 1), bytes }
  }

  static compact(value) {
    const bytes = []
    for (let rest = value; rest > 0n; rest >>= 8n) bytes.unshift(Number(rest & 0xffn))
    return new Uint8Array(bytes)
  }

  static parse(bytes) {
    const config = TacMediaFrames.byteAt(bytes, 0)
    const kid = TacMediaFrames.readField(bytes, config >> 4, 1)
    const ctr = TacMediaFrames.readField(bytes, config & 0x0f, 1 + kid.length)
    return { kid: kid.value, ctr: ctr.value, length: 1 + kid.length + ctr.length }
  }

  static readField(bytes, bits, offset) {
    if ((bits & TacMediaFrames.extended) === 0) return { value: BigInt(bits), length: 0 }
    const length = (bits & TacMediaFrames.lengthMask) + 1
    TacMediaFrames.byteAt(bytes, offset + length - 1)
    return { value: TacMediaFrames.fromBytes(bytes.subarray(offset, offset + length)), length }
  }

  static byteAt(bytes, index) {
    if (index >= bytes.length) throw new Error('The SFrame header is cut short.')
    return bytes[index]
  }

  static fromBytes(bytes) {
    let value = 0n
    for (const byte of bytes) value = (value << 8n) | BigInt(byte)
    return value
  }

  static bigEndian(value, size) {
    const bytes = new Uint8Array(size)
    let rest = BigInt(value)
    for (let index = size - 1; index >= 0; index -= 1) {
      bytes[index] = Number(rest & 0xffn)
      rest >>= 8n
    }
    return bytes
  }

  static join(parts) {
    let size = 0
    for (const part of parts) size += part.length
    const joined = new Uint8Array(size)
    let offset = 0
    for (const part of parts) {
      joined.set(part, offset)
      offset += part.length
    }
    return joined
  }
}

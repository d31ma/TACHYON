class TacCrypto {
  static key = Symbol.for('tachyon.tac.crypto')
  static hooksKey = Symbol.for('tachyon.tac.crypto.hooks')
  static api = 'crypto'
  static databaseName = 'tachyon-keys'
  static storeName = 'keys'
  static mostBytes = 32768
  static sealedExtraBytes = 29
  static nonceBytes = 12
  static version = 1
  static keyBytes = 32
  static keyBits = 256
  static signatureBytes = 64
  static leastSaltBytes = 16
  static mostCodeBytes = 1024
  static mostDerivedBytes = 1024
  static rounds = 600000
  static base64url = /^[A-Za-z0-9_-]*$/
  static keyName = /^[A-Za-z0-9_.-]{1,64}$/
  static prfKeyName = /^[A-Za-z0-9_.-]{1,57}$/
  static secondSuffix = '.second'
  static algorithms = ['x25519', 'ed25519']
  static uses = ['encrypt', 'derive']
  static knownReasons = ['unsupported', 'invalid', 'unknown', 'exists', 'rejected', 'locked', 'failed']
  static webNames = Object.freeze({ x25519: 'X25519', ed25519: 'Ed25519' })
  static pairUsages = Object.freeze({ x25519: ['deriveBits'], ed25519: ['sign'] })
  static secretAlgorithms = Object.freeze({ encrypt: { name: 'AES-GCM' }, derive: { name: 'HKDF' } })
  static secretUsages = Object.freeze({ encrypt: ['encrypt', 'decrypt'], derive: ['deriveKey', 'deriveBits'] })
  static reasons = Object.freeze({
    NotSupportedError: 'unsupported',
    DataError: 'invalid',
    InvalidAccessError: 'invalid',
    SyntaxError: 'invalid',
    ConstraintError: 'exists',
  })
  static messages = Object.freeze({
    unsupported: 'This page cannot run this crypto call: the algorithm or the key store is missing here.',
    invalid: 'The key or the input does not fit this crypto call.',
    unknown: 'No key with this name is on this device.',
    exists: 'A key with this name already exists. Forget it first.',
    rejected: 'The sealed bytes do not open with this key and AAD.',
    locked: 'The key store is locked.',
    failed: 'The crypto call failed.',
  })
  static events = Object.freeze({
    check: 'CryptoSupport',
    makeKeyPair: 'CryptoKeyMade',
    publicKey: 'CryptoKeyMade',
    sign: 'CryptoSigned',
    verify: 'CryptoVerified',
    agree: 'CryptoKeyMade',
    deriveKey: 'CryptoKeyMade',
    importKey: 'CryptoKeyMade',
    fromCode: 'CryptoKeyMade',
    encrypt: 'CryptoEncrypted',
    decrypt: 'CryptoDecrypted',
    hkdf: 'CryptoDerived',
    hash: 'CryptoHashed',
    forget: 'CryptoForgotten',
    forgetAll: 'CryptoForgotten',
  })
  static checked = Object.freeze({
    check: () => [],
    makeKeyPair: (key, algorithm, keep) => [TacCrypto.keyText(key), TacCrypto.choice(algorithm, TacCrypto.algorithms, 'The algorithm'), TacCrypto.flag(keep)],
    publicKey: (key) => [TacCrypto.keyText(key)],
    sign: (key, data) => [TacCrypto.keyText(key), TacCrypto.bytes(data)],
    verify: (publicKey, data, signature) => [TacCrypto.sized(publicKey, TacCrypto.keyBytes, 'A public key'), TacCrypto.bytes(data), TacCrypto.bytes(signature)],
    agree: (key, peerPublicKey, salt, info, newKey, use, keep) => [TacCrypto.keyText(key), TacCrypto.sized(peerPublicKey, TacCrypto.keyBytes, 'A public key'), TacCrypto.bytes(salt), TacCrypto.bytes(info), ...TacCrypto.newKey(newKey, use, keep)],
    deriveKey: (fromKey, salt, info, newKey, use, keep) => [TacCrypto.keyText(fromKey), TacCrypto.bytes(salt), TacCrypto.bytes(info), ...TacCrypto.newKey(newKey, use, keep)],
    importKey: (newKey, secret, use, keep) => [TacCrypto.keyText(newKey), TacCrypto.sized(secret, TacCrypto.keyBytes, 'A secret'), TacCrypto.choice(use, TacCrypto.uses, 'The use'), TacCrypto.flag(keep)],
    fromCode: (newKey, code, salt, use, keep) => [TacCrypto.keyText(newKey), TacCrypto.code(code), TacCrypto.salt(salt), TacCrypto.choice(use, TacCrypto.uses, 'The use'), TacCrypto.flag(keep)],
    encrypt: (key, data, aad) => [TacCrypto.keyText(key), TacCrypto.bytes(data), TacCrypto.bytes(aad)],
    decrypt: (key, sealed, aad) => [TacCrypto.keyText(key), TacCrypto.bytes(sealed, TacCrypto.mostBytes + TacCrypto.sealedExtraBytes), TacCrypto.bytes(aad)],
    hkdf: (input, salt, info, length) => [TacCrypto.bytes(input), TacCrypto.bytes(salt), TacCrypto.bytes(info), TacCrypto.length(length)],
    hash: (data) => [TacCrypto.bytes(data)],
    forget: (key) => [TacCrypto.keyText(key)],
    forgetAll: () => [],
  })
  static work = Object.freeze({
    check: () => TacCrypto.support(),
    makeKeyPair: (...values) => TacCrypto.makePair(...values),
    publicKey: (key) => TacCrypto.publicOf(key),
    sign: (key, data) => TacCrypto.signed(key, data),
    verify: (publicKey, data, signature) => TacCrypto.verified(publicKey, data, signature),
    agree: (...values) => TacCrypto.agreed(...values),
    deriveKey: (...values) => TacCrypto.derivedFrom(...values),
    importKey: (...values) => TacCrypto.imported(...values),
    fromCode: (...values) => TacCrypto.fromCodeText(...values),
    encrypt: (key, data, aad) => TacCrypto.sealed(key, data, aad),
    decrypt: (key, sealed, aad) => TacCrypto.unsealed(key, sealed, aad),
    hkdf: (...values) => TacCrypto.expanded(...values),
    hash: (data) => TacCrypto.hashed(data),
    forget: (key) => TacCrypto.forgotten(key),
    forgetAll: () => TacCrypto.forgottenAll(),
  })
  static memory = new Map()
  static database = null
  static verifierPath = '/.tachyon/tac-ed25519.js'
  static verifier = null
  static counter = 0
  static queue = Promise.resolve()
  static installed = TacCrypto.install()

  static install() {
    const calls = Object.keys(TacCrypto.checked).map((call) => [call, (reply, ...values) => TacCrypto.start(reply, call, TacCrypto.checked[call](...values))])
    Object.defineProperty(globalThis, TacCrypto.key, { configurable: true, value: Object.freeze({ calls: Object.freeze(Object.keys(TacCrypto.checked)), ...Object.fromEntries(calls) }) })
    Object.defineProperty(globalThis, TacCrypto.hooksKey, { configurable: true, value: Object.freeze({ key: (name) => TacCrypto.hookKey(name), prfName: (name) => TacCrypto.prfName(name), prf: (name, results) => TacCrypto.prfKeys(name, results), frameKey: (name, keyId, publicKey) => TacCrypto.hookFrameKey(name, keyId, publicKey) }) })
    return true
  }

  static start(reply, call, values) {
    TacCrypto.counter += 1
    const request = `crypto-${TacCrypto.counter}`
    const running = TacCrypto.queue.then(() => TacCrypto.run(call, values))
    TacCrypto.queue = running.catch(() => null)
    running.then((fields) => reply([TacCrypto.events[call], { ...fields, request }]), (error) => reply(TacCrypto.failure(request, call, error)))
    return request
  }

  static run(call, values) {
    return TacCrypto.bridged() ? TacCrypto.hosted(call, values) : TacCrypto.work[call](...values)
  }

  static failure(request, call, error) {
    const reason = TacCrypto.reasonOf(error)
    return ['CryptoFailed', { request, call, reason, message: TacCrypto.messages[reason] }]
  }

  static reasonOf(error) {
    return error?.reason ?? TacCrypto.reasons[String(error?.name ?? '')] ?? 'failed'
  }

  static refusal(reason) {
    const error = new Error(reason)
    error.reason = TacCrypto.knownReasons.includes(reason) ? reason : 'failed'
    return error
  }

  static keyText(value) {
    const text = String(value ?? '')
    if (!TacCrypto.keyName.test(text)) throw new Error('A key name is 1 to 64 letters, digits, dots, dashes or underscores.')
    return text
  }

  static choice(value, allowed, what) {
    if (!allowed.includes(value)) throw new Error(`${what} is one of: ${allowed.join(', ')}.`)
    return value
  }

  static flag(value) {
    if (typeof value !== 'boolean') throw new Error('keep is true or false.')
    return value
  }

  static newKey(newKey, use, keep) {
    return [TacCrypto.keyText(newKey), TacCrypto.choice(use, TacCrypto.uses, 'The use'), TacCrypto.flag(keep)]
  }

  static bytes(value, most = TacCrypto.mostBytes) {
    const text = String(value ?? '')
    if (!TacCrypto.base64url.test(text) || text.length % 4 === 1) throw new Error('Crypto bytes are base64url text without padding.')
    if (Math.floor(text.length * 3 / 4) > most) throw new Error(`Crypto bytes are at most ${most} bytes here.`)
    return text
  }

  static sized(value, size, what) {
    const text = TacCrypto.bytes(value)
    if (Math.floor(text.length * 3 / 4) !== size) throw new Error(`${what} is ${size} bytes.`)
    return text
  }

  static salt(value) {
    const text = TacCrypto.bytes(value)
    if (Math.floor(text.length * 3 / 4) < TacCrypto.leastSaltBytes) throw new Error(`The salt is at least ${TacCrypto.leastSaltBytes} bytes.`)
    return text
  }

  static code(value) {
    const text = String(value ?? '')
    const size = new TextEncoder().encode(text).length
    if (size < 1 || size > TacCrypto.mostCodeBytes) throw new Error(`The code is 1 to ${TacCrypto.mostCodeBytes} bytes of text.`)
    return text
  }

  static length(value) {
    if (!Number.isInteger(value) || value < 1 || value > TacCrypto.mostDerivedBytes) throw new Error(`The length is a whole number from 1 to ${TacCrypto.mostDerivedBytes}.`)
    return value
  }

  static decoded(text) {
    const binary = atob(text.replace(/-/g, '+').replace(/_/g, '/'))
    return Uint8Array.from(binary, (letter) => letter.charCodeAt(0))
  }

  static encoded(buffer) {
    const binary = Array.from(new Uint8Array(buffer), (byte) => String.fromCharCode(byte)).join('')
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
  }

  static subtle() {
    return globalThis.isSecureContext === true ? globalThis.crypto?.subtle ?? null : null
  }

  static ensureSubtle() {
    const subtle = TacCrypto.subtle()
    if (!subtle) throw TacCrypto.refusal('unsupported')
    return subtle
  }

  static async support() {
    if (!TacCrypto.subtle()) return { x25519: false, ed25519: false, aesGcm: false, hkdf: false, pbkdf2: false, keyStore: 'none' }
    const subtle = TacCrypto.subtle()
    const raw = new Uint8Array(TacCrypto.keyBytes)
    const probes = [
      () => subtle.generateKey({ name: 'X25519' }, false, ['deriveBits']),
      () => subtle.generateKey({ name: 'Ed25519' }, false, ['sign', 'verify']),
      () => subtle.importKey('raw', raw, { name: 'AES-GCM' }, false, ['encrypt']),
      () => subtle.importKey('raw', raw, 'HKDF', false, ['deriveBits']),
      () => subtle.importKey('raw', raw, 'PBKDF2', false, ['deriveBits']),
    ]
    const [x25519, ed25519, aesGcm, hkdf, pbkdf2] = await Promise.all(probes.map((probe) => TacCrypto.works(probe)))
    return { x25519, ed25519, aesGcm, hkdf, pbkdf2, keyStore: await TacCrypto.storeKind() }
  }

  static works(probe) {
    return Promise.resolve().then(probe).then(() => true, () => false)
  }

  static async storeKind() {
    const database = await TacCrypto.opened().catch(() => null)
    return database ? 'browser' : 'none'
  }

  static async makePair(key, algorithm, keep) {
    const subtle = TacCrypto.ensureSubtle()
    await TacCrypto.ensureFree(key)
    const pair = await subtle.generateKey({ name: TacCrypto.webNames[algorithm] }, false, TacCrypto.pairUsages[algorithm])
    const publicKey = new Uint8Array(await subtle.exportKey('raw', pair.publicKey))
    await TacCrypto.save({ name: key, algorithm, key: pair.privateKey, publicKey }, keep)
    return { key, publicKey: TacCrypto.encoded(publicKey) }
  }

  static async publicOf(key) {
    const entry = await TacCrypto.found(key)
    if (!entry.publicKey) throw TacCrypto.refusal('invalid')
    return { key, publicKey: TacCrypto.encoded(entry.publicKey) }
  }

  static async signed(key, data) {
    const subtle = TacCrypto.ensureSubtle()
    const entry = TacCrypto.ofKind(await TacCrypto.found(key), 'ed25519')
    const signature = await subtle.sign({ name: 'Ed25519' }, entry.key, TacCrypto.decoded(data))
    return { signature: TacCrypto.encoded(signature) }
  }

  static async verified(publicKey, data, signature) {
    const valid = await TacCrypto.webVerified(publicKey, data, signature)
    return { valid: valid || await TacCrypto.emptyVerified(publicKey, data, signature) }
  }

  static async webVerified(publicKey, data, signature) {
    const subtle = TacCrypto.ensureSubtle()
    const signer = await subtle.importKey('raw', TacCrypto.decoded(publicKey), { name: 'Ed25519' }, true, ['verify'])
    return await subtle.verify({ name: 'Ed25519' }, signer, TacCrypto.decoded(signature), TacCrypto.decoded(data)) === true
  }

  static async emptyVerified(publicKey, data, signature) {
    if (data !== '') return false
    const ed25519 = await TacCrypto.ed25519()
    return TacCrypto.strictlyVerified(ed25519, TacCrypto.decoded(publicKey), TacCrypto.decoded(signature))
  }

  static ed25519() {
    TacCrypto.verifier ??= import(TacCrypto.verifierPath)
    TacCrypto.verifier.catch(() => { TacCrypto.verifier = null })
    return TacCrypto.verifier
  }

  static async strictlyVerified(ed25519, publicKey, signature) {
    if (signature.length !== TacCrypto.signatureBytes || TacCrypto.smallOrder(ed25519, signature.subarray(0, TacCrypto.keyBytes))) return false
    return await ed25519.verifyAsync(signature, new Uint8Array(0), publicKey, { zip215: false }) === true
  }

  static smallOrder(ed25519, encoded) {
    try {
      return ed25519.Point.fromBytes(encoded, false).isSmallOrder()
    } catch {
      return true
    }
  }

  static async agreed(key, peerPublicKey, salt, info, newKey, use, keep) {
    const subtle = TacCrypto.ensureSubtle()
    const entry = TacCrypto.ofKind(await TacCrypto.found(key), 'x25519')
    await TacCrypto.ensureFree(newKey)
    const peer = await subtle.importKey('raw', TacCrypto.decoded(peerPublicKey), { name: 'X25519' }, true, [])
    const agreement = subtle.deriveBits({ name: 'X25519', public: peer }, entry.key, TacCrypto.keyBits)
    const shared = new Uint8Array(await agreement.catch(() => { throw TacCrypto.refusal('invalid') }))
    const base = await TacCrypto.secret(TacCrypto.nonZero(shared), 'derive')
    return TacCrypto.expandedKey(base, salt, info, newKey, use, keep)
  }

  static nonZero(shared) {
    if (shared.every((byte) => byte === 0)) throw TacCrypto.refusal('invalid')
    return shared
  }

  static async derivedFrom(fromKey, salt, info, newKey, use, keep) {
    TacCrypto.ensureSubtle()
    const entry = TacCrypto.ofKind(await TacCrypto.found(fromKey), 'derive')
    await TacCrypto.ensureFree(newKey)
    return TacCrypto.expandedKey(entry.key, salt, info, newKey, use, keep)
  }

  static async expandedKey(base, salt, info, newKey, use, keep) {
    const parameters = { name: 'HKDF', hash: 'SHA-256', salt: TacCrypto.decoded(salt), info: TacCrypto.decoded(info) }
    const bits = new Uint8Array(await TacCrypto.subtle().deriveBits(parameters, base, TacCrypto.keyBits))
    return TacCrypto.stored(newKey, bits, use, keep)
  }

  static async imported(newKey, secret, use, keep) {
    TacCrypto.ensureSubtle()
    await TacCrypto.ensureFree(newKey)
    return TacCrypto.stored(newKey, TacCrypto.decoded(secret), use, keep)
  }

  static async fromCodeText(newKey, code, salt, use, keep) {
    const subtle = TacCrypto.ensureSubtle()
    await TacCrypto.ensureFree(newKey)
    const base = await subtle.importKey('raw', new TextEncoder().encode(code), 'PBKDF2', false, ['deriveBits'])
    const parameters = { name: 'PBKDF2', hash: 'SHA-256', salt: TacCrypto.decoded(salt), iterations: TacCrypto.rounds }
    const bits = new Uint8Array(await subtle.deriveBits(parameters, base, TacCrypto.keyBits))
    return TacCrypto.stored(newKey, bits, use, keep)
  }

  static async stored(newKey, bits, use, keep) {
    const key = await TacCrypto.secret(bits, use)
    await TacCrypto.save({ name: newKey, algorithm: use, key, publicKey: null }, keep)
    return { key: newKey, publicKey: '' }
  }

  static async secret(bits, use) {
    try {
      return await TacCrypto.subtle().importKey('raw', bits, TacCrypto.secretAlgorithms[use], false, TacCrypto.secretUsages[use])
    } finally {
      bits.fill(0)
    }
  }

  static async sealed(key, data, aad) {
    const subtle = TacCrypto.ensureSubtle()
    const entry = TacCrypto.ofKind(await TacCrypto.found(key), 'encrypt')
    const nonce = globalThis.crypto.getRandomValues(new Uint8Array(TacCrypto.nonceBytes))
    const parameters = { name: 'AES-GCM', iv: nonce, additionalData: TacCrypto.decoded(aad), tagLength: 128 }
    const cipher = new Uint8Array(await subtle.encrypt(parameters, entry.key, TacCrypto.decoded(data)))
    const sealed = new Uint8Array(1 + nonce.length + cipher.length)
    sealed.set([TacCrypto.version], 0)
    sealed.set(nonce, 1)
    sealed.set(cipher, 1 + nonce.length)
    return { sealed: TacCrypto.encoded(sealed) }
  }

  static async unsealed(key, sealed, aad) {
    TacCrypto.ensureSubtle()
    const entry = TacCrypto.ofKind(await TacCrypto.found(key), 'encrypt')
    const bytes = TacCrypto.sealedBytes(sealed)
    const nonce = bytes.subarray(1, 1 + TacCrypto.nonceBytes)
    const parameters = { name: 'AES-GCM', iv: nonce, additionalData: TacCrypto.decoded(aad), tagLength: 128 }
    const data = await TacCrypto.subtle().decrypt(parameters, entry.key, bytes.subarray(1 + TacCrypto.nonceBytes)).catch(() => { throw TacCrypto.refusal('rejected') })
    return { data: TacCrypto.encoded(data) }
  }

  static sealedBytes(sealed) {
    const bytes = TacCrypto.decoded(sealed)
    if (bytes.length < TacCrypto.sealedExtraBytes || bytes[0] !== TacCrypto.version) throw TacCrypto.refusal('rejected')
    return bytes
  }

  static async expanded(input, salt, info, length) {
    const subtle = TacCrypto.ensureSubtle()
    const base = await subtle.importKey('raw', TacCrypto.decoded(input), 'HKDF', false, ['deriveBits'])
    const parameters = { name: 'HKDF', hash: 'SHA-256', salt: TacCrypto.decoded(salt), info: TacCrypto.decoded(info) }
    const bytes = await subtle.deriveBits(parameters, base, length * 8)
    return { bytes: TacCrypto.encoded(bytes) }
  }

  static async hashed(data) {
    const digest = await TacCrypto.ensureSubtle().digest('SHA-256', TacCrypto.decoded(data))
    return { digest: TacCrypto.encoded(digest) }
  }

  static async forgotten(key) {
    TacCrypto.memory.delete(key)
    await TacCrypto.stores('readwrite', (store) => store.delete(key))
    return {}
  }

  static async forgottenAll() {
    TacCrypto.memory.clear()
    await TacCrypto.stores('readwrite', (store) => store.clear())
    return {}
  }

  static ofKind(entry, algorithm) {
    if (entry.algorithm !== algorithm) throw TacCrypto.refusal('invalid')
    return entry
  }

  static async found(name) {
    const entry = await TacCrypto.lookup(name)
    if (!entry) throw TacCrypto.refusal('unknown')
    return entry
  }

  static async lookup(name) {
    return TacCrypto.memory.get(name) ?? await TacCrypto.stores('readonly', (store) => store.get(name)) ?? null
  }

  static async ensureFree(name) {
    if (await TacCrypto.lookup(name)) throw TacCrypto.refusal('exists')
  }

  static async save(record, keep) {
    if (!keep) return TacCrypto.memory.set(record.name, record)
    const database = await TacCrypto.opened().catch(() => null)
    if (!database) throw TacCrypto.refusal('unsupported')
    return TacCrypto.stores('readwrite', (store) => store.add(record))
  }

  static prfName(value) {
    if (typeof value !== 'string' || !TacCrypto.prfKeyName.test(value)) throw new Error(`A PRF key name is 1 to 57 letters, digits, dots, dashes or underscores, so that the name with ${TacCrypto.secondSuffix} is a key name too.`)
    return value
  }

  static prfKeys(name, results) {
    const running = TacCrypto.queue.then(() => TacCrypto.prfStored(TacCrypto.prfName(name), results))
    TacCrypto.queue = running.catch(() => null)
    return running
  }

  static async prfStored(name, { first, second }) {
    const secondName = second ? `${name}${TacCrypto.secondSuffix}` : ''
    try {
      await TacCrypto.ensurePrfFree(name, secondName)
      await TacCrypto.stored(name, first, 'derive', false)
      if (second) await TacCrypto.stored(secondName, second, 'derive', false)
      return { prfKey: name, prfSecondKey: secondName }
    } finally {
      first.fill(0)
      second?.fill(0)
    }
  }

  static async ensurePrfFree(name, secondName) {
    if (TacCrypto.bridged()) throw TacCrypto.refusal('unsupported')
    TacCrypto.ensureSubtle()
    await TacCrypto.ensureFree(name)
    if (secondName) await TacCrypto.ensureFree(secondName)
  }

  static async hookKey(name) {
    await TacCrypto.queue
    const entry = await TacCrypto.lookup(String(name)).catch(() => null)
    return TacCrypto.uses.includes(entry?.algorithm) ? entry.key : null
  }

  static async hookFrameKey(name, keyId, publicKey) {
    await TacCrypto.queue
    return TacCrypto.hosted('frameKey', [String(name), Number(keyId), String(publicKey)])
  }

  static opened() {
    TacCrypto.database ??= TacCrypto.openDatabase()
    TacCrypto.database.catch(() => { TacCrypto.database = null })
    return TacCrypto.database
  }

  static openDatabase() {
    if (typeof globalThis.indexedDB?.open !== 'function') return Promise.resolve(null)
    return new Promise((resolve, reject) => {
      const request = globalThis.indexedDB.open(TacCrypto.databaseName, 1)
      request.onupgradeneeded = () => request.result.createObjectStore(TacCrypto.storeName, { keyPath: 'name' })
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }

  static async stores(mode, act) {
    const database = await TacCrypto.opened().catch(() => null)
    return database ? TacCrypto.transaction(database, mode, act) : null
  }

  static transaction(database, mode, act) {
    return new Promise((resolve, reject) => {
      const transaction = database.transaction(TacCrypto.storeName, mode)
      const request = act(transaction.objectStore(TacCrypto.storeName))
      transaction.oncomplete = () => resolve(request.result ?? null)
      transaction.onerror = () => reject(transaction.error ?? request.error)
      transaction.onabort = () => reject(transaction.error ?? request.error)
    })
  }

  static bridged() {
    return typeof globalThis.__tachyonNativeHostCall === 'function' && globalThis.__tachyonBrowserApiBridge?.includes(TacCrypto.api) === true
  }

  static async hosted(call, values) {
    const raw = await globalThis.__tachyonNativeHostCall(`browserApi.${TacCrypto.api}.${call}`, JSON.stringify(values))
    const reply = typeof raw === 'string' ? JSON.parse(raw) : raw
    return TacCrypto.hostValue(reply?.value)
  }

  static hostValue(value) {
    if (value === null || typeof value !== 'object') throw TacCrypto.refusal('failed')
    if ('reason' in value) throw TacCrypto.refusal(String(value.reason))
    return value
  }
}

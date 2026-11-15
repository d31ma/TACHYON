class TacPasskeys {
  static key = Symbol.for('tachyon.tac.passkeys')
  static webKey = Symbol.for('tachyon.tac.web')
  static cryptoHooksKey = Symbol.for('tachyon.tac.crypto.hooks')
  static noPrfKeys = Object.freeze({ prfKey: '', prfSecondKey: '' })
  static mostOptionBytes = 65536
  static base64url = /^[A-Za-z0-9_-]*={0,2}$/
  static errorName = /^[A-Za-z]{1,40}$/
  static reasons = Object.freeze({
    NotAllowedError: 'cancelled',
    AbortError: 'cancelled',
    InvalidStateError: 'exists',
    NotSupportedError: 'unsupported',
    SecurityError: 'refused',
    TypeError: 'invalid',
    SyntaxError: 'invalid',
    EncodingError: 'invalid',
    KeyExistsError: 'keyExists',
  })
  static messages = Object.freeze({
    cancelled: 'The person closed the passkey prompt, or chose no passkey.',
    exists: 'A passkey in excludeCredentials is already on this device.',
    unsupported: 'This page cannot use passkeys.',
    refused: 'The relying party does not match this page.',
    busy: 'Another passkey prompt is already open.',
    invalid: 'The passkey options are not WebAuthn JSON of at most 65536 bytes.',
    keyExists: 'A Crypto key with the PRF key name is already on this device. Forget it first.',
    failed: 'The passkey call failed.',
  })
  static pending = null
  static installed = TacPasskeys.install()

  static install() {
    Object.defineProperty(globalThis, TacPasskeys.key, {
      configurable: true,
      value: Object.freeze({
        check: () => TacPasskeys.check().catch((error) => TacPasskeys.failed('check', error)),
        create: (options, prfKey) => TacPasskeys.ceremony('create', options, prfKey),
        get: (options, prfKey) => TacPasskeys.ceremony('get', options, prfKey),
        offer: (options, prfKey) => TacPasskeys.ceremony('offer', options, prfKey),
        cancel: () => TacPasskeys.stop(),
      }),
    })
    return true
  }

  static usable() {
    return globalThis[TacPasskeys.webKey]?.passkeys.supported() === true
  }

  static async check() {
    if (!TacPasskeys.usable()) return ['PasskeySupport', { platform: false, conditional: false }]
    const capabilities = await PublicKeyCredential.getClientCapabilities?.() ?? {}
    const platform = capabilities.userVerifyingPlatformAuthenticator ?? await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable()
    const conditional = capabilities.conditionalGet ?? await PublicKeyCredential.isConditionalMediationAvailable?.()
    return ['PasskeySupport', { platform: platform === true, conditional: conditional === true }]
  }

  static ceremony(call, text, prfKey = '') {
    if (!TacPasskeys.usable()) return TacPasskeys.failure(call, 'unsupported')
    if (TacPasskeys.pending && TacPasskeys.pending.call !== 'offer') return TacPasskeys.failure(call, 'busy')
    return TacPasskeys.begin(call, text, prfKey)
  }

  static begin(call, text, prfKey) {
    try {
      return TacPasskeys.ask(call, TacPasskeys.options(call, text), prfKey)
    } catch (error) {
      return TacPasskeys.failed(call, error)
    }
  }

  static options(call, text) {
    const json = TacPasskeys.parsed(String(text))
    return call === 'create' ? TacPasskeys.creation(json) : TacPasskeys.request(json)
  }

  static parsed(text) {
    if (new TextEncoder().encode(text).length > TacPasskeys.mostOptionBytes) throw new TypeError('too large')
    const json = JSON.parse(text)
    if (json === null || typeof json !== 'object' || Array.isArray(json)) throw new TypeError('not an object')
    return json
  }

  static creation(json) {
    if (typeof PublicKeyCredential.parseCreationOptionsFromJSON === 'function') return PublicKeyCredential.parseCreationOptionsFromJSON(json)
    return TacPasskeys.decodedCreation(json)
  }

  static request(json) {
    if (typeof PublicKeyCredential.parseRequestOptionsFromJSON === 'function') return PublicKeyCredential.parseRequestOptionsFromJSON(json)
    return TacPasskeys.decodedRequest(json)
  }

  static decodedCreation(json) {
    const user = { ...json.user, id: TacPasskeys.bytes(json.user?.id) }
    const excludeCredentials = json.excludeCredentials?.map((credential) => TacPasskeys.descriptor(credential))
    return { ...json, challenge: TacPasskeys.bytes(json.challenge), user, excludeCredentials, extensions: TacPasskeys.decodedExtensions(json.extensions) }
  }

  static decodedRequest(json) {
    const allowCredentials = json.allowCredentials?.map((credential) => TacPasskeys.descriptor(credential))
    return { ...json, challenge: TacPasskeys.bytes(json.challenge), allowCredentials, extensions: TacPasskeys.decodedExtensions(json.extensions) }
  }

  static decodedExtensions(extensions) {
    if (extensions?.prf === undefined) return extensions
    return { ...extensions, prf: TacPasskeys.decodedPrf(extensions.prf) }
  }

  static decodedPrf(prf) {
    const decoded = { ...prf }
    if (prf.eval !== undefined) decoded.eval = TacPasskeys.prfInputs(prf.eval)
    if (prf.evalByCredential !== undefined) decoded.evalByCredential = TacPasskeys.prfInputsByCredential(prf.evalByCredential)
    return decoded
  }

  static prfInputsByCredential(inputs) {
    return Object.fromEntries(Object.entries(inputs).map(([id, values]) => [id, TacPasskeys.prfInputs(values)]))
  }

  static prfInputs(values) {
    const decoded = { first: TacPasskeys.bytes(values?.first) }
    if (values.second !== undefined) decoded.second = TacPasskeys.bytes(values.second)
    return decoded
  }

  static descriptor(credential) {
    return { ...credential, id: TacPasskeys.bytes(credential.id) }
  }

  static bytes(text) {
    if (!TacPasskeys.isBase64url(text)) throw new TypeError('not base64url')
    const binary = atob(text.replace(/=+$/, '').replace(/-/g, '+').replace(/_/g, '/'))
    return Uint8Array.from(binary, (letter) => letter.charCodeAt(0))
  }

  static isBase64url(text) {
    return typeof text === 'string' && TacPasskeys.base64url.test(text) && text.replace(/=+$/, '').length % 4 !== 1
  }

  static ask(call, publicKey, prfKey) {
    TacPasskeys.pending?.controller.abort()
    const pending = { call, controller: new AbortController() }
    TacPasskeys.pending = pending
    const asking = call === 'offer' ? TacPasskeys.offered(pending, publicKey) : TacPasskeys.modal(call, { publicKey, signal: pending.controller.signal })
    return asking.then((credential) => TacPasskeys.answered(call, credential, prfKey)).catch((error) => TacPasskeys.refused(pending, error)).finally(() => TacPasskeys.release(pending))
  }

  static modal(call, options) {
    return call === 'create' ? navigator.credentials.create(options) : navigator.credentials.get(options)
  }

  static async offered(pending, publicKey) {
    if (await PublicKeyCredential.isConditionalMediationAvailable?.() !== true) throw new DOMException('', 'NotSupportedError')
    pending.controller.signal.throwIfAborted()
    return navigator.credentials.get({ publicKey, mediation: 'conditional', signal: pending.controller.signal })
  }

  static async answered(call, credential, prfKey) {
    if (!credential) return TacPasskeys.failure(call, 'cancelled')
    const credentialJson = JSON.stringify(TacPasskeys.withoutPrfResults(TacPasskeys.json(credential)))
    const keys = await TacPasskeys.prfKeys(prfKey, credential.getClientExtensionResults?.().prf?.results)
    if (call === 'create') return ['PasskeyCreated', { credentialJson, ...keys }]
    return ['PasskeyAsserted', { credentialJson, conditional: call === 'offer', ...keys }]
  }

  static prfKeys(prfKey, results) {
    if (prfKey === '' || !results?.first) return TacPasskeys.noPrfKeys
    const second = results.second ? TacPasskeys.view(results.second) : null
    return globalThis[TacPasskeys.cryptoHooksKey].prf(prfKey, { first: TacPasskeys.view(results.first), second }).catch((error) => { throw TacPasskeys.keyError(error) })
  }

  static keyError(error) {
    return new DOMException('', error?.reason === 'exists' ? 'KeyExistsError' : 'OperationError')
  }

  static withoutPrfResults(json) {
    const prf = json.clientExtensionResults?.prf
    if (prf?.results === undefined) return json
    const { results, ...kept } = prf
    return { ...json, clientExtensionResults: { ...json.clientExtensionResults, prf: kept } }
  }

  static refused(pending, error) {
    return pending.controller.signal.aborted ? null : TacPasskeys.failed(pending.call, error)
  }

  static release(pending) {
    if (TacPasskeys.pending === pending) TacPasskeys.pending = null
  }

  static stop() {
    TacPasskeys.pending?.controller.abort()
    TacPasskeys.pending = null
    return null
  }

  static failed(call, error) {
    const name = String(error?.name ?? '')
    const reason = TacPasskeys.reasons[name] ?? 'failed'
    return TacPasskeys.failure(call, reason, TacPasskeys.errorName.test(name) ? name : '')
  }

  static failure(call, reason, name = '') {
    const text = TacPasskeys.messages[reason]
    return ['PasskeyFailed', { call, reason, message: name ? `${text} (${name})` : text }]
  }

  static json(credential) {
    return typeof credential.toJSON === 'function' ? credential.toJSON() : TacPasskeys.encoded(credential)
  }

  static encoded(credential) {
    return {
      id: credential.id,
      rawId: TacPasskeys.text(credential.rawId),
      response: TacPasskeys.encodedResponse(credential.response),
      authenticatorAttachment: credential.authenticatorAttachment ?? undefined,
      clientExtensionResults: TacPasskeys.plain(credential.getClientExtensionResults()),
      type: credential.type,
    }
  }

  static encodedResponse(response) {
    if ('attestationObject' in response) return TacPasskeys.encodedAttestation(response)
    return {
      clientDataJSON: TacPasskeys.text(response.clientDataJSON),
      authenticatorData: TacPasskeys.text(response.authenticatorData),
      signature: TacPasskeys.text(response.signature),
      userHandle: response.userHandle ? TacPasskeys.text(response.userHandle) : undefined,
    }
  }

  static encodedAttestation(response) {
    const publicKey = response.getPublicKey?.()
    return {
      clientDataJSON: TacPasskeys.text(response.clientDataJSON),
      authenticatorData: TacPasskeys.text(response.getAuthenticatorData()),
      transports: response.getTransports?.() ?? [],
      publicKey: publicKey ? TacPasskeys.text(publicKey) : undefined,
      publicKeyAlgorithm: response.getPublicKeyAlgorithm(),
      attestationObject: TacPasskeys.text(response.attestationObject),
    }
  }

  static plain(value) {
    if (value instanceof ArrayBuffer || ArrayBuffer.isView(value)) return TacPasskeys.text(value)
    if (Array.isArray(value)) return value.map((item) => TacPasskeys.plain(item))
    if (value === null || typeof value !== 'object') return value
    return Object.fromEntries(Object.entries(value).map(([name, item]) => [name, TacPasskeys.plain(item)]))
  }

  static view(data) {
    return ArrayBuffer.isView(data) ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength) : new Uint8Array(data)
  }

  static text(data) {
    const binary = Array.from(TacPasskeys.view(data), (byte) => String.fromCharCode(byte)).join('')
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
  }
}

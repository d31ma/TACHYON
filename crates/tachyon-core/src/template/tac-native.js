class TacNativeCapabilities {
  static statuses = new Set(['available', 'selected', 'cancelled', 'read', 'released', 'denied', 'unsupported', 'unavailable', 'invalid', 'disposed', 'tooLarge'])
  static operations = new Set(['available', 'open', 'readText', 'release', 'dispose'])
  static route = '/__tachyon/capability/'
  static mostOwners = 1024
  static owners = new Map()

  static now(language, capability, request) {
    const operation = /^filePicker\.(\w+)$/.exec(capability)?.[1]
    let values
    try { values = JSON.parse(request) } catch { values = null }
    if (!TacNativeCapabilities.operations.has(operation) || typeof values?.owner !== 'string') return '{"status":"invalid"}'
    const key = `${language}\u001f${values.owner}`
    const owner = TacNativeCapabilities.owners.get(key)
    if (operation === 'dispose') return TacNativeCapabilities.disposeNow(key, owner)
    if (globalThis.__tachyonNativeFilePickerRoute !== true) return '{"status":"unsupported"}'
    if (!globalThis.__tachyonNativeGrants?.includes('filePicker')) return '{"status":"denied"}'
    if (operation === 'available') return '{"status":"available"}'
    if (!owner && TacNativeCapabilities.owners.size >= TacNativeCapabilities.mostOwners) return '{"status":"unavailable"}'
    if (!owner) TacNativeCapabilities.owners.set(key, TacWebApis.randomUuid())
    return TacNativeCapabilities.send(capability, { ...values, owner: TacNativeCapabilities.owners.get(key) })
  }

  static disposeNow(key, owner) {
    TacNativeCapabilities.owners.delete(key)
    return owner && globalThis.__tachyonNativeFilePickerRoute === true ? TacNativeCapabilities.send('filePicker.dispose', { owner }) : '{"status":"disposed"}'
  }

  static send(capability, values) {
    try {
      const request = new XMLHttpRequest()
      request.open('POST', TacNativeCapabilities.route + capability, false)
      request.setRequestHeader('X-Tachyon-Capability', capability)
      request.send(JSON.stringify(values))
      const result = request.status === 200 ? JSON.parse(request.responseText) : null
      return TacNativeCapabilities.statuses.has(result?.status) ? request.responseText : '{"status":"unavailable"}'
    } catch {
      return '{"status":"unavailable"}'
    }
  }

  static filePicker(signal) {
    const owner = TacWebApis.randomUuid()
    let disposed = signal?.aborted ?? false
    const call = async (operation, values = {}) => {
      if (disposed) return { status: 'disposed' }
      if (globalThis.__tachyonNativeFilePicker !== true) return { status: 'unsupported' }
      if (!globalThis.__tachyonNativeGrants?.includes('filePicker')) return { status: 'denied' }
      try {
        const answer = await globalThis.__tachyonNativeHostCall(`filePicker.${operation}`, JSON.stringify({ owner, ...values }))
        if (disposed) return { status: 'disposed' }
        const result = JSON.parse(answer)
        return result && TacNativeCapabilities.statuses.has(result.status) ? result : { status: 'unavailable' }
      } catch {
        return { status: 'unavailable' }
      }
    }
    signal?.addEventListener('abort', () => {
      disposed = true
      Promise.resolve().then(() => globalThis.__tachyonNativeHostCall?.('filePicker.dispose', JSON.stringify({ owner }))).catch(() => {})
    }, { once: true })
    return Object.freeze({
      available: () => !disposed && globalThis.__tachyonNativeFilePicker === true && globalThis.__tachyonNativeGrants?.includes('filePicker') === true,
      open: (options = {}) => !options || typeof options !== 'object' || (options.multiple !== undefined && options.multiple !== false) || (options.types && (!Array.isArray(options.types) || options.types.some((type) => !['text/plain', 'application/json'].includes(type))))
        ? Promise.resolve({ status: 'invalid' }) : call('open', { types: options.types ?? [] }),
      readText: (handle) => call('readText', { handle }),
      release: (handle) => call('release', { handle }),
    })
  }
}

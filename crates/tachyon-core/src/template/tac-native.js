class TacNativeCapabilities {
  static statuses = new Set(['available', 'selected', 'cancelled', 'read', 'released', 'denied', 'unsupported', 'unavailable', 'invalid', 'disposed', 'tooLarge'])

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

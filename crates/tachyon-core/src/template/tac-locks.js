class TacLocks {
  static key = Symbol.for('tachyon.tac.locks')
  static prefix = 'tachyon:'
  static mostNameLength = 256
  static held = new Map()
  static installed = TacLocks.install()

  static install() {
    Object.defineProperty(globalThis, TacLocks.key, {
      configurable: true,
      value: Object.freeze({
        request: (reply, name) => TacLocks.request(reply, TacLocks.checked(name)),
        release: (reply, name) => TacLocks.release(TacLocks.checked(name)),
      }),
    })
    return true
  }

  static checked(name) {
    if (typeof name === 'string' && name.length > 0 && name.length <= TacLocks.mostNameLength) return name
    throw new RangeError(`A lock name is text of 1 to ${TacLocks.mostNameLength} characters.`)
  }

  static request(reply, name) {
    const entry = TacLocks.held.get(name) ?? TacLocks.take(name)
    entry.granted.then((held) => TacLocks.answer(reply, name, held), (error) => TacLocks.failed(reply, error))
    return null
  }

  static answer(reply, name, held) {
    if (held) reply(['LockGranted', { name }])
  }

  static failed(reply, error) {
    reply(['BrowserCallFailed', { api: 'locks', call: 'request', message: `${error?.message ?? error}` }])
  }

  static take(name) {
    const entry = { granted: null, free: null, stop: new AbortController() }
    TacLocks.held.set(name, entry)
    entry.granted = TacLocks.shared() ? TacLocks.wait(name, entry) : Promise.resolve(true)
    entry.granted.catch(() => TacLocks.forget(name, entry))
    return entry
  }

  static forget(name, entry) {
    if (TacLocks.held.get(name) === entry) TacLocks.held.delete(name)
  }

  static shared() {
    return typeof globalThis.navigator?.locks?.request === 'function'
  }

  static wait(name, entry) {
    return new Promise((resolve, reject) => {
      const hold = () => new Promise((free) => { entry.free = free; resolve(true) })
      navigator.locks.request(`${TacLocks.prefix}${name}`, { signal: entry.stop.signal }, hold)
        .catch((error) => TacLocks.stopped(error, resolve, reject))
    })
  }

  static stopped(error, resolve, reject) {
    if (error?.name === 'AbortError') resolve(false)
    else reject(error)
  }

  static release(name) {
    const entry = TacLocks.held.get(name)
    TacLocks.held.delete(name)
    entry?.stop.abort()
    entry?.free?.()
    return null
  }
}

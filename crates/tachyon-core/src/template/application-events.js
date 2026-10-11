class TacApplicationEvents {
  constructor({ changed, failed, calling = (owner, work) => work(), announced = () => {} }) {
    this.changed = changed
    this.failed = failed
    this.calling = calling
    this.announced = announced
    this.listeners = new Map()
    this.retained = new Map()
    this.owners = new WeakMap()
    this.topics = new Map()
    this.listenerCount = 0
    this.pending = 0
    this.depth = 0
    this.publications = 0
    this.resetTimer = null
  }

  snapshot(value) {
    const active = new Set()
    let nodes = 0
    let characters = 0
    const text = (value) => this.fitsSize(characters += value.length)
    const visit = (item, depth) => {
      this.fitsStructure(++nodes > 8192 || depth > 32)
      if (typeof item === 'string') { text(item); return }
      if (item === null || typeof item === 'boolean') return
      if (typeof item === 'number' && Number.isFinite(item)) return
      if (typeof item !== 'object' || active.has(item)) throw new TypeError('Application events require finite JSON values.')
      const prototype = Object.getPrototypeOf(item)
      if (!Array.isArray(item) && prototype !== Object.prototype && prototype !== null) throw new TypeError('Application events require plain JSON values.')
      const array = Array.isArray(item)
      this.fitsStructure(array && item.length > 8192)
      active.add(item)
      let properties = 0
      for (const key of Reflect.ownKeys(item)) {
        if (array && key === 'length') continue
        const descriptor = Object.getOwnPropertyDescriptor(item, key)
        if (typeof key !== 'string' || !descriptor.enumerable || !Object.hasOwn(descriptor, 'value')) throw new TypeError('Application events require JSON data properties.')
        if (array && (!/^(0|[1-9][0-9]*)$/.test(key) || Number(key) >= item.length)) throw new TypeError('Application event arrays require indexed JSON values.')
        text(key)
        properties += 1
        visit(descriptor.value, depth + 1)
      }
      if (array && properties !== item.length) throw new TypeError('Application event arrays cannot contain holes.')
      active.delete(item)
    }
    visit(value, 0)
    const encoded = JSON.stringify(value)
    this.fitsSize(new TextEncoder().encode(encoded).byteLength)
    return JSON.parse(encoded)
  }

  fitsSize(size) {
    if (size > 65536) throw new RangeError('Application event exceeds 64 KiB.')
  }

  fitsStructure(exceeded) {
    if (exceeded) throw new RangeError('Application event structure exceeds its limit.')
  }

  name(value) {
    if (typeof value !== 'string' || !/^[A-Za-z0-9._:-]{1,128}$/.test(value)) throw new TypeError('Invalid application event name.')
    return value
  }

  publish(name, value, { retain = false } = {}) {
    this.name(name)
    if (this.depth >= 32 || this.publications >= 1024) throw new RangeError('Application event delivery cycle exceeds its limit.')
    const snapshot = this.snapshot(value)
    this.publications += 1
    if (this.resetTimer === null) this.resetTimer = setTimeout(() => { this.publications = 0; this.resetTimer = null }, 0)
    if (retain) {
      if (!this.retained.has(name) && this.retained.size >= 128) throw new RangeError('Application event retention limit reached.')
      this.retained.set(name, snapshot)
    }
    this.depth += 1
    try {
      for (const listener of [...(this.listeners.get(name) || [])]) this.deliver(listener, snapshot)
    } finally { this.depth -= 1 }
  }

  send(owner, name, value) {
    if (owner === null) return this.publish(name, value)
    this.name(name)
    const snapshot = this.snapshot(value)
    for (const listener of this.owners.get(owner)?.listeners || []) {
      if (listener.event === name) this.deliver(listener, snapshot)
    }
  }

  async settle(value, state) {
    if (this.pending >= 128) {
      void Promise.resolve(value).catch(() => {})
      throw new RangeError('Application event async admission limit reached.')
    }
    this.pending += 1
    let timer
    let cancel
    try {
      await Promise.race([
        value,
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Application event work timed out.')), 10000) }),
        new Promise((_, reject) => {
          cancel = () => reject(new Error('Application owner disposed.'))
          if (state.controller.signal.aborted) cancel()
          else state.controller.signal.addEventListener('abort', cancel, { once: true })
        }),
      ])
    } finally {
      clearTimeout(timer)
      state.controller.signal.removeEventListener('abort', cancel)
      this.pending -= 1
    }
  }

  deliver(listener, value) {
    const { owner, state, method, condition, asynchronous, event, models, returns } = listener
    if (state.disposed) return
    if (this.pending >= 128 || (models && !this.fits(models, event, value))) { this.failed(); return }
    if (condition && !this.matches(condition, value, event)) return
    const complete = (result) => {
      if (state.disposed) return
      if (!asynchronous && returns) this.republish(result, state, returns)
      this.changed()
    }
    const invoke = () => {
      if (state.disposed) return
      try {
        const result = owner[method](this.snapshot(value))
        if (!result || typeof result.then !== 'function') { complete(result); return }
        void this.settle(result, state).then(complete, () => { if (!state.disposed) this.failed() })
      } catch { this.failed() }
    }
    if (asynchronous) queueMicrotask(invoke)
    else invoke()
  }

  attach(owner, metadata, resources) {
    let state = this.owners.get(owner)
    if (state) { state.resources = resources; return }
    state = { owner, controller: new AbortController(), resources, metadata, listeners: [], injected: new Map(), disposed: false, initialized: false }
    this.owners.set(owner, state)
    try {
      for (const { field, name } of metadata.resources || []) {
        if (!Object.hasOwn(resources, name)) throw new TypeError('Application resource is unavailable.')
        Object.defineProperty(owner, field, { configurable: true, enumerable: false, get: () => this.resource(state, name) })
      }
      for (const listener of metadata.listeners || []) this.listen(owner, state, listener)
    } catch (error) { this.dispose(owner); throw error }
  }

  resource(state, name) {
    if (name === 'lifecycle') return state.controller.signal
    if (name === 'view') return state.resources.view
    if (!state.injected.has(name)) state.injected.set(name, Object.freeze(this.injectable(state, name)))
    return state.injected.get(name)
  }

  injectable(state, name) {
    if (name === 'applicationEventPublisher') return { publishEvent: (event, value, retain = false) => {
      this.alive(state)
      this.publish(event, value, { retain: retain === true })
      this.announced(state.resources.view, event, value)
    } }
    return Object.fromEntries(Object.keys(state.resources[name]).map(method => [method, (...args) => {
      this.alive(state)
      return this.calling(state.owner, () => state.resources[name][method](...args))
    }]))
  }

  alive(state) { if (state.disposed) throw new Error('Application owner disposed.') }

  listen(owner, state, listener) {
    if (typeof owner[listener.method] !== 'function') throw new TypeError('Application listener method is unavailable.')
    for (const name of listener.events || [listener.event]) {
      this.name(name)
      if (this.listenerCount >= 1024 || (!this.listeners.has(name) && this.listeners.size >= 128)) throw new RangeError('Application listener limit reached.')
      const entry = { order: 0, ...listener, owner, state, event: name, sequence: this.listenerCount }
      const listeners = this.listeners.get(name) || new Set()
      listeners.add(entry)
      this.listeners.set(name, new Set([...listeners].sort((left, right) => left.order - right.order || left.sequence - right.sequence)))
      this.listenerCount += 1
      state.listeners.push(entry)
    }
  }

  matches(matcher, value, event) {
    const operand = (node) => {
      if (node.k === 'lit') return node.v
      if (node.k === 'event') return event
      let current = value
      for (const segment of node.path || []) {
        if (current === null || typeof current !== 'object') return undefined
        current = current[segment]
      }
      return current
    }
    const test = (node) => {
      switch (node.k) {
        case 'and': return test(node.l) && test(node.r)
        case 'or': return test(node.l) || test(node.r)
        case 'not': return !test(node.e)
        case 'truthy': return Boolean(operand(node.e))
        case 'cmp': {
          const left = operand(node.l), right = operand(node.r)
          if (node.op === 'eq') return left === right
          if (node.op === 'ne') return left !== right
          if (typeof left !== typeof right || (typeof left !== 'number' && typeof left !== 'string')) return false
          if (node.op === 'lt') return left < right
          if (node.op === 'le') return left <= right
          if (node.op === 'gt') return left > right
          return left >= right
        }
        default: return false
      }
    }
    try { return test(matcher) } catch { return false }
  }

  republish(result, state, name) {
    for (const item of [result].flat().slice(0, 32)) {
      if (state.disposed) return
      if (item != null) try { this.publish(name, item) } catch { this.failed() }
    }
  }

  fits(models, shape, value) {
    const next = (item, type) => this.fits(models, type, item), model = models[shape]
    if (shape[0] === '?') return value === null || next(value, shape.slice(1))
    if (shape[0] === '[') return Array.isArray(value) && value.every((item) => next(item, shape.slice(2)))
    if (!model) return shape === 'any' || (shape === 'integer' ? Number.isInteger(value) : typeof value === shape)
    const { open, fields } = model, keys = Object.keys(fields)
    return value?.constructor === Object && (open || Object.keys(value).every((key) => keys.includes(key))) &&
      keys.every((key) => Object.hasOwn(value, key) ? next(value[key], fields[key].replace('!', '')) : fields[key][0] !== '!')
  }

  async initialize(owner) {
    const state = this.owners.get(owner)
    if (!state || state.initialized || state.disposed) return
    state.initialized = true
    for (const listener of state.listeners) {
      if (listener.replay && this.retained.has(listener.event)) this.deliver(listener, this.retained.get(listener.event))
    }
    for (const stream of state.metadata.streams || []) stream.topic ? this.topic(state, stream) : this.stream(owner, state, stream)
    const method = state.metadata.post_construct
    if (method) await this.settle(Promise.resolve().then(() => {
      if (!state.disposed) return owner[method]()
    }), state)
  }

  topic(state, stream) {
    let shared = this.topics.get(stream.path)
    if (!shared) {
      shared = { owners: 0, disposed: false, controller: new AbortController() }
      this.topics.set(stream.path, shared)
      this.stream(null, shared, stream)
    }
    shared.owners++
    state.controller.signal.addEventListener('abort', () => {
      if (--shared.owners) return
      shared.disposed = true
      shared.controller.abort()
      this.topics.delete(stream.path)
    }, { once: true })
  }

  stream(owner, state, stream) {
    const socket = stream.socket, Kind = globalThis[socket ? 'WebSocket' : 'EventSource']
    if (typeof Kind !== 'function') { this.failed(); return }
    const entry = { ...stream, owner, state }, waiting = []
    let source = null, timer, tries = 0, last = null
    const close = () => { clearTimeout(timer); source?.close(); source = null }
    const wait = () => { clearTimeout(timer); if (!socket) timer = setTimeout(retry, 60000) }
    const retry = () => { close(); if (!state.disposed && tries < 5) timer = setTimeout(open, 1000 * 2 ** tries++) }
    const receive = ({ data, lastEventId }) => {
      tries = 0
      wait()
      if (lastEventId) last = Number(lastEventId)
      try {
        const message = this.snapshot(JSON.parse(data))
        if (!stream.topic) this.deliver(entry, message)
        if (!stream.models || this.fits(stream.models, stream.event, message)) this.publish(stream.event, message)
      } catch { this.failed() }
    }
    const open = () => {
      if (state.disposed) return
      if (socket) {
        const url = new URL(stream.path, location.href)
        url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
        source = new Kind(url)
        source.onopen = () => { for (const text of waiting.splice(0)) source.send(text) }
        source.onclose = ({ code }) => { source = null; if (![1000, 1003, 1007, 1009].includes(code)) retry() }
      } else {
        source = new Kind(stream.topic && last !== null ? `${stream.path}?position=${last + 1}` : stream.path)
        source.onerror = retry
        source.addEventListener('end', close)
      }
      source.onmessage = receive
      wait()
    }
    if (socket) Object.defineProperty(owner, stream.sender ?? 'send', { configurable: true, value: async (message) => {
      this.alive(state)
      const text = JSON.stringify(this.snapshot(message))
      if (source?.readyState === 1) return source.send(text)
      if (waiting.length >= 16) throw new RangeError('The socket is not open.')
      waiting.push(text)
      if (!source) { tries = 0; clearTimeout(timer); open() }
    } })
    state.controller.signal.addEventListener('abort', close, { once: true })
    open()
  }

  dispose(owner) {
    const state = this.owners.get(owner)
    if (!state || state.disposed) return
    state.disposed = true
    state.controller.abort()
    for (const listener of state.listeners) {
      const listeners = this.listeners.get(listener.event)
      if (listeners?.delete(listener)) this.listenerCount -= 1
      if (!listeners?.size) this.listeners.delete(listener.event)
    }
    state.listeners.length = 0
    const method = state.metadata.pre_destroy
    if (method && state.initialized) {
      try {
        const result = owner[method]()
        if (result && typeof result.then === 'function') {
          void this.settle(result, { controller: new AbortController() }).catch(() => this.failed())
        }
      } catch { this.failed() }
    }
  }
}

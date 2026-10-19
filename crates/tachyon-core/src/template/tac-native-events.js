class TacNativeEvents {
  static plans = new WeakMap()
  static listenerLimit = 128
  static streamLimit = 16
  static publishing = null

  static listenerMethod(key) {
    return `__tachyonListen:${key}`
  }

  static senderMethod(client) {
    return `__tachyonSend:${client}`
  }

  static bind(owner, declared, call, refresh, topics) {
    const listeners = (Array.isArray(declared?.listeners) ? declared.listeners : []).slice(0, TacNativeEvents.listenerLimit)
      .filter((listener) => typeof listener?.key === 'string' && Array.isArray(listener.events))
    for (const listener of listeners) {
      Object.defineProperty(owner, TacNativeEvents.listenerMethod(listener.key), { configurable: true, value: async (value) => {
        if (TacNativeEvents.publishing === owner) return
        await call({ op: 'event', name: listener.key, value })
        await refresh()
        owner.__tachyonStoredChanged?.()
      } })
    }
    Object.defineProperty(owner, '__tachyonStream', { configurable: true, value: () => {} })
    TacNativeEvents.plans.set(owner, {
      listeners: listeners.map((listener) => ({ ...listener, asynchronous: false, method: TacNativeEvents.listenerMethod(listener.key) })),
      streams: [...TacNativeEvents.held(declared?.streams), ...TacNativeEvents.topics(listeners, topics)],
    })
  }

  static held(streams) {
    return (Array.isArray(streams) ? streams : []).slice(0, TacNativeEvents.streamLimit)
      .filter((stream) => typeof stream?.path === 'string' && stream.path.startsWith('/') && !stream.path.startsWith('//'))
      .map(({ client, path, event, socket, models }) => ({
        method: '__tachyonStream', path, event, socket: socket === true, models, topic: false,
        ...(socket === true ? { sender: TacNativeEvents.senderMethod(client) } : {}),
      }))
  }

  static topics(listeners, topics) {
    const streams = new Map()
    for (const listener of listeners) {
      const [event] = listener.events
      const path = topics?.[event]
      if (typeof path === 'string' && !streams.has(path)) streams.set(path, { method: '', path, event, socket: false, models: listener.models, topic: true })
    }
    return [...streams.values()]
  }

  static planned(owner, metadata) {
    const native = TacNativeEvents.plans.get(owner)
    return native ? { ...metadata, listeners: [...(metadata.listeners || []), ...native.listeners], streams: [...(metadata.streams || []), ...native.streams] } : metadata
  }

  static publish(owner, publish) {
    TacNativeEvents.publishing = owner
    try { publish() } finally { TacNativeEvents.publishing = null }
  }

  static send(owner, signal) {
    const sender = owner?.[TacNativeEvents.senderMethod(signal.send)]
    if (typeof sender !== 'function') throw new TypeError('The native service holds no socket client of that name.')
    return sender(signal.value)
  }
}

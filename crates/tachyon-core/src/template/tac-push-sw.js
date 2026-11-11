class TacPushWorker {
  static home = '/'
  static installed = TacPushWorker.install()

  static install() {
    self.addEventListener('install', (event) => event.waitUntil(self.skipWaiting()))
    self.addEventListener('push', (event) => event.waitUntil(TacPushWorker.show(event.data)))
    self.addEventListener('notificationclick', (event) => event.waitUntil(TacPushWorker.open(event.notification)))
    return true
  }

  static show(data) {
    const message = TacPushWorker.message(data)
    return Promise.all([TacPushWorker.notify(message), TacPushWorker.tell(message)])
  }

  static notify({ title, body, path, tag }) {
    const options = { body: String(body ?? ''), data: { path: TacPushWorker.path(path) } }
    if (tag) options.tag = String(tag)
    return self.registration.showNotification(String(title || self.location.hostname), options)
  }

  static async tell({ tag, path, data }) {
    const received = { tachyon: 'PushReceived', tag: TacPushWorker.text(tag), path: TacPushWorker.path(path), data: TacPushWorker.text(data) }
    const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true })
    for (const page of windows.filter(TacPushWorker.ownPage)) page.postMessage(received)
  }

  static ownPage(client) {
    return new URL(client.url).origin === self.location.origin
  }

  static text(value) {
    if (value === undefined || value === null) return ''
    if (typeof value === 'string') return value
    return JSON.stringify(value)
  }

  static message(data) {
    if (!data) return {}
    try {
      return TacPushWorker.record(data.json())
    } catch {
      return { body: data.text() }
    }
  }

  static record(value) {
    if (value && typeof value === 'object') return value
    return { body: String(value) }
  }

  static path(value) {
    if (typeof value !== 'string' || !value.startsWith('/') || value.startsWith('//') || value.includes('\\')) return TacPushWorker.home
    const url = new URL(value, self.location.origin)
    if (url.origin !== self.location.origin) return TacPushWorker.home
    return url.pathname + url.search + url.hash
  }

  static async open(notification) {
    notification.close()
    const url = new URL(TacPushWorker.path(notification.data?.path), self.location.origin).href
    const windows = await self.clients.matchAll({ type: 'window', includeUncontrolled: true })
    const open = windows.find((client) => client.url === url)
    if (open) return open.focus()
    return self.clients.openWindow(url)
  }
}

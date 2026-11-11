class TacWebPush {
  static key = Symbol.for('tachyon.tac.push')
  static worker = '/.tachyon/tac-push-sw.js'
  static scope = '/.tachyon/push/'
  static keyBytes = 65
  static uncompressedPoint = 4
  static startTimeout = 10000
  static installed = TacWebPush.install()

  static install() {
    Object.defineProperty(globalThis, TacWebPush.key, {
      configurable: true,
      value: Object.freeze({
        supported: () => TacWebPush.missing() === '',
        subscribe: (key) => TacWebPush.subscribe(key),
        subscription: () => TacWebPush.subscription(),
        unsubscribe: () => TacWebPush.unsubscribe(),
      }),
    })
    return true
  }

  static missing() {
    if (typeof globalThis.__tachyonNativeHostCall === 'function') return 'Web Push needs a web browser. The native app has no Push API.'
    if (globalThis.isSecureContext !== true) return 'Web Push needs an HTTPS page or localhost.'
    if (!('serviceWorker' in navigator) || typeof globalThis.PushManager !== 'function') return 'This browser has no Push API. On iPhone and iPad, Web Push needs iOS 16.4 or newer and the app added to the Home Screen.'
    return ''
  }

  static usable() {
    const reason = TacWebPush.missing()
    if (reason) throw new Error(reason)
  }

  static subscribe(key) {
    TacWebPush.usable()
    const serverKey = TacWebPush.serverKey(key)
    return () => TacWebPush.subscribed(serverKey, Notification.requestPermission())
  }

  static subscription() {
    TacWebPush.usable()
    return async () => TacWebPush.event(await TacWebPush.current())
  }

  static unsubscribe() {
    TacWebPush.usable()
    return () => TacWebPush.ended()
  }

  static async subscribed(serverKey, asking) {
    if (await asking !== 'granted') throw new Error('Notifications are not allowed, so Web Push cannot start.')
    const registration = await TacWebPush.registration()
    const existing = await registration.pushManager.getSubscription()
    if (existing && !TacWebPush.sameKey(existing, serverKey)) await existing.unsubscribe()
    const subscription = await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: serverKey })
    return TacWebPush.event(subscription)
  }

  static async ended() {
    const subscription = await TacWebPush.current()
    const endpoint = subscription?.endpoint ?? ''
    await subscription?.unsubscribe()
    return ['PushUnsubscribed', { endpoint }]
  }

  static async current() {
    const registration = await navigator.serviceWorker.getRegistration(TacWebPush.scope)
    return registration ? registration.pushManager.getSubscription() : null
  }

  static async registration() {
    const registration = await navigator.serviceWorker.register(TacWebPush.worker, { scope: TacWebPush.scope })
    return registration.active ? registration : TacWebPush.started(registration)
  }

  static started(registration) {
    const worker = registration.installing ?? registration.waiting
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('The push service worker did not start.')), TacWebPush.startTimeout)
      worker.addEventListener('statechange', () => {
        if (worker.state !== 'activated') return
        clearTimeout(timer)
        resolve(registration)
      })
    })
  }

  static event(subscription) {
    if (!subscription) return ['PushUnsubscribed', { endpoint: '' }]
    const { endpoint, expirationTime, keys } = subscription.toJSON()
    return ['PushSubscribed', { endpoint, p256dh: keys?.p256dh ?? '', auth: keys?.auth ?? '', expirationTime: expirationTime ?? 0 }]
  }

  static sameKey(subscription, serverKey) {
    const current = new Uint8Array(subscription.options?.applicationServerKey ?? new ArrayBuffer(0))
    return current.length === serverKey.length && current.every((byte, index) => byte === serverKey[index])
  }

  static serverKey(key) {
    const text = String(key)
    if (!/^[A-Za-z0-9_-]+={0,2}$/.test(text)) throw TacWebPush.badKey()
    const bytes = Uint8Array.from(atob(text.replace(/=+$/, '').replace(/-/g, '+').replace(/_/g, '/')), (letter) => letter.charCodeAt(0))
    if (bytes.length !== TacWebPush.keyBytes || bytes[0] !== TacWebPush.uncompressedPoint) throw TacWebPush.badKey()
    return bytes
  }

  static badKey() {
    return new Error('The Web Push server key must be a base64url VAPID public key: 65 bytes that start with 0x04.')
  }
}

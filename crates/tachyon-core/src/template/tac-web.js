class TacWebApis {
  static key = Symbol.for('tachyon.tac.web')
  static mostRandomBytes = 65536
  static darkScheme = '(prefers-color-scheme: dark)'
  static reducedMotion = '(prefers-reduced-motion: reduce)'
  static grants = 'meta[name="tachyon-browser-apis"]'
  static positionTimeout = 10000
  static smallestWindow = 64
  static largestWindow = 16384
  static applicationStates = ['background', 'foreground']
  static caller = null
  static events = null
  static hostPermission = globalThis.__tachyonNotificationPermission ?? 'default'
  static clock = Object.freeze({
    now: () => Date.now(),
    monotonic: () => performance.now(),
    timeZone: () => Intl.DateTimeFormat().resolvedOptions().timeZone,
  })
  static secureRandom = Object.freeze({
    bytes: (count) => TacWebApis.randomBytes(count),
    uuid: () => TacWebApis.randomUuid(),
  })
  static connectivity = Object.freeze({ online: () => navigator.onLine })
  static pageVisibility = Object.freeze({ visible: () => document.visibilityState === 'visible' })
  static preferences = Object.freeze({
    scheme: () => TacWebApis.matches(TacWebApis.darkScheme) ? 'dark' : 'light',
    motion: () => TacWebApis.matches(TacWebApis.reducedMotion) ? 'reduce' : 'no-preference',
    language: () => navigator.language,
  })
  static preferenceStrings = Object.freeze({
    read: (key) => TacPreferenceStrings.read(key),
    write: (key, value) => TacPreferenceStrings.write(key, value),
  })
  static documentTitle = Object.freeze({ set: (...arguments_) => TacDocumentTitle.host(...arguments_) })
  static clipboard = Object.freeze({
    writeText: (text) => TacWebApis.later({ api: 'clipboard', call: 'writeText', gesture: true, values: { text: String(text) } }, () => navigator.clipboard.writeText(String(text)), () => null),
    readText: () => TacWebApis.later({ api: 'clipboard', call: 'readText', gesture: true }, () => navigator.clipboard.readText(), (text) => ['ClipboardTextRead', { text }]),
  })
  static share = Object.freeze({
    open: (title, text, url) => TacWebApis.later({ api: 'share', call: 'open', gesture: true, values: { title, text, url } }, () => navigator.share({ title, text, url }), () => ['ShareFinished', {}]),
  })
  static notifications = Object.freeze({
    permission: () => TacWebApis.permission(),
    requestPermission: () => TacWebApis.later({ api: 'notifications', call: 'requestPermission', gesture: true }, () => Notification.requestPermission(), (permission) => ['NotificationPermissionChanged', { permission: TacWebApis.hostPermission = permission }]),
    show: (title, body) => TacWebApis.notify(title, body),
  })
  static geolocation = Object.freeze({
    currentPosition: () => TacWebApis.later({ api: 'geolocation', call: 'currentPosition', gesture: false, values: { timeout: TacWebApis.positionTimeout } }, () => TacWebApis.position(), (position) => TacWebApis.found(position)),
  })
  static windowControls = Object.freeze({
    granted: (control) => TacWebApis.windowGrants().includes(control),
    ...Object.fromEntries(['minimize', 'maximize', 'fullscreen', 'close'].map((control) => [control, () => TacWebApis.window(control, {})])),
    resize: (width, height) => TacWebApis.window('resize', TacWebApis.windowSize(width, height)),
    setTitle: (text) => TacWebApis.window('title', { title: String(text) }, 'setTitle'),
  })
  static apis = Object.freeze({
    clock: TacWebApis.clock,
    secureRandom: TacWebApis.secureRandom,
    connectivity: TacWebApis.connectivity,
    pageVisibility: TacWebApis.pageVisibility,
    preferences: TacWebApis.preferences,
    preferenceStrings: TacWebApis.preferenceStrings,
    documentTitle: TacWebApis.documentTitle,
    clipboard: TacWebApis.clipboard,
    share: TacWebApis.share,
    notifications: TacWebApis.notifications,
    geolocation: TacWebApis.geolocation,
    windowControls: TacWebApis.windowControls,
  })
  static installed = TacWebApis.install()

  static install() {
    Object.defineProperty(globalThis, TacWebApis.key, {
      configurable: true,
      value: Object.freeze({
        ...TacWebApis.apis,
        host: (text) => TacWebApis.answer(text),
        calling: (owner, work) => TacWebApis.calling(owner, work),
      }),
    })
    return true
  }

  static watch(events) {
    TacWebApis.events = events
    const connectivity = () => events.publish('ConnectivityChanged', { online: TacWebApis.connectivity.online() })
    const hosted = typeof globalThis.__tachyonNativeHostCall === 'function'
    const application = (state) => { if (TacWebApis.applicationStates.includes(state)) events.publish('ApplicationStateChanged', { state }) }
    const visibility = () => {
      const visible = TacWebApis.pageVisibility.visible()
      events.publish('VisibilityChanged', { visible })
      if (!hosted) application(TacWebApis.applicationStates[+visible])
    }
    const preferences = () => events.publish('PreferencesChanged', TacWebApis.currentPreferences())
    addEventListener('online', connectivity)
    addEventListener('offline', connectivity)
    addEventListener('languagechange', preferences)
    addEventListener('tac.application.state', (event) => application(event.detail?.state))
    document.addEventListener('visibilitychange', visibility)
    TacWebApis.media(TacWebApis.darkScheme)?.addEventListener('change', preferences)
    TacWebApis.media(TacWebApis.reducedMotion)?.addEventListener('change', preferences)
    connectivity()
    visibility()
    if (hosted) application(globalThis.__tachyonApplicationStateNow ?? 'foreground')
    preferences()
    return true
  }

  static calling(owner, work) {
    const previous = TacWebApis.caller
    TacWebApis.caller = owner
    try {
      return work()
    } finally {
      TacWebApis.caller = previous
    }
  }

  static later({ api, call, gesture, values = {} }, work, answer) {
    TacWebApis.allowed(api, gesture)
    const owner = TacWebApis.caller
    const failed = (error) => ['BrowserCallFailed', { api, call, message: TacWebApis.message(error) }]
    const run = TacWebApis.bridged(api) ? () => TacWebApis.bridge(api, call, values) : work
    new Promise((resolve) => resolve(run())).then(answer, failed).then((event) => TacWebApis.reply(owner, event))
    return null
  }

  static bridged(api) {
    return typeof globalThis.__tachyonNativeHostCall === 'function' && globalThis.__tachyonBrowserApiBridge?.includes(api) === true
  }

  static async bridge(api, call, values) {
    const raw = await globalThis.__tachyonNativeHostCall(`browserApi.${api}.${call}`, JSON.stringify(values))
    const reply = typeof raw === 'string' ? JSON.parse(raw) : raw
    if (!reply || 'error' in reply) throw new Error(reply?.error || `The native app could not run ${api}.${call}.`)
    return reply.value ?? null
  }

  static reply(owner, event) {
    if (event) TacWebApis.events?.send(owner, ...event)
  }

  static allowed(api, gesture) {
    const granted = document.querySelector(TacWebApis.grants)?.content.split(' ') ?? []
    if (!granted.includes(api)) throw new Error(`The ${api} browser API is not granted. Add "${api}" to tac.browserApis in client/manifest.json.`)
    if (gesture && navigator.userActivation?.isActive === false) throw new Error(`The ${api} browser API needs a user action. Call it from a click or key handler.`)
  }

  static permission() {
    TacWebApis.allowed('notifications', false)
    return TacWebApis.bridged('notifications') ? TacWebApis.hostPermission : Notification.permission
  }

  static notify(title, body) {
    if (TacWebApis.permission() !== 'granted') throw new Error('Notifications are not allowed. Call requestPermission first.')
    const values = { title: String(title), body: String(body ?? '') }
    if (TacWebApis.bridged('notifications')) return TacWebApis.later({ api: 'notifications', call: 'show', gesture: false, values }, null, () => null)
    new Notification(values.title, { body: values.body })
    return null
  }

  static windowGrants() {
    return Array.isArray(globalThis.__tachyonWindowControls) ? globalThis.__tachyonWindowControls : []
  }

  static window(control, payload, call = control) {
    if (typeof globalThis.__tachyonNativeHostCall !== 'function') throw new Error(`The ${control} window control needs the native app window.`)
    if (!TacWebApis.windowGrants().includes(control)) throw new Error(`The ${control} window control is not granted. Add "${control}" to tac.application.window.controls in client/manifest.json.`)
    const owner = TacWebApis.caller
    new Promise((resolve) => resolve(globalThis.__tachyonNativeHostCall(`window.${control}`, JSON.stringify(payload)))).then(TacWebApis.refusal).catch(TacWebApis.message).then((message) => message && TacWebApis.reply(owner, ['BrowserCallFailed', { api: 'windowControls', call, message }]))
    return null
  }

  static refusal(answer) {
    const { ok, error } = typeof answer === 'string' ? JSON.parse(answer) : answer ?? {}
    return error ?? (ok === false ? 'The native host refused the call.' : '')
  }

  static windowSize(width, height) {
    const { smallestWindow: least, largestWindow: most } = TacWebApis
    if (![width, height].every((size) => Number.isInteger(size) && size >= least && size <= most)) throw new RangeError(`A window size is a whole number from ${least} to ${most}.`)
    return { width, height }
  }

  static position() {
    return new Promise((resolve, reject) => {
      navigator.geolocation.getCurrentPosition(resolve, reject, { timeout: TacWebApis.positionTimeout })
    })
  }

  static found({ coords, timestamp }) {
    return ['PositionFound', { latitude: coords.latitude, longitude: coords.longitude, accuracy: coords.accuracy, timestamp }]
  }

  static currentPreferences() {
    const { scheme, motion, language } = TacWebApis.preferences
    return { scheme: scheme(), motion: motion(), language: language() }
  }

  static media(query) {
    return typeof matchMedia === 'function' ? matchMedia(query) : undefined
  }

  static matches(query) {
    return TacWebApis.media(query)?.matches === true
  }

  static message(error) {
    return typeof error?.message === 'string' && error.message ? error.message : 'Tachyon browser call failed.'
  }

  static answer(text) {
    try {
      return JSON.stringify({ value: TacWebApis.run(JSON.parse(text)) })
    } catch (error) {
      return JSON.stringify({ error: TacWebApis.message(error) })
    }
  }

  static run(request) {
    const api = Object.hasOwn(TacWebApis.apis, request?.api) ? TacWebApis.apis[request.api] : undefined
    const call = api && Object.hasOwn(api, request.call) ? api[request.call] : undefined
    if (typeof call !== 'function') throw new Error(`Tachyon has no browser call named '${request?.api}.${request?.call}'.`)
    return call(...(Array.isArray(request.args) ? request.args : []))
  }

  static randomBytes(count) {
    if (!Number.isInteger(count) || count < 0 || count > TacWebApis.mostRandomBytes) {
      throw new RangeError(`SecureRandom gives 0 to ${TacWebApis.mostRandomBytes} bytes.`)
    }
    return [...crypto.getRandomValues(new Uint8Array(count))]
  }

  static randomUuid() {
    const bytes = crypto.getRandomValues(new Uint8Array(16))
    bytes[6] = (bytes[6] & 0x0f) | 0x40
    bytes[8] = (bytes[8] & 0x3f) | 0x80
    const hex = [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('')
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
  }
}

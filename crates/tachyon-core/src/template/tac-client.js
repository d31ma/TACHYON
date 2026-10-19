const readViewPlan = () => {
  const planElement = document.getElementById('tachyon-view')
  return planElement ? JSON.parse(planElement.textContent || '{}') : null
}

class ClientView {
  static page = readViewPlan()
  static plan = ClientView.page ?? { nodes: [], services: {}, state: {} }
}

class Components {
  static instances = new Map()
  static records = new Map()
  static mountSchedules = new Map()
  static activations = new WeakMap()
  static activating = new Set()
}

const cancelMount = (key) => Components.mountSchedules.get(key)?.cancel()

class Page {
  static owner = Object.assign(Object.create(null), ClientView.plan.state)
}

class Rendering {
  static current = null
  static renderAgain = false
  static nodeCount = 0
  static iterationCount = 0
  static instanceKeys = new Set()
  static elements = new Map()
  static mounts = []
  static loopContents = new WeakMap()
}

/** @typedef {{nodes: object[], named: Record<string, object[]>, environment: object}} RenderSlot */
/** @typedef {{slot: RenderSlot | null, namespace: string}} RenderContext */

class Elements {
  static records = new WeakMap()
  static listeners = new WeakMap()
  static #scopeProperty = '--_tac-owner'
  /** @type {string} */
  static #htmlNamespace = 'http://www.w3.org/1999/xhtml'
  /** @type {string} */
  static #svgNamespace = 'http://www.w3.org/2000/svg'
  /** @type {string} */
  static #svgTag = 'svg'
  /** @type {string} */
  static #foreignObjectTag = 'foreignObject'
  /** @type {string} */
  static #xlinkNamespace = 'http://www.w3.org/1999/xlink'
  /** @type {string} */
  static #xmlNamespace = 'http://www.w3.org/XML/1998/namespace'
  /** @type {string} */
  static #xmlnsNamespace = 'http://www.w3.org/2000/xmlns/'
  /** @type {RenderContext} */
  static rootContext = { slot: null, namespace: Elements.#htmlNamespace }
  /** @type {string[]} */
  static #htmlIntegrationPoints = [Elements.#foreignObjectTag, 'desc', 'title']
  /** @type {Map<string, string>} */
  static #svgTags = new Map([
    'altGlyph', 'altGlyphDef', 'altGlyphItem', 'animateColor', 'animateMotion',
    'animateTransform', 'clipPath', 'feBlend', 'feColorMatrix', 'feComponentTransfer',
    'feComposite', 'feConvolveMatrix', 'feDiffuseLighting', 'feDisplacementMap',
    'feDistantLight', 'feDropShadow', 'feFlood', 'feFuncA', 'feFuncB', 'feFuncG',
    'feFuncR', 'feGaussianBlur', 'feImage', 'feMerge', 'feMergeNode', 'feMorphology',
    'feOffset', 'fePointLight', 'feSpecularLighting', 'feSpotLight', 'feTile',
    'feTurbulence', Elements.#foreignObjectTag, 'glyphRef', 'linearGradient', 'radialGradient',
    'textPath',
  ].map(name => /** @type {[string, string]} */ ([name.toLowerCase(), name])))
  /** @type {Map<string, string>} */
  static #svgAttributes = new Map([
    'attributeName', 'attributeType', 'baseFrequency', 'baseProfile', 'calcMode',
    'clipPathUnits', 'diffuseConstant', 'edgeMode', 'filterUnits', 'glyphRef',
    'gradientTransform', 'gradientUnits', 'kernelMatrix', 'kernelUnitLength',
    'keyPoints', 'keySplines', 'keyTimes', 'lengthAdjust', 'limitingConeAngle',
    'markerHeight', 'markerUnits', 'markerWidth', 'maskContentUnits', 'maskUnits',
    'numOctaves', 'pathLength', 'patternContentUnits', 'patternTransform',
    'patternUnits', 'pointsAtX', 'pointsAtY', 'pointsAtZ', 'preserveAlpha',
    'preserveAspectRatio', 'primitiveUnits', 'refX', 'refY', 'repeatCount',
    'repeatDur', 'requiredExtensions', 'requiredFeatures', 'specularConstant',
    'specularExponent', 'spreadMethod', 'startOffset', 'stdDeviation', 'stitchTiles',
    'surfaceScale', 'systemLanguage', 'tableValues', 'targetX', 'targetY',
    'textLength', 'viewBox', 'viewTarget', 'xChannelSelector', 'yChannelSelector',
    'zoomAndPan',
  ].map(name => /** @type {[string, string]} */ ([`${Elements.#svgNamespace}:${name.toLowerCase()}`, name])))
  /** @type {Map<string, string>} */
  static #foreignAttributes = new Map([
    ['xlink:actuate', Elements.#xlinkNamespace],
    ['xlink:arcrole', Elements.#xlinkNamespace],
    ['xlink:href', Elements.#xlinkNamespace],
    ['xlink:role', Elements.#xlinkNamespace],
    ['xlink:show', Elements.#xlinkNamespace],
    ['xlink:title', Elements.#xlinkNamespace],
    ['xlink:type', Elements.#xlinkNamespace],
    ['xml:lang', Elements.#xmlNamespace],
    ['xml:space', Elements.#xmlNamespace],
    ['xmlns', Elements.#xmlnsNamespace],
    ['xmlns:xlink', Elements.#xmlnsNamespace],
  ].map(([name, namespace]) => /** @type {[string, string]} */ ([`${Elements.#svgNamespace}:${name}`, namespace])))

  /** @param {string} tag @param {string} namespace @returns {Element} */
  static create(tag, namespace) {
    const svg = Elements.#usesSvg(tag, namespace)
    const adjusted = Elements.#adjustedTag(tag)
    return Elements.#createElement(tag, adjusted, svg)
  }

  /** @param {Element} element @param {RenderContext} context @returns {RenderContext} */
  static childContext(element, context) {
    const namespace = Elements.#childNamespace(element)
    return { slot: context.slot, namespace }
  }

  /** @param {Element} element @param {string} name @param {string} value @returns {void} */
  static setAttribute(element, name, value) {
    const namespace = Elements.#attributeNamespace(element, name)
    const adjusted = Elements.#adjustedAttribute(element, name)
    Elements.#writeAttribute(element, namespace, adjusted, value)
  }

  /** @param {Element} element @param {string} name @returns {void} */
  static removeAttribute(element, name) {
    const namespace = Elements.#attributeNamespace(element, name)
    const adjusted = Elements.#adjustedAttribute(element, name)
    const local = name.slice(name.indexOf(':') + 1)
    Elements.#deleteAttribute(element, namespace, adjusted, local)
  }

  /** @param {string} tag @param {string} namespace @returns {boolean} */
  static #usesSvg(tag, namespace) {
    return namespace === Elements.#svgNamespace || tag === Elements.#svgTag
  }

  /** @param {string} tag @returns {string} */
  static #adjustedTag(tag) {
    return Elements.#svgTags.get(tag) ?? tag
  }

  /** @param {string} tag @param {string} adjusted @param {boolean} svg @returns {Element} */
  static #createElement(tag, adjusted, svg) {
    if (svg) return document.createElementNS(Elements.#svgNamespace, adjusted)
    return document.createElement(tag)
  }

  /** @param {Element} element @returns {string} */
  static #childNamespace(element) {
    if (element.namespaceURI === Elements.#svgNamespace && !Elements.#htmlIntegrationPoints.includes(element.localName)) return Elements.#svgNamespace
    return Elements.#htmlNamespace
  }

  /** @param {Element} element @param {string} name @returns {string | null} */
  static #attributeNamespace(element, name) {
    return Elements.#foreignAttributes.get(`${element.namespaceURI}:${name}`) ?? null
  }

  /** @param {Element} element @param {string} name @returns {string} */
  static #adjustedAttribute(element, name) {
    return Elements.#svgAttributes.get(`${element.namespaceURI}:${name}`) ?? name
  }

  /** @param {Element} element @param {string | null} namespace @param {string} name @param {string} value @returns {void} */
  static #writeAttribute(element, namespace, name, value) {
    if (namespace) {
      element.setAttributeNS(namespace, name, value)
      return
    }
    element.setAttribute(name, value)
  }

  /** @param {Element} element @param {string | null} namespace @param {string} name @param {string} local @returns {void} */
  static #deleteAttribute(element, namespace, name, local) {
    if (namespace) {
      element.removeAttributeNS(namespace, local)
      return
    }
    element.removeAttribute(name)
  }

  /** @param {HTMLElement} host @param {string} name @returns {void} */
  static markScope(host, name) {
    host.setAttribute('data-tac-scope', name)
    host.style.setProperty(Elements.#scopeProperty, name)
  }
}

class NativeState {
  static writes = Promise.resolve()
  static writeCount = 0
  static methods = new WeakSet()
  static owners = new Map()
  static plans = new WeakMap()
  static applicationState = 'ApplicationStateChanged'
  static startUpRequestLimit = 1048576
  static memberRequestLimit = 65536

  static planned(owner, metadata) {
    const native = NativeState.plans.get(owner)
    return native ? { ...metadata, listeners: [...(metadata.listeners || []), ...native.listeners] } : metadata
  }

  static tooLarge(request, service, payload, limit) {
    const subject = request.op === 'init' ? 'start-up request' : `request to ${request.name ?? request.op}`
    const size = new TextEncoder().encode(payload).byteLength
    return `Native service ${service}: The ${subject} is ${size} bytes. The limit is ${limit} bytes. Pass less data to the service.`
  }
}

class SharedData {
  static root = Object.freeze(Object.create(null))
  static modules = new Map()
}

const readSharedData = (name) => {
  if (!/^[A-Za-z_][A-Za-z0-9_]{0,127}$/.test(name)) return undefined
  if (!SharedData.modules.has(name)) {
    const url = new URL(`../shared/data/${name}.json`, import.meta.url)
    SharedData.modules.set(name, import(url.href, { with: { type: 'json' } }).then((module) => module.default))
  }
  return SharedData.modules.get(name)
}

const markDocument = (name, value) => {
  document.documentElement.dataset[name] = value
}

const pageContext = () => Object.freeze({
  path: location.pathname.replace(/(.)\/+$/, '$1'),
  development: document.body?.dataset.tachyonEnvironment === 'development',
})

const evaluate = async (node, environment) => {
  if (!node || typeof node !== 'object') return undefined
  switch (node.k) {
    case 'await': return await evaluate(node.e, environment)
    case 'lit': return node.v
    case 'id':
      if (node.n in environment.locals) return environment.locals[node.n]
      if (node.n === 'data') return SharedData.root
      if (node.n === 'page') return pageContext()
      return environment.owner?.[node.n]
    case 'get': {
      const target = await evaluate(node.o, environment)
      if (target === SharedData.root) return await readSharedData(node.p)
      return target === null || target === undefined ? undefined : target[node.p]
    }
    case 'idx': {
      const target = await evaluate(node.o, environment)
      return Array.isArray(target) ? target[node.i] : undefined
    }
    case 'not': return !await evaluate(node.e, environment)
    case 'cmp': {
      const left = await evaluate(node.l, environment)
      const right = await evaluate(node.r, environment)
      if (node.op === 'eq') return left === right
      if (node.op === 'ne') return left !== right
      if (node.op === 'lt') return left < right
      if (node.op === 'le') return left <= right
      if (node.op === 'gt') return left > right
      return left >= right
    }
    case 'log': {
      const left = await evaluate(node.l, environment)
      if (node.op === 'and') return left ? await evaluate(node.r, environment) : left
      return left || await evaluate(node.r, environment)
    }
    case 'num': {
      const left = await evaluate(node.l, environment)
      const right = await evaluate(node.r, environment)
      if (node.op === 'add') return left + right
      if (node.op === 'sub') return left - right
      if (node.op === 'mul') return left * right
      return left / right
    }
    case 'if':
      return await evaluate(node.c, environment)
        ? await evaluate(node.t, environment)
        : await evaluate(node.f, environment)
    case 'call': {
      const callee = node.c
      const owner = callee.k === 'get' || callee.k === 'idx'
        ? await evaluate(callee.o, environment)
        : environment.owner
      const target = await evaluate(callee, environment)
      if (typeof target !== 'function') return undefined
      const arguments_ = await Promise.all((node.a || []).map((value) => evaluate(value, environment)))
      return target.apply(owner, arguments_)
    }
    default: return undefined
  }
}

const display = (value) => {
  if (value === null || value === undefined || typeof value === 'object') return ''
  return String(value)
}
const childEnvironment = (environment, locals) => ({
  owner: environment.owner,
  locals: Object.assign(Object.create(environment.locals), locals),
})
const memberTarget = (owner, path) => {
  const dot = typeof path === 'string' ? path.indexOf('.') : -1
  if (dot < 0) return [owner, path]
  return [owner?.[path.slice(0, dot)], path.slice(dot + 1)]
}

const ownedEnvironment = (owner) => ({ owner, locals: Object.create(null) })

const resolveEventArgument = async (argument, event, environment) => {
  if ('value' in argument) return argument.value
  if ('expression' in argument) return evaluate(argument.expression, environment)
  if (!('event' in argument) || argument.event === '') return event
  let value = event
  for (const segment of String(argument.event).split('.')) {
    if (value === null || value === undefined) return undefined
    value = value[segment]
  }
  return value
}

const bindEvent = (element, type, binding, environment) => {
  const listener = async (event) => {
    try {
      const arguments_ = await Promise.all(
        (binding.arguments || []).map((argument) => resolveEventArgument(argument, event, environment)),
      )
      if (binding.assign) {
        const { target, operator } = binding.assign
        const [holder, field] = memberTarget(environment.owner, target)
        const current = holder?.[field]
        const value = arguments_[0]
        holder[field] =
          operator === '+=' ? current + value
          : operator === '-=' ? current - value
          : operator === '*=' ? current * value
          : operator === '/=' ? current / value
          : value
      } else {
        const [holder, name] = memberTarget(environment.owner, binding.handler)
        const handler = holder?.[name]
        if (typeof handler !== 'function') throw new TypeError(`Tac handler '${binding.handler}' is not available.`)
        await handler.call(holder, ...(NativeState.methods.has(handler) ? arguments_ : [event, ...arguments_]))
      }
      await render()
      markDocument('tachyonEvents', 'handled')
    } catch {
      markDocument('tachyonEventError', 'handler_failed')
    }
  }
  element.addEventListener(type, listener)
  const listeners = Elements.listeners.get(element) || []
  listeners.push({ type, listener })
  Elements.listeners.set(element, listeners)
}

const applyAttributes = async (element, attributes, environment) => {
  let safeHtml
  for (const attribute of attributes || []) {
    if (attribute.event) {
      bindEvent(element, attribute.eventType, attribute.event, environment)
      continue
    }
    const value = 'expression' in attribute
      ? await evaluate(attribute.expression, environment)
      : attribute.value
    if (attribute.name === 'safe-html') {
      safeHtml = value === false || value === null || value === undefined ? '' : String(value)
      continue
    }
    if (value === false || value === null || value === undefined) {
      Elements.removeAttribute(element, attribute.name)
      continue
    }
    const rendered = value === true ? '' : display(value)
    Elements.setAttribute(element, attribute.name, rendered)
    if (['checked', 'disabled', 'selected'].includes(attribute.name)) element[attribute.name] = Boolean(value)
    else if (attribute.name === 'value' && 'value' in element) {
      try { element.value = rendered } catch {}
    }
  }
  return safeHtml
}

class StaticCache {
  static namePrefix = 'tachyon-static-'
}

const openStaticCache = async () => {
  try {
    if (!globalThis.caches) return null
    const name = (await caches.keys()).find((value) => value.startsWith(StaticCache.namePrefix))
    return name ? await caches.open(name) : null
  } catch { return null }
}

const variesOnlyByEncoding = (response) => (response.headers.get('vary') || '').split(',')
  .every((name) => !name.trim() || name.trim().toLowerCase() === 'accept-encoding')

const storableResponse = (response) => response.ok && response.type === 'basic'
  && !/(?:no-store|private)/i.test(response.headers.get('cache-control') || '')
  && variesOnlyByEncoding(response)

const precacheAssets = async (urls) => {
  const cache = await openStaticCache()
  if (!cache) return
  if (!Array.isArray(urls) || urls.length > 512) throw new RangeError('Tac precache exceeds 512 assets.')
  const deadline = Date.now() + 30000
  let warmedBytes = 0
  for (const raw of new Set(urls)) {
    if (Date.now() >= deadline || warmedBytes >= 33554432) break
    try {
      const url = new URL(raw, location.href)
      if (url.origin !== location.origin || !['http:', 'https:'].includes(url.protocol)) continue
      if (await cache.match(url)) continue
      const response = await fetch(url, { credentials: 'omit', signal: AbortSignal.timeout(5000) })
      const body = await responseBody(response, 4194304)
      if (body === null || !storableResponse(response)) continue
      if (warmedBytes + body.byteLength > 33554432) break
      warmedBytes += body.byteLength
      await cache.put(url, new Response(body, { status: response.status, statusText: response.statusText, headers: response.headers }))
    } catch {}
  }
}

const clearStaticCache = async () => {
  try {
    if (!globalThis.caches) return
    for (const name of await caches.keys()) {
      if (name.startsWith(StaticCache.namePrefix)) await caches.delete(name)
    }
  } catch {}
}

const planAssets = (nodes, found = []) => {
  for (const node of nodes || []) {
    if (found.length >= 512) break
    if (node.k === 'component') {
      for (const service of node.services || []) if (service.module) found.push(service.module)
      planAssets(node.template, found)
      planAssets(node.slot, found)
      for (const nodes of Object.values(node.slots || {})) planAssets(nodes, found)
    }
    planAssets(node.children, found)
  }
  return found
}

const responseBody = async (response, limit) => {
  if (!response.body) return new ArrayBuffer(0)
  const reader = response.body.getReader()
  const chunks = []
  let length = 0
  let timedOut = false
  const timer = setTimeout(() => { timedOut = true; void reader.cancel().catch(() => {}) }, 2000)
  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      length += value.byteLength
      if (length > limit) { void reader.cancel().catch(() => {}); return null }
      chunks.push(value)
    }
    if (timedOut) return null
    const body = new Uint8Array(length)
    let offset = 0
    for (const chunk of chunks) { body.set(chunk, offset); offset += chunk.byteLength }
    return body.buffer
  } finally { clearTimeout(timer) }
}

const clientRequest = (input, init = {}, options = {}) => {
  const request = new Request(typeof input === 'string' ? new URL(input, location.href) : input, init)
  return fetch(new Request(request, { cache: options.publicRead !== true ? 'no-store' : options.revalidate === true ? 'no-cache' : 'default' }))
}

const webStorage = (persistent) => {
  try { return persistent ? localStorage : sessionStorage } catch { return null }
}

const fitsStorage = (raw) => raw.length <= 65536 && new TextEncoder().encode(raw).byteLength <= 65536

const readStoredField = (store, key, fallback) => {
  try {
    const raw = store.getItem(key)
    if (raw === null || !fitsStorage(raw)) return fallback
    return JSON.parse(raw)
  } catch { return fallback }
}

const writeStoredField = (store, key, value) => {
  try {
    if (value === undefined) { store.removeItem(key); return }
    const raw = JSON.stringify(value)
    if (raw !== undefined && fitsStorage(raw)) store.setItem(key, raw)
  } catch {}
}

class Owners {
  static bound = new WeakSet()
  static managed = new WeakMap()
  static modules = new WeakMap()
  static stored = new WeakMap()
  static adoptLegacyField(store, key, legacy) {
    try {
      const raw = store.getItem(legacy)
      if (raw === null) return
      store.removeItem(legacy)
      if (store.getItem(key) === null) store.setItem(key, raw)
    } catch {}
  }
}

class StoredAccessor {
  constructor(owner, read, store, key) {
    this.owner = owner
    this.read = read
    this.store = store
    this.key = key
    this.saved = JSON.stringify(read.call(owner))
  }
  save() {
    try {
      const value = this.read.call(this.owner)
      const raw = JSON.stringify(value)
      if (raw === this.saved) return
      this.saved = raw
      writeStoredField(this.store, this.key, value)
    } catch {}
  }
}

const bindPersistentFields = (owner, scope) => {
  if (Owners.bound.has(owner)) return owner
  Owners.bound.add(owner)
  const stored = Owners.stored.get(owner) || {}
  const accessors = []
  for (const name of Object.keys(owner)) {
    if (!Object.hasOwn(stored, name) || !Object.getOwnPropertyDescriptor(owner, name)?.configurable) continue
    const local = stored[name] === 'local'
    const store = webStorage(local)
    if (!store) continue
    const key = `tac:${scope}:${name}`
    Owners.adoptLegacyField(store, key, `tac:${scope}:${local ? '$$' : '$'}${name}`)
    const { get, set } = Object.getOwnPropertyDescriptor(owner, name)
    if (get && set) {
      const restored = readStoredField(store, key, undefined)
      if (restored !== undefined) set.call(owner, restored)
      const accessor = new StoredAccessor(owner, get, store, key)
      accessors.push(accessor)
      Object.defineProperty(owner, name, {
        configurable: true,
        enumerable: true,
        get,
        set: (value) => { set.call(owner, value); accessor.save() },
      })
      continue
    }
    let current = readStoredField(store, key, owner[name])
    Object.defineProperty(owner, name, {
      configurable: true,
      enumerable: true,
      get: () => current,
      set: (value) => { current = value; writeStoredField(store, key, value) },
    })
  }
  if (accessors.length) {
    Object.defineProperty(owner, '__tachyonStoredChanged', {
      configurable: true,
      value: () => { for (const accessor of accessors) accessor.save() },
    })
  }
  return owner
}

const storedPath = () => location.pathname.replace(/\/index\.html$/, '/')
const persistScope = (id, properties) =>
  `${id}:${properties?.persistId ?? properties?.['persist-id'] ?? storedPath()}`

const importAsset = (raw) => {
  const url = new URL(raw, location.href)
  const sameOrigin = url.origin === location.origin
  const web = sameOrigin && url.pathname.startsWith('/.tachyon/services/')
  const native = sameOrigin && url.pathname.includes('/WebBundle/tachyon-runtime/services/')
  if (!web && !native) throw new TypeError('Tac service asset is not a generated same-origin URL.')
  return import(url.href)
}

const createOwner = async (module, properties) => {
  const controller = new AbortController()
  try {
    let owner
    if (typeof module.__tachyon_create_owner__ !== 'function') {
      const required = module.__tachyon_required_args__
      owner = Array.isArray(required)
        ? new module.default(...required.map((name) => properties?.[name]))
        : new module.default(properties)
    } else {
      owner = await module.__tachyon_create_owner__(properties, controller.signal, clientRequest)
    }
    if (typeof module.__tachyon_own_context__ === 'function') module.__tachyon_own_context__(owner, controller.signal)
    Owners.managed.set(owner, controller)
    Owners.modules.set(owner, module)
    return owner
  } catch (error) { controller.abort(); throw error }
}
const releaseOwner = (owner) => {
  Owners.managed.get(owner)?.abort()
  Owners.managed.delete(owner)
  disposeNativeInstance(owner)
}
const disposeManagedOwner = (owner) => {
  for (const entry of Services.byOwner.get(owner) || []) if (!entry.shared) disposeManagedOwner(entry.instance)
  releaseOwner(owner)
}

class Services {
  static catalog = ClientView.plan.services || {}
  static singletons = new Map()
  static resolved = new Map()
  static creating = new Set()
  static byOwner = new WeakMap()
  static shared = new WeakSet()
  static given(service, properties) {
    if (!Array.isArray(service.required)) return properties
    return Object.fromEntries(service.required.map((field) => [field, properties?.[field]]))
  }
}

const requireProperties = (name, service, properties) => {
  for (const field of service.required || []) {
    if (properties?.[field] !== undefined && properties?.[field] !== null) continue
    throw new TypeError(`Tac service '${name}' needs '${field}'; pass :${field}="..." to the component that uses it.`)
  }
}

const createServiceInstance = async (name, service, properties, scope) => {
  requireProperties(name, service, properties)
  const given = Services.given(service, properties)
  let instance
  if (service.module) {
    const module = await importAsset(service.module)
    if (typeof module.default !== 'function') throw new TypeError(`Tac service '${name}' must default-export its class.`)
    instance = await createOwner(module, given)
  } else if (service.native && nativeHost()) {
    instance = {}
  } else {
    throw new TypeError(`Tac service '${name}' has no browser module.`)
  }
  if (service.native && nativeHost()) await bindNativeInstance(instance, name, given)
  Owners.stored.set(instance, service.stored || {})
  attachRuntime(instance, scope)
  if (service.native && nativeHost()) drainCompanionQueue()
  for (const key of Object.keys(instance)) {
    if (key === name || !Object.hasOwn(Services.catalog, key) || instance[key] !== undefined) continue
    instance[key] = await serviceSingleton(key)
  }
  return instance
}
const registerSingleton = (name, instance) => {
  Services.shared.add(instance)
  for (const member of Runtime.graphs.get(instance) || [instance]) Services.shared.add(member)
  Services.resolved.set(name, instance)
}
const disposeSharedInstance = (instance) => {
  disposeRuntime(instance, true)
  releaseOwner(instance)
}
const serviceSingleton = (name) => {
  if (Services.creating.has(name)) throw new Error(`Tac services '${name}' depend on each other in a cycle.`)
  if (Services.singletons.has(name)) return Services.singletons.get(name)
  const service = Services.catalog[name]
  if (!service) throw new TypeError(`Tac service '${name}' is not declared.`)
  Services.creating.add(name)
  const promise = createServiceInstance(name, service, {}, `service:${name}:${storedPath()}`)
    .then((instance) => {
      registerSingleton(name, instance)
      return instance
    })
    .finally(() => Services.creating.delete(name))
  Services.singletons.set(name, promise)
  promise.catch(() => Services.singletons.delete(name))
  return promise
}
const disposeSharedServices = () => {
  for (const [name, instance] of Services.resolved) {
    disposeSharedInstance(instance)
    Services.resolved.delete(name)
    Services.singletons.delete(name)
  }
}

const composeOwner = (name, properties, entries) => {
  const owner = { ...properties }
  for (const entry of entries) {
    Object.defineProperty(owner, entry.name, { value: entry.instance, enumerable: false, configurable: true, writable: true })
  }
  Services.byOwner.set(owner, entries)
  attachRuntime(owner, persistScope(name, properties))
  return owner
}

const componentOwner = async (node, properties, path) => {
  Rendering.instanceKeys.add(path)
  if (Components.instances.has(path)) return Components.instances.get(path)
  const entries = []
  for (const service of node.services || []) {
    const shared = service.scope !== 'prototype'
    const instance = shared
      ? await serviceSingleton(service.name)
      : await createServiceInstance(service.name, service, properties, persistScope(`${node.name}.${service.name}`, properties))
    entries.push({ name: service.name, service, instance, shared })
  }
  const owner = composeOwner(node.name, properties, entries)
  Components.instances.set(path, owner)
  Components.records.set(path, { node, properties, owner })
  return owner
}

const snapshotState = (owner) => {
  const state = {}
  for (const [name, value] of Object.entries(owner || {})) {
    if (typeof value !== 'function') state[name] = value
  }
  try { return structuredClone(state) } catch { return {} }
}

const isTextControl = (element) => element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement
const snapshotMutableDom = () => ({
  active: document.activeElement?.id || null,
  elements: [...document.querySelectorAll('[id]')].slice(0, 2048).map((element) => ({
    id: element.id,
    open: element instanceof HTMLDetailsElement ? element.open : undefined,
    value: isTextControl(element) ? element.value : undefined,
    checked: element instanceof HTMLInputElement ? element.checked : undefined,
    scroll: element.scrollLeft || element.scrollTop ? [element.scrollLeft, element.scrollTop] : undefined,
  })),
})
const restoreMutableDom = (state) => {
  for (const saved of state.elements) {
    const element = document.getElementById(saved.id)
    if (!element) continue
    if (saved.open !== undefined && element instanceof HTMLDetailsElement) element.open = saved.open
    if (saved.value !== undefined && isTextControl(element)) element.value = saved.value
    if (saved.checked !== undefined && element instanceof HTMLInputElement) element.checked = saved.checked
    if (saved.scroll) element.scrollTo(saved.scroll[0], saved.scroll[1])
  }
  if (state.active) document.getElementById(state.active)?.focus({ preventScroll: true })
}

const carryHotState = async (stale, fresh) => {
  const state = typeof stale.hotState === 'function' ? structuredClone(await stale.hotState()) : snapshotState(stale)
  if (typeof stale.hotDispose === 'function') await stale.hotDispose()
  if (typeof fresh.restoreHotState === 'function') await fresh.restoreHotState(state)
  else Object.assign(fresh, state)
}

const hotService = (service, version) => {
  const url = new URL(service.module, location.href)
  url.searchParams.set('tachyon_hot', version || String(Date.now()))
  return { ...service, module: url.href }
}

const serviceBoundary = (service) => {
  const fileName = new URL(service.module, location.href).pathname.split('/').pop()
  return `service:${fileName.replace(/\.js$/, '')}`
}

const replaceSharedService = async (entry, version) => {
  const next = await createServiceInstance(entry.name, hotService(entry.service, version), {}, `service:${entry.name}:${storedPath()}`)
  await carryHotState(entry.instance, next)
  disposeSharedInstance(entry.instance)
  registerSingleton(entry.name, next)
  Services.singletons.set(entry.name, Promise.resolve(next))
  return next
}

const replaceOwnedService = async (entry, record, version) => {
  const scope = persistScope(`${record.node.name}.${entry.name}`, record.properties)
  const instance = await createServiceInstance(entry.name, hotService(entry.service, version), record.properties, scope)
  try { await carryHotState(entry.instance, instance) } catch (error) { disposeManagedOwner(instance); throw error }
  return instance
}

const hotUpdate = async (boundaries, version) => {
  const selected = new Set(boundaries || [])
  const domState = snapshotMutableDom()
  await clearStaticCache()
  const replaced = new Map()
  const affected = (service) => service.module && selected.has(serviceBoundary(service))
  for (const [path, record] of Components.records) {
    const entries = Services.byOwner.get(record.owner) || []
    if (!entries.some((entry) => affected(entry.service))) continue
    const fresh = []
    for (const entry of entries) {
      let instance = entry.instance
      if (affected(entry.service) && entry.shared) {
        if (!replaced.has(entry.name)) replaced.set(entry.name, await replaceSharedService(entry, version))
        instance = replaced.get(entry.name)
      } else if (affected(entry.service)) {
        instance = await replaceOwnedService(entry, record, version)
      }
      fresh.push({ ...entry, instance })
    }
    const owner = composeOwner(record.node.name, record.properties, fresh)
    disposeOwner(record.owner)
    cancelMount(path)
    record.owner = owner
    record.host = undefined
    Components.instances.set(path, owner)
  }
  await render()
  while (Components.activating.size) await Promise.allSettled([...Components.activating])
  if (Rendering.current) await Rendering.current.catch(() => {})
  restoreMutableDom(domState)
}

const trackActivation = (promise) => {
  Components.activating.add(promise)
  promise.finally(() => Components.activating.delete(promise)).catch(() => {})
  return promise
}

const activateOnInteraction = (host, pending, activate) => {
  const stopListening = () => {
    host.removeEventListener('pointerdown', listener, true)
    host.removeEventListener('keydown', listener, true)
  }
  const listener = async (event) => {
    if (event.cancelable) event.preventDefault()
    event.stopImmediatePropagation()
    stopListening()
    const target = event.target
    if (await activate()) {
      target.dispatchEvent(new Event(event.type, { bubbles: true, cancelable: true }))
    }
  }
  host.addEventListener('pointerdown', listener, true)
  host.addEventListener('keydown', listener, true)
  pending.cleanup = stopListening
}

const scheduleActivation = (host, policy, pending, activate) => {
  if (policy === 'idle' && globalThis.requestIdleCallback) {
    const id = requestIdleCallback(activate)
    pending.cleanup = () => globalThis.cancelIdleCallback?.(id)
  } else if (policy === 'idle') {
    const id = setTimeout(activate, 1)
    pending.cleanup = () => clearTimeout(id)
  } else if (policy === 'visible' && globalThis.IntersectionObserver) {
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) { observer.disconnect(); void activate() }
    }, { rootMargin: '100px' })
    pending.cleanup = () => observer.disconnect()
    observer.observe(host)
  } else if (policy === 'interaction') {
    activateOnInteraction(host, pending, activate)
  } else queueMicrotask(activate)
}

const mountComponent = (host, owner, policy, key) => {
  const schedules = Components.mountSchedules
  const activations = Components.activations
  if (activations.has(owner)) return
  const previous = schedules.get(key)
  if (previous?.host === host && previous.owner === owner && previous.policy === policy) return
  cancelMount(key)
  if (!policy || policy === 'never') return
  const pending = { host, owner, policy, cancelled: false, cleanup: () => {}, cancel: () => {} }
  const release = () => {
    pending.cleanup()
    pending.cleanup = () => {}
    if (schedules.get(key) === pending) schedules.delete(key)
  }
  pending.cancel = () => { pending.cancelled = true; release() }
  schedules.set(key, pending)
  const stillOwned = () => host.isConnected && Components.records.get(key)?.owner === owner
  const activate = () => trackActivation(activateNow())
  const activateNow = async () => {
    if (pending.cancelled || !stillOwned()) { pending.cancel(); return false }
    if (activations.has(owner)) { release(); return false }
    activations.set(owner, 'initializing')
    release()
    try {
      attachRuntime(owner, key, host)
      await initializeRuntime(owner)
      if (!stillOwned()) return false
      activations.set(owner, 'active')
      host.dataset.tachyonActive = 'true'
      host.removeAttribute('data-tachyon-mount-error')
      trackActivation(render())
      return true
    } catch {
      activations.set(owner, 'failed')
      disposeOwner(owner)
      if (!stillOwned()) return false
      host.dataset.tachyonActive = 'false'
      host.dataset.tachyonMountError = 'activation_failed'
      return false
    }
  }
  scheduleActivation(host, policy, pending, activate)
}

const renderText = async (parts, environment) => {
  let value = ''
  for (const part of parts || []) {
    value += 'expression' in part ? display(await evaluate(part.expression, environment)) : part.value
  }
  return document.createTextNode(value)
}

const recordElement = (element, node, environment, path) => {
  const locals = []
  for (const name in environment.locals) locals.push([name, environment.locals[name]])
  Elements.records.set(element, { node, owner: environment.owner, path,
    locals, children: [...element.childNodes],
    attributes: new Map(element.getAttributeNames().map((name) => [name, element.getAttribute(name)])) })
  Rendering.elements.set(path, element)
  return element
}

const plainLoopData = (_key, value) => {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return value
  const prototype = Object.getPrototypeOf(value)
  if (prototype === Object.prototype || prototype === null) return value
  throw new TypeError('Tac loop item is not plain data.')
}

const loopContent = (value) => {
  if (value === null || typeof value !== 'object') return undefined
  if (!Rendering.loopContents.has(value)) {
    let content
    try { content = JSON.stringify(value, plainLoopData) } catch {}
    Rendering.loopContents.set(value, content)
  }
  return Rendering.loopContents.get(value)
}

const sameLocal = (previous, next) => {
  if (Object.is(previous, next)) return true
  const content = loopContent(previous)
  return content !== undefined && content === loopContent(next)
}

const sameLocals = (previous, next) => previous.length === next.length
  && previous.every(([name, value], index) => next[index][0] === name && sameLocal(value, next[index][1]))

const retainedPair = (old, freshChild) => {
  const previous = Elements.records.get(old)
  const fresh = previous && Rendering.elements.get(previous.path)
  const next = fresh && Elements.records.get(fresh)
  if (!next || previous.node !== next.node || previous.owner !== next.owner) return null
  if (old.tagName !== fresh.tagName || old.namespaceURI !== fresh.namespaceURI || !sameLocals(previous.locals, next.locals)) return null
  if (freshChild && freshChild.parentNode !== fresh) return null
  return { old, fresh, previous }
}

const focusChain = (active, fragment, container) => {
  const chain = []
  let old = active
  let freshChild = null
  while (old && old !== container) {
    if (!Elements.records.has(old) && chain.length) {
      old = old.parentNode
      continue
    }
    const pair = retainedPair(old, freshChild)
    if (!pair) return []
    chain.unshift(pair)
    freshChild = pair.fresh
    old = old.parentNode
  }
  if (old !== container || chain[0]?.fresh.parentNode !== fragment) return []
  return chain
}

const ownedSlots = (parent) => [...parent.querySelectorAll('slot')].filter((slot) => {
  for (let node = slot.parentElement; node && node !== parent; node = node.parentElement) {
    if (Elements.records.has(node)) return false
  }
  return true
})

const distributionGroups = (parent, source, focusedChild) => {
  const slots = ownedSlots(parent)
  const groups = new Map(slots.map((slot) => [slot.getAttribute('name') || '', { slot, children: [] }]))
  if (!slots.includes(focusedChild.parentNode) || groups.size !== slots.length) return null
  for (const child of [...source.childNodes]) {
    const name = child.nodeType === Node.ELEMENT_NODE ? child.getAttribute('slot') || '' : ''
    const group = groups.get(name)
    if (!group) return null
    group.children.push(child)
  }
  return groups
}

const changesObservedAttribute = ({ old, fresh, previous }) => {
  if (!old.localName.includes('-')) return false
  try {
    const observed = old.constructor.observedAttributes || []
    if (!Array.isArray(observed) || observed.length > 256) return true
    const attributes = Elements.records.get(fresh).attributes
    return observed.some((name) => previous.attributes.get(name) !== attributes.get(name))
  } catch { return true }
}

const preflightFocus = (chain, fragment, container) => {
  const distributions = new Map()
  for (let depth = 0; depth < chain.length; depth += 1) {
    const pair = chain[depth]
    if (changesObservedAttribute(pair)) return null
    const parent = depth ? chain[depth - 1].old : container
    if (pair.old.parentNode === parent) continue
    const source = depth ? chain[depth - 1].fresh : fragment
    const groups = depth ? distributionGroups(parent, source, pair.old) : null
    if (!groups) return null
    distributions.set(pair, groups)
  }
  return distributions
}

const commitAndRefocus = (authored, active, replacement) => {
  const selection = [active.selectionStart, active.selectionEnd, active.selectionDirection]
  authored.commit()
  replacement.focus({ preventScroll: true })
  if (selection[0] !== null && selection[0] !== undefined) {
    try { replacement.setSelectionRange(...selection) } catch {}
  }
}

const syncAuthoredAttributes = (element, previous, next) => {
  for (const [name] of previous.attributes) {
    if (!next.attributes.has(name)) Elements.removeAttribute(element, name)
  }
  for (const [name, value] of next.attributes) {
    if (previous.attributes.get(name) !== value) Elements.setAttribute(element, name, value)
  }
}

const changedDuringRender = (element, active, editingAtStart) => element === active
  && editingAtStart?.element === element && editingAtStart.value !== element.value

const syncControlValue = (element, fresh, next, active, editingAtStart) => {
  if (!next.attributes.has('value') || !('value' in element)) return
  if (changedDuringRender(element, active, editingAtStart) || element.value === fresh.value) return
  try { element.value = fresh.value } catch {}
}

const syncControlFlags = (element, fresh, previous, next) => {
  for (const name of ['checked', 'disabled', 'selected']) {
    if (previous.attributes.has(name) || next.attributes.has(name)) element[name] = fresh[name]
  }
}

const transferEventListeners = (element, fresh) => {
  for (const { type, listener } of Elements.listeners.get(element) || []) element.removeEventListener(type, listener)
  const listeners = Elements.listeners.get(fresh) || []
  for (const { type, listener } of listeners) element.addEventListener(type, listener)
  Elements.listeners.set(element, listeners)
}

class FocusedRootUpdate {
  constructor(chain, distributions, active, editingAtStart) {
    Object.assign(this, { chain, distributions, active, editingAtStart })
  }

  syncElement(element, fresh) {
    const previous = Elements.records.get(element)
    const next = Elements.records.get(fresh)
    syncAuthoredAttributes(element, previous, next)
    syncControlValue(element, fresh, next, this.active, this.editingAtStart)
    syncControlFlags(element, fresh, previous, next)
    transferEventListeners(element, fresh)
    Elements.records.set(element, next)
    if (fresh.tachyonComponent) element.tachyonComponent = fresh.tachyonComponent
  }

  commit(parent, source, depth) {
    const pair = this.chain[depth]
    if (!pair) {
      if (!parent.matches('input, textarea, [contenteditable]')) parent.replaceChildren(...source.childNodes)
      return
    }
    if (pair.old.parentNode !== parent) {
      this.commitSlots(pair, depth)
      return
    }
    this.commitSiblings(parent, source, pair, depth)
  }

  commitSlots(pair, depth) {
    const owner = this.chain[depth - 1]
    for (const { slot, children } of this.distributions.get(pair).values()) {
      if (slot === pair.old.parentNode) {
        const content = document.createDocumentFragment()
        content.append(...children)
        this.commit(slot, content, depth)
      } else if (children.length || owner.previous.children.some((child) => child.parentNode === slot)) {
        slot.replaceChildren(...children)
      }
    }
  }

  commitSiblings(parent, source, pair, depth) {
    for (const child of [...parent.childNodes]) if (child !== pair.old) child.remove()
    let before = true
    for (const child of [...source.childNodes]) {
      if (child === pair.fresh) {
        this.syncElement(pair.old, pair.fresh)
        this.commit(pair.old, pair.fresh, depth + 1)
        before = false
      } else if (before) parent.insertBefore(child, pair.old)
      else parent.append(child)
    }
  }

  finalize() {
    const retained = new Map(this.chain.map((pair) => [pair.fresh, pair.old]))
    for (const { old, fresh } of this.chain) {
      const record = Elements.records.get(old)
      record.children = Elements.records.get(fresh).children.map((child) => retained.get(child) || child)
    }
    Rendering.mounts = Rendering.mounts.map(([host, ...arguments_]) => [retained.get(host) || host, ...arguments_])
  }
}

const focusedElement = () => {
  let active = document.activeElement
  while (active?.shadowRoot?.activeElement) active = active.shadowRoot.activeElement
  return active
}

const commitRoot = (container, fragment, editingAtStart) => {
  const active = container.getRootNode().activeElement
  const chain = focusChain(active, fragment, container)
  const authored = new AuthoredRootUpdate(container, fragment, active, editingAtStart).prepare()
  if (!chain.length || authored.retains(active)) { authored.commit(); return }
  const distributions = preflightFocus(chain, fragment, container)
  if (!distributions || authored.refuses(chain)) { commitAndRefocus(authored, active, chain.at(-1).fresh); return }
  const update = new FocusedRootUpdate(chain, distributions, active, editingAtStart)
  update.commit(container, fragment, 0)
  update.finalize()
}

const retainComponentRoots = () => {
  const retained = new Map()
  Rendering.mounts = Rendering.mounts.map(([fresh, owner, policy, key]) => {
    const record = Components.records.get(key)
    const previous = record?.host
    if (!record || !previous || previous === fresh) {
      if (record) record.host = fresh
      return [fresh, owner, policy, key]
    }
    syncAuthoredAttributes(previous, Elements.records.get(previous), Elements.records.get(fresh))
    const active = focusedElement()
    const editing = active && fresh.contains(active) ? active : null
    const selection = editing && typeof editing.selectionStart === 'number'
      ? [editing.selectionStart, editing.selectionEnd, editing.selectionDirection] : null
    new AuthoredRootUpdate(previous, fresh, active, null).prepare().commit()
    Elements.records.set(previous, { ...Elements.records.get(fresh), children: [...previous.childNodes] })
    previous.tachyonComponent = fresh.tachyonComponent
    fresh.replaceWith(previous)
    if (editing && document.activeElement !== editing) {
      editing.focus({ preventScroll: true })
      if (selection && typeof editing.setSelectionRange === 'function') {
        try { editing.setSelectionRange(selection[0], selection[1], selection[2] || 'none') } catch {}
      }
    }
    retained.set(fresh, previous)
    return [previous, owner, policy, key]
  })
  for (const [path, fresh] of Rendering.elements) {
    const element = retained.get(fresh) || fresh
    Rendering.elements.set(path, element)
    const record = Elements.records.get(element)
    if (record) record.children = record.children.map(child => retained.get(child) || child)
  }
}

const renderComponent = async (node, environment, path) => {
  const properties = {}
  for (const property of node.properties || []) {
    if (property.event) continue
    properties[property.name] = 'expression' in property
      ? await evaluate(property.expression, environment)
      : property.value
  }
  const componentKey = `${path}:${node.name}`
  const owner = await componentOwner(node, properties, componentKey)
  for (const [name, value] of Object.entries(properties)) {
    if (node.exported || !(name in owner)) owner[name] = value
  }
  const host = document.createElement('tachyon-component')
  host.dataset.tachyonComponent = node.name
  host.dataset.tachyonMount = node.mount || 'none'
  if (node.scope) Elements.markScope(host, node.name)
  host.append(await renderNodes(node.template || [], ownedEnvironment(owner), `${path}.template`, {
    namespace: Elements.rootContext.namespace,
    slot: { nodes: node.slot || [], named: node.slots || {}, environment },
  }))
  host.tachyonComponent = { instance: owner, refresh: render }
  Rendering.mounts.push([host, owner, node.mount, componentKey])
  return recordElement(host, node, environment, path)
}

const countIteration = () => {
  if (++Rendering.iterationCount > 100000) throw new RangeError('Tac client view exceeds 100,000 iterations.')
}

/** @param {RenderContext} context */
const renderIteration = async (node, environment, path, context) => {
  const fragment = document.createDocumentFragment()
  const values = await evaluate(node.iterable, environment)
  if (!Array.isArray(values)) return fragment
  if (values.length > 10000) throw new RangeError('Tac iteration exceeds 10,000 items.')
  for (let index = 0; index < values.length; index += 1) {
    countIteration()
    const local = childEnvironment(environment, { [node.binding]: values[index], $index: index })
    fragment.append(await renderNodes(node.children || [], local, `${path}.${index}`, context))
  }
  return fragment
}

const countedLoopContinues = (comparison, counter, to) => {
  if (comparison === 'lt') return counter < to
  if (comparison === 'le') return counter <= to
  if (comparison === 'gt') return counter > to
  return counter >= to
}

/** @param {RenderContext} context */
const renderCounted = async (node, environment, path, context) => {
  const fragment = document.createDocumentFragment()
  const from = Number(await evaluate(node.from, environment))
  const to = Number(await evaluate(node.to, environment))
  const magnitude = Number(await evaluate(node.step, environment))
  const ascending = node.comparison === 'lt' || node.comparison === 'le'
  const step = ascending ? magnitude : -magnitude
  if (!Number.isFinite(from) || !Number.isFinite(to) || !Number.isFinite(step) || magnitude <= 0) return fragment
  let index = 0
  for (let counter = from; countedLoopContinues(node.comparison, counter, to); counter += step) {
    countIteration()
    if (index >= 10000) throw new RangeError('Tac iteration exceeds 10,000 items.')
    const local = childEnvironment(environment, { [node.binding]: counter, $index: index })
    fragment.append(await renderNodes(node.children || [], local, `${path}.${index}`, context))
    index += 1
    if (counter + step === counter) throw new RangeError('Tac iteration step makes no progress.')
  }
  return fragment
}

/** @param {RenderContext} context */
const renderSwitch = async (node, environment, path, context) => {
  const value = await evaluate(node.value, environment)
  let fallback
  for (const candidate of node.children || []) {
    if (candidate.k !== 'case') continue
    if (!candidate.when) fallback = candidate
    else if (value === await evaluate(candidate.when, environment)) {
      return renderNodes(candidate.children || [], environment, `${path}.case`, context)
    }
  }
  if (!fallback) return document.createDocumentFragment()
  return renderNodes(fallback.children || [], environment, `${path}.default`, context)
}

/** @param {RenderContext} context */
const renderNode = async (node, environment, path, context) => {
  Rendering.nodeCount += 1
  if (Rendering.nodeCount > 100000) throw new RangeError('Tac client view exceeds 100,000 nodes.')
  if (node.k === 'text') return renderText(node.parts, environment)
  if (node.k === 'comment') return document.createComment(node.value || '')
  if (node.k === 'slot') {
    const slot = context.slot
    const nodes = node.name ? slot?.named[node.name] : slot?.nodes
    return nodes ? renderNodes(nodes, slot.environment, `${path}.slot`, { slot: null, namespace: context.namespace }) : document.createDocumentFragment()
  }
  if (node.k === 'iteration') return renderIteration(node, environment, path, context)
  if (node.k === 'counted') return renderCounted(node, environment, path, context)
  if (node.k === 'switch') return renderSwitch(node, environment, path, context)
  if (node.k === 'component') return renderComponent(node, environment, path)
  if (node.k !== 'element') return document.createDocumentFragment()
  const element = Elements.create(node.tag, context.namespace)
  const safeHtml = await applyAttributes(element, node.attributes, environment)
  if (!node.void && safeHtml !== undefined) element.append(TacSafeHtml.fill(element, safeHtml))
  else if (!node.void) {
    const children = node.children || []
    const childContext = Elements.childContext(element, context)
    element.append(await renderNodes(children, environment, `${path}.children`, childContext))
  }
  if (element instanceof HTMLSelectElement && element.hasAttribute('value')) element.value = element.getAttribute('value')
  return recordElement(element, node, environment, path)
}

/** @param {RenderContext} [context] @returns {Promise<DocumentFragment>} */
const renderNodes = async (nodes, environment, path, context = Elements.rootContext) => {
  const fragment = document.createDocumentFragment()
  for (let index = 0; index < nodes.length; index += 1) {
    const node = nodes[index]
    if (node.k === 'conditional') {
      if (node.branch !== 'if') continue
      let cursor = index
      let selected
      while (cursor < nodes.length) {
        const candidate = nodes[cursor]
        if (cursor > index && (candidate.k === 'comment'
          || (candidate.k === 'text' && candidate.parts?.every((part) => 'value' in part && !part.value.trim())))) {
          cursor += 1
          continue
        }
        if (candidate.k !== 'conditional') break
        if (!candidate.condition || await evaluate(candidate.condition, environment)) {
          selected = candidate
          cursor += 1
          break
        }
        cursor += 1
      }
      if (selected) fragment.append(await renderNodes(selected.children || [], environment, `${path}.${index}.branch`, context))
      index = cursor - 1
      continue
    }
    fragment.append(await renderNode(node, environment, `${path}.${index}`, context))
  }
  return fragment
}

class TacElements {
  static connected = new Set()
  static created = 0
  static stylesheet = null

  static define(definition) {
    for (const [name, service] of Object.entries(definition.services || {})) {
      if (!Object.hasOwn(Services.catalog, name)) Services.catalog[name] = service
    }
    if (!customElements.get(definition.node.name)) customElements.define(definition.node.name, TacElements.elementClass(definition))
  }

  static elementClass(definition) {
    const attributes = new Map(definition.props.map((name) => [TacElements.kebab(name), name]))
    const element = class extends HTMLElement {
      static observedAttributes = [...attributes.keys()]
      constructor() {
        super()
        this.attachShadow({ mode: 'open' })
        this._tacDefinition = definition
        this._tacPath = `element.${TacElements.created += 1}`
        this._tacNode = { ...definition.node, properties: [] }
        this._tacProperties = Object.create(null)
        for (const name of definition.props) TacElements.upgradeProperty(this, name)
      }
      connectedCallback() { TacElements.connected.add(this); TacElements.refresh() }
      disconnectedCallback() { TacElements.connected.delete(this); TacElements.refresh() }
      attributeChangedCallback(attribute, _previous, value) { TacElements.change(this, attributes.get(attribute), value) }
    }
    for (const name of definition.props) {
      Object.defineProperty(element.prototype, name, {
        configurable: true,
        enumerable: true,
        get() { return this._tacProperties[name] },
        set(value) { TacElements.change(this, name, value) },
      })
    }
    return element
  }

  static upgradeProperty(element, name) {
    if (!Object.hasOwn(element, name)) return
    const value = element[name]
    delete element[name]
    element[name] = value
  }

  static kebab(name) {
    return name.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase()
  }

  static change(element, name, value) {
    element._tacProperties[name] = value
    element._tacNode.properties = Object.entries(element._tacProperties).map(([key, given]) => ({ name: key, value: given }))
    if (element.isConnected) TacElements.refresh()
  }

  static refresh() {
    void render().catch(signalFailure)
  }

  static loadStylesheet() {
    TacElements.stylesheet ??= fetch(new URL('components.css', import.meta.url))
      .then((response) => response.ok ? response.text() : '')
      .then((text) => {
        const sheet = new CSSStyleSheet()
        sheet.replaceSync(text)
        return sheet
      })
    return TacElements.stylesheet
  }

  static async renderInto(element, environment, editingAtStart) {
    try {
      const root = element.shadowRoot
      if (element._tacDefinition.styles && !root.adoptedStyleSheets.length) root.adoptedStyleSheets = [await TacElements.loadStylesheet()]
      commitRoot(root, await renderNodes([element._tacNode], environment, element._tacPath), editingAtStart)
      delete element.dataset.tachyonRenderError
    } catch (error) {
      element.dataset.tachyonRenderError = 'render_failed'
      globalThis.reportError?.(error)
    }
  }

  static announce(view, name, value) {
    const root = view?.getRootNode?.()
    if (!(root instanceof ShadowRoot) || !TacElements.connected.has(root.host)) return
    const detail = JSON.parse(JSON.stringify(value))
    view.dispatchEvent(new CustomEvent(TacElements.kebab(name), { detail, bubbles: true, composed: true }))
  }
}

const renderDocument = async (environment) => {
  Rendering.nodeCount = 0
  Rendering.iterationCount = 0
  Rendering.instanceKeys = new Set()
  Rendering.elements = new Map()
  Rendering.mounts = []
  Rendering.loopContents = new WeakMap()
  const focused = focusedElement()
  const editingAtStart = { element: focused, value: focused?.value }
  if (ClientView.page) commitRoot(document.body, await renderNodes(ClientView.plan.nodes || [], environment, 'body'), editingAtStart)
  for (const element of TacElements.connected) await TacElements.renderInto(element, environment, editingAtStart)
  retainComponentRoots()
  for (const arguments_ of Rendering.mounts) mountComponent(...arguments_)
  for (const [key, record] of Components.records) {
    if (Rendering.instanceKeys.has(key)) continue
    cancelMount(key)
    disposeOwner(record.owner)
    Components.records.delete(key)
    Components.instances.delete(key)
  }
}

const render = async () => {
  if (Rendering.current) { Rendering.renderAgain = true; return Rendering.current }
  Rendering.current = (async () => {
    do {
      Rendering.renderAgain = false
      await NativeState.writes
      const scroll = { x: scrollX, y: scrollY }
      await renderDocument(ownedEnvironment(Page.owner))
      scrollTo(scroll.x, scroll.y)
    } while (Rendering.renderAgain)
    markDocument('tachyonRendered', 'client')
    delete document.documentElement.dataset.tachyonRenderError
  })().catch((error) => {
    markDocument('tachyonRenderError', 'render_failed')
    throw error
  }).finally(() => { Rendering.current = null })
  return Rendering.current
}

const signalFailure = () => {
  markDocument('tachyonSignalError', 'listener_failed')
}
class Runtime {
  static events = new TacApplicationEvents({
    changed: () => { void render().catch(signalFailure) },
    failed: signalFailure,
    calling: (owner, work) => TacWebApis.calling(owner, work),
    announced: (view, name, value) => TacElements.announce(view, name, value),
  })
  static graphs = new WeakMap()
  static watching = TacWebApis.watch({
    publish: (name, value) => {
      try { Runtime.events.publish(name, value, { retain: true }) } catch { signalFailure() }
    },
    send: (owner, name, value) => {
      try { Runtime.events.send(owner, name, value) } catch { signalFailure() }
    },
  })
}
const ownerResources = (element, signal) => ({
  view: element, lifecycle: null, applicationEventPublisher: null,
  resourceLoader: Object.freeze({ precache: precacheAssets, clearAssetCache: clearStaticCache }),
  taskScheduler: Object.freeze({ render }),
  clock: TacWebApis.clock,
  secureRandom: TacWebApis.secureRandom,
  connectivity: TacWebApis.connectivity,
  pageVisibility: TacWebApis.pageVisibility,
  preferences: TacWebApis.preferences,
  preferenceStrings: TacWebApis.preferenceStrings,
  documentTitle: TacDocumentTitle.owned(signal),
  clipboard: TacWebApis.clipboard,
  share: TacWebApis.share,
  notifications: TacWebApis.notifications,
  geolocation: TacWebApis.geolocation,
  windowControls: TacWebApis.windowControls,
  filePicker: TacNativeCapabilities.filePicker(signal),
})
const attachRuntime = (owner, scope, element) => {
  if (!owner || typeof owner !== 'object') return owner
  const composed = Services.byOwner.get(owner)
  if (composed) {
    const graph = []
    for (const entry of composed) {
      attachRuntime(entry.instance, entry.shared ? `service:${entry.name}:${storedPath()}` : scope, element)
      graph.push(...(Runtime.graphs.get(entry.instance) || [entry.instance]))
    }
    Runtime.graphs.set(owner, graph)
    return bindPersistentFields(owner, scope)
  }
  const module = Owners.modules.get(owner)
  const graph = Runtime.graphs.get(owner) || module?.__tachyon_event_owners__?.(owner) || [owner]
  Runtime.graphs.set(owner, graph)
  const resources = ownerResources(element, Owners.managed.get(owner)?.signal)
  for (const instance of graph) {
    const metadata = NativeState.planned(instance, module?.__tachyon_event_plan__?.types?.find((type) => type.name === instance.constructor?.name) || {})
    Runtime.events.attach(instance, metadata, resources)
  }
  return bindPersistentFields(owner, scope)
}
const initializeRuntime = async (owner) => {
  for (const instance of Runtime.graphs.get(owner) || [owner]) await Runtime.events.initialize(instance)
}
const disposeRuntime = (owner, shared = false) => {
  for (const instance of [...(Runtime.graphs.get(owner) || [owner])].reverse()) {
    if (!shared && Services.shared.has(instance)) continue
    Runtime.events.dispose(instance)
  }
}
const disposeOwner = (owner) => {
  disposeRuntime(owner)
  disposeManagedOwner(owner)
}

const nativeHost = () => typeof globalThis.__tachyonNativeHostCall === 'function'
const nativeCall = async (request, ownerId, service) => {
  const payload = JSON.stringify({ ...request, route: ClientView.plan.route, service, owner: ownerId })
  const limit = request.op === 'init' ? NativeState.startUpRequestLimit : NativeState.memberRequestLimit
  if (payload.length > limit || new TextEncoder().encode(payload).byteLength > limit) throw new RangeError(NativeState.tooLarge(request, service, payload, limit))
  let timer
  try {
    const raw = await Promise.race([
      Promise.resolve(globalThis.__tachyonNativeHostCall('companion.invoke', payload)),
      new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Tac native call timed out.')), 10000) }),
    ])
    if (typeof raw === 'string' && (raw.length > 65536 || new TextEncoder().encode(raw).byteLength > 65536)) throw new RangeError('Tac native response exceeds 64 KiB.')
    const answer = typeof raw === 'string' ? JSON.parse(raw) : raw
    if (!answer || answer.error || answer.ok === false) throw new Error('Tac native call failed.')
    return answer.value
  } finally { clearTimeout(timer) }
}
const nativeMember = (name) => typeof name === 'string'
  && /^[A-Za-z_$][A-Za-z0-9_$]{0,127}$/.test(name)
  && !['__proto__', 'prototype', 'constructor', 'tac'].includes(name)

const bindNativeFields = (owner, names, call) => {
  const state = new Map()
  const revisions = new Map()
  const pending = new Map()
  const readField = async (name, revision = revisions.get(name) ?? 0) => {
    const value = await call({ op: 'get', name })
    if ((revisions.get(name) ?? 0) === revision) state.set(name, value)
  }
  const refresh = async () => {
    for (const name of names) {
      if (!pending.has(name)) await readField(name)
    }
  }
  for (const name of names) {
    Object.defineProperty(owner, name, {
      configurable: true,
      enumerable: true,
      get: () => state.get(name),
      set: (value) => {
        if (NativeState.writeCount >= 128) throw new RangeError('Tac native assignment queue exceeds 128 writes.')
        NativeState.writeCount += 1
        const revision = (revisions.get(name) ?? 0) + 1
        revisions.set(name, revision)
        pending.set(name, revision)
        state.set(name, value)
        NativeState.writes = NativeState.writes.catch(() => {}).then(async () => {
          await call({ op: 'set', name, value })
          await readField(name, revision)
        }).finally(() => {
          NativeState.writeCount -= 1
          if (pending.get(name) === revision) pending.delete(name)
        })
        void NativeState.writes.catch(() => { markDocument('tachyonNativeError', 'assignment_failed') })
      },
    })
  }
  return refresh
}

const drainCompanionQueue = () => {
  const queue = globalThis.__tachyonCompanionQueue
  const queued = Array.isArray(queue) ? queue.splice(0).slice(-128) : []
  for (const signal of queued) globalThis.__tachyonCompanionPublish?.(signal)
}
const bindNativeInstance = async (owner, service, properties) => {
  const ownerId = crypto.randomUUID()
  const record = { ownerId, service, disposed: false }
  const call = (request) => {
    if (record.disposed && request.op !== 'dispose') throw new Error('Tac native owner disposed.')
    return nativeCall(request, ownerId, service)
  }
  let props = {}
  try { props = JSON.parse(JSON.stringify(properties ?? {})) ?? {} } catch {}
  const declared = await call({ op: 'init', props })
  const fields = (Array.isArray(declared?.fields) ? declared.fields : []).slice(0, 128).filter(nativeMember)
  const refreshNativeFields = bindNativeFields(owner, fields, call)
  await refreshNativeFields()
  for (const name of (Array.isArray(declared?.methods) ? declared.methods : []).slice(0, 128)) {
    if (!nativeMember(name)) continue
    const method = async (...args) => {
      await NativeState.writes
      const answer = await call({ op: 'call', name, args })
      await refreshNativeFields()
      owner.__tachyonStoredChanged?.()
      return answer
    }
    NativeState.methods.add(method)
    owner[name] = method
  }
  bindNativeListeners(owner, declared?.listeners, call)
  NativeState.owners.set(owner, record)
}
const bindNativeListeners = (owner, declared, call) => {
  const event = NativeState.applicationState
  const listeners = (Array.isArray(declared) ? declared : []).slice(0, 128)
    .filter((listener) => nativeMember(listener?.key) && Array.isArray(listener.events) && listener.events.includes(event))
  for (const { key } of listeners) {
    Object.defineProperty(owner, `__tachyonListen:${key}`, { configurable: true, value: (value) => call({ op: 'event', name: key, value }) })
  }
  const plan = listeners.map((listener) => ({ ...listener, events: [event], method: `__tachyonListen:${listener.key}` }))
  NativeState.plans.set(owner, { listeners: plan })
}
const disposeNativeInstance = (owner) => {
  const record = NativeState.owners.get(owner)
  if (!record || record.disposed) return
  record.disposed = true
  NativeState.owners.delete(owner)
  void nativeCall({ op: 'dispose' }, record.ownerId, record.service).catch(signalFailure)
}
if (nativeHost()) {
  globalThis.__tachyonCompanionPublish = (signal) => {
    if (signal?.route !== ClientView.plan.route) return
    const live = [...NativeState.owners.values()].some((record) => !record.disposed && record.ownerId === signal?.owner && record.service === signal?.service)
    if (!live) return
    try { Runtime.events.publish(signal.name, signal.value, { retain: signal.retain === true }) } catch { signalFailure() }
  }
  drainCompanionQueue()
}

addEventListener('pagehide', (event) => {
  if (event.persisted) return
  for (const record of Components.records.values()) disposeOwner(record.owner)
  disposeSharedServices()
})
Object.defineProperty(globalThis, '__tachyonTac', {
  configurable: true,
  value: Object.freeze({ render, hotUpdate, instance: Page.owner, instances: Components.instances, service: (name) => Services.resolved.get(name) }),
})
globalThis.__tc_rerender = render
addEventListener('tachyon:rerender', render)
await TacStorage.ready()
await render()
await render()

const whenIdle = (task) => globalThis.requestIdleCallback ? requestIdleCallback(task) : setTimeout(task, 1)
whenIdle(() => {
  void precacheAssets(planAssets(ClientView.plan.nodes).map((asset) => new URL(asset, location.href).href)).catch(() => {
    markDocument('tachyonCacheError', 'precache_failed')
  })
})

export { TacElements }

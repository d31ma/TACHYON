function __tachyon_create_behaviors__(registrations, environment, moduleId) {
  'use strict'
  const MAX_BYTES = 1048576
  const MAX_ACTIVE = 64
  const MAX_EVICTED = 1024
  const encoder = new TextEncoder()
  const records = new WeakMap()
  const contexts = new WeakSet()
  const calledKeys = new Set()
  const forbidden = new Set(['__proto__', 'prototype', 'constructor', 'behaviors', 'tac'])

  class BehaviorError extends Error {
    constructor(code, message, path = null) {
      super(message)
      this.name = 'BehaviorError'
      this.code = code
      this.path = path
    }
  }
  const invalid = (message) => { throw new BehaviorError('invalid', message) }
  const integer = (value, min, max) => Number.isSafeInteger(value) && value >= min && value <= max
  const own = (object, key) => Object.prototype.hasOwnProperty.call(object, key)
  const identifier = (value) => typeof value === 'string' && /^[A-Za-z_$][A-Za-z0-9_$]{0,127}$/.test(value) && !forbidden.has(value)
  const plain = (value) => value !== null && typeof value === 'object'
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null)
  const signalLike = (value) => value && typeof value.aborted === 'boolean'
    && typeof value.addEventListener === 'function' && typeof value.removeEventListener === 'function'
  const configurationError = (code) => { throw new BehaviorError(code, 'Invalid or inactive behavior configuration.') }
  const environmentPrefix = environment === 'tac' ? 'TAC_' : 'YON_'

  class JsonBoundary {
    #nodes = 0
    #characters = 0
    #limit
    constructor(limit) { this.#limit = limit }
    copy(value, depth = 0) {
      if (++this.#nodes > 10000 || depth > 32) invalid('Behavior JSON exceeds the structural bound.')
      if (value === null || typeof value === 'boolean') return value
      if (typeof value === 'string') return this.#string(value)
      if (typeof value === 'number' && Number.isFinite(value)) return value
      if (Array.isArray(value)) return this.#array(value, depth)
      if (plain(value)) return this.#object(value, depth)
      invalid('Behavior values must contain only finite JSON data.')
    }
    #string(value) {
      this.#characters += value.length
      if (this.#characters > this.#limit) invalid('Behavior JSON exceeds the byte bound.')
      return value
    }
    #array(value, depth) {
      if (value.length > 10000) invalid('Behavior JSON exceeds the array bound.')
      return Array.from({ length: value.length }, (_, index) => {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
        if (!descriptor || !own(descriptor, 'value')) invalid('Behavior JSON cannot contain accessors or sparse arrays.')
        return this.copy(descriptor.value, depth + 1)
      })
    }
    #object(value, depth) {
      const keys = Reflect.ownKeys(value)
      if (keys.length > 10000) invalid('Behavior JSON exceeds the object bound.')
      if (keys.some((key) => typeof key !== 'string')) invalid('Behavior JSON cannot contain symbol keys.')
      const output = {}
      for (const key of keys.sort()) {
        if (forbidden.has(key) && !(key === 'tac' && depth === 0)) invalid('Behavior JSON contains an unsafe field.')
        this.#string(key)
        const descriptor = Object.getOwnPropertyDescriptor(value, key)
        if (!descriptor || !own(descriptor, 'value')) invalid('Behavior JSON cannot contain accessors.')
        if (descriptor.enumerable) output[key] = this.copy(descriptor.value, depth + 1)
      }
      return output
    }
  }
  const json = (value, limit = MAX_BYTES) => {
    const copy = new JsonBoundary(limit).copy(value)
    if (encoder.encode(JSON.stringify(copy)).byteLength > limit) invalid('Behavior JSON exceeds the byte bound.')
    return copy
  }
  const freeze = (value) => {
    if (value && typeof value === 'object') {
      for (const child of Object.values(value)) freeze(child)
      Object.freeze(value)
    }
    return value
  }
  const exactKeys = (value, allowed, label) => {
    if (!plain(value) || Reflect.ownKeys(value).some((key) => !allowed.includes(key))) invalid(`Invalid behavior ${label}.`)
  }

  function propertySource(input, code) {
    try {
      let values = json(input, 65536)
      if (plain(values) && (own(values, 'tac') || own(values, 'yon'))) {
        values = Object.fromEntries(Object.entries(applicationSource(values)).filter(([path]) => path.startsWith(environment + '.')).map(([path, value]) => [path.replaceAll('.', '_').toUpperCase(), value]))
      }
      if (!plain(values) || Object.keys(values).length > 256) configurationError(code)
      for (const [key, value] of Object.entries(values)) {
        if (key.length > 256 || key.trim() !== key || !key.startsWith(environmentPrefix) || !/^[A-Z][A-Z0-9_]*$/.test(key)
          || typeof value !== 'string' || encoder.encode(value).length > 4096) configurationError(code)
      }
      return freeze(values)
    } catch { configurationError(code) }
  }
  function configurationProfiles(input) {
    try { input = json(input, 8192) } catch { configurationError('configuration') }
    if (!Array.isArray(input) || input.length > 32 || input.some((name) => typeof name !== 'string' || name.trim() !== name || !/^[A-Za-z_$][A-Za-z0-9_$]{0,63}$/.test(name))
      || new Set(input).size !== input.length) configurationError('configuration')
    return Object.freeze(input.slice())
  }
  function selectedConfiguration(record, options) {
    if (record.profiles && !record.profiles.some((profile) => options.profiles.includes(profile))) configurationError('inactive-configuration')
    if (record.conditionalOnProperty) {
      const condition = record.conditionalOnProperty
      if (Object.keys(options.properties).some((key) => key.startsWith(condition.name + '_'))) configurationError('inactive-configuration')
      const matches = own(options.properties, condition.name) ? options.properties[condition.name] === condition.havingValue : condition.matchIfMissing
      if (!matches) configurationError('inactive-configuration')
    }
  }
  function propertyValue(descriptor, raw) {
    let value
    if (raw === undefined) {
      if (!own(descriptor, 'default')) configurationError('configuration-properties')
      value = descriptor.default
    } else if (descriptor.type === 'string') value = raw
    else if (descriptor.type === 'boolean') {
      if (!['true', 'false'].includes(raw)) configurationError('configuration-properties')
      value = raw === 'true'
    } else {
      const grammar = descriptor.type === 'integer' ? /^-?(0|[1-9][0-9]*)$/ : /^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$/
      if (raw.trim() !== raw || !grammar.test(raw)) configurationError('configuration-properties')
      value = Number(raw)
    }
    if (['integer', 'number'].includes(descriptor.type)) {
      if (typeof value !== 'number' || !Number.isFinite(value) || Math.abs(value) > Number.MAX_SAFE_INTEGER
        || (descriptor.type === 'integer' && !Number.isSafeInteger(value))
        || (descriptor.min !== undefined && value < descriptor.min) || (descriptor.max !== undefined && value > descriptor.max)) configurationError('configuration-properties')
    }
    if (typeof value === 'string' && encoder.encode(value).length > 4096) configurationError('configuration-properties')
    return value
  }
  function bindProperties(Type, input) {
    const policy = records.get(Type)?.configurationProperties
    if (!policy) configurationError('configuration-properties')
    const values = propertySource(input, 'configuration-properties')
    const names = Object.values(policy.fields).map((field) => policy.prefix + field.name)
    if (Object.keys(values).some((key) => key.startsWith(policy.prefix) && !names.includes(key))) configurationError('configuration-properties')
    const result = {}
    for (const [field, descriptor] of Object.entries(policy.fields)) result[field] = propertyValue(descriptor, values[policy.prefix + descriptor.name])
    return freeze(json(result, 65536))
  }
  function applicationSource(input) {
    try { input = json(input, 65536) } catch { configurationError('configuration-properties') }
    if (!plain(input)) configurationError('configuration-properties')
    const flat = {}
    function visit(value, path, depth) {
      if (depth > 16 || path.length > 256 || Object.keys(flat).length >= 256) configurationError('configuration-properties')
      if (path === 'yon.server.port' && !Number.isSafeInteger(value)) configurationError('configuration-properties')
      if (plain(value)) {
        if (depth > 0 && Object.keys(value).length === 0) flat[path + '.'] = ''
        for (const [key, child] of Object.entries(value)) {
          if (!/^[a-z][a-z0-9]*$/.test(key)) configurationError('configuration-properties')
          visit(child, path + '.' + key, depth + 1)
        }
      } else {
        if (value === null || Array.isArray(value)) { flat[path + '.'] = ''; return }
        if (!['string', 'boolean', 'number'].includes(typeof value) || value === null) configurationError('configuration-properties')
        const text = String(value)
        if (encoder.encode(text).length > 4096 || Object.keys(flat).length >= 256) configurationError('configuration-properties')
        flat[path] = text
      }
    }
    const roots = ['tac', 'yon'].filter((root) => own(input, root))
    if (roots.length === 0) configurationError('configuration-properties')
    for (const root of roots) {
      if (!plain(input[root])) configurationError('configuration-properties')
      visit(input[root], root, 0)
    }
    return flat
  }
  function bindValues(Type, input) {
    const policy = records.get(Type)?.values
    if (!policy) configurationError('configuration-properties')
    const values = applicationSource(input)
    const result = {}
    for (const [field, descriptor] of Object.entries(policy)) {
      if (Object.keys(values).some((path) => path.startsWith(descriptor.path + '.'))) configurationError('configuration-properties')
      result[field] = propertyValue(descriptor, values[descriptor.path])
    }
    return freeze(json(result, 65536))
  }

  function normalizeSort(input, allowed) {
    if (!Array.isArray(input) || input.length > 16) invalid('Invalid behavior pagination sort.')
    const seen = new Set()
    return input.map((entry) => {
      if (typeof entry !== 'string') invalid('Invalid behavior pagination sort.')
      const [field, direction = 'asc', extra] = entry.split(':')
      if (!allowed.includes(field) || !['asc', 'desc'].includes(direction) || extra !== undefined || seen.has(field)) {
        invalid('Invalid behavior pagination sort.')
      }
      seen.add(field)
      return `${field}:${direction}`
    })
  }
  function normalizePage(input, options) {
    exactKeys(input, ['page', 'size', 'sort', 'offset'], 'pagination')
    const value = json(input, 4096)
    const page = value.page ?? 0
    const size = value.size ?? options.defaultSize
    const offset = page * size
    if (!integer(page, 0, 1000000) || !integer(size, 1, options.maxSize) || !integer(offset, 0, 1000000)) {
      invalid('Behavior pagination exceeds its integer or offset bound.')
    }
    if (value.offset !== undefined && value.offset !== offset) invalid('Invalid behavior pagination offset.')
    return freeze({ page, size, offset, sort: normalizeSort(value.sort ?? [], options.sort) })
  }
  function methodArguments(args, method) {
    if (!Array.isArray(args) || args.length > 64) invalid('Invalid behavior method arguments.')
    const result = args.slice()
    if (method.valid) result[method.valid.argument] = checkData(method.valid.schema, result[method.valid.argument])
    if (method.pageable) {
      const index = method.pageable.argument
      result[index] = normalizePage(result[index] === undefined ? {} : result[index], method.pageable)
    }
    return result
  }
  function keyArguments(method, args) {
    const selected = method.cacheable.keyArgs.map((index) => {
      if (!own(args, index) || args[index] === undefined) invalid('Cache key argument is missing.')
      return args[index]
    })
    return freeze(json(selected, 4096))
  }
  class CheckRejection extends BehaviorError {
    constructor(violations, path) {
      super('validation', 'Behavior validation failed.', path)
      this.violations = violations.map(({ field, rule }) => ({ field, rule }))
    }
  }
  function validationFailure(path) { throw new BehaviorError('validation', 'Behavior validation failed.', path) }
  function checkedObject(input) {
    let value
    try { value = json(input) } catch { validationFailure('$.json') }
    if (!plain(value)) validationFailure('$.object')
    return value
  }
  function violations(schema, value) {
    const broken = []
    const add = (field, key, rule) => broken.push({ field, rule, path: `${field}.${key}` })
    for (const [field, rules] of Object.entries(schema.fields)) {
      const data = own(value, field) ? value[field] : null
      if (rules.notNull && data === null) add(field, 'notNull', 'Required')
      if (rules.notBlank && (typeof data !== 'string' || /^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]*$/u.test(data))) add(field, 'notBlank', 'NotEmpty')
      if (data === null) continue
      if (rules.size) {
        const size = typeof data === 'string' ? encoder.encode(data).length : Array.isArray(data) ? data.length : plain(data) ? Object.keys(data).length : -1
        if (size < rules.size.min || size > rules.size.max) add(field, 'size', 'Length')
      }
      if (rules.min !== undefined && (typeof data !== 'number' || data < rules.min)) add(field, 'min', 'Min')
      if (rules.max !== undefined && (typeof data !== 'number' || data > rules.max)) add(field, 'max', 'Max')
    }
    return broken
  }
  function validateData(schema, input) {
    const value = checkedObject(input)
    if (!plain(schema?.fields)) validationFailure('$.schema')
    const broken = violations(schema, value)
    if (broken.length) validationFailure(broken[0].path)
    return value
  }
  function checkData(schema, input) {
    if (!plain(schema?.fields)) validationFailure('$.schema')
    let value
    try { value = checkedObject(input) } catch (error) { throw new CheckRejection([{ field: '', rule: 'Checked' }], error.path) }
    const broken = violations(schema, value)
    if (broken.length) throw new CheckRejection(broken, broken[0].path)
    return value
  }
  function cacheKey(partition, record, method, args) {
    const selected = keyArguments(method, args)
    return JSON.stringify(json([partition, record.cache.name, moduleId, record.name, method.name, selected], 4096))
  }
  const retryable = (error) => !(error instanceof BehaviorError) || error.code === 'adapter'
  const backoff = (policy, attempt) => {
    const delay = Math.min(policy.delayMs * policy.multiplier ** attempt, policy.maxDelayMs)
    return Math.round(delay + Math.random() * policy.jitterMs)
  }
  function safePath(path) {
    if (typeof path !== 'string' || path.length > 2048 || !path.startsWith('/') || path.startsWith('//')) return false
    if (/[\\?#\s\u0000-\u001f]/.test(path)) return false
    let decoded
    try { decoded = decodeURIComponent(path) } catch { return false }
    if (/[\\?#%\s\u0000-\u001f]/.test(decoded) || decoded.startsWith('//')) return false
    return !decoded.split('/').some((part) => part === '.' || part === '..')
  }
  function exchangeRequest(metadata, args) {
    const pick = (index) => {
      if (!own(args, index)) invalid('A request argument is missing.')
      return args[index]
    }
    const descriptor = { query: {}, headers: {} }
    for (const [name, index] of Object.entries(metadata.query ?? {})) descriptor.query[name] = pick(index)
    for (const [name, index] of Object.entries(metadata.headers ?? {})) descriptor.headers[name] = pick(index)
    if (metadata.body !== undefined) descriptor.body = pick(metadata.body)
    return descriptor
  }
  function exchangePath(metadata, args) {
    let path = metadata.path
    for (const [name, index] of Object.entries(metadata.pathVariables ?? {})) {
      const value = own(args, index) ? args[index] : undefined
      const text = ['number', 'boolean'].includes(typeof value) ? String(value) : value
      if (typeof text !== 'string' || !/^[A-Za-z0-9._~-]{1,256}$/.test(text) || text === '.' || text === '..') invalid('A path variable may use only letters, digits and - _ . ~')
      path = path.split(`{${name}}`).join(text)
    }
    if (!safePath(path)) invalid('The request path is not a safe origin-relative path.')
    return path
  }
  function requestDescriptor(raw) {
    exactKeys(raw, ['query', 'headers', 'body'], 'HTTP request descriptor')
    const value = json(raw)
    const query = value.query ?? {}
    const headers = value.headers ?? {}
    if (!plain(query) || !plain(headers)) invalid('Invalid behavior HTTP request descriptor.')
    for (const item of Object.values(query)) {
      if (!['string', 'number', 'boolean'].includes(typeof item)) invalid('Invalid behavior HTTP query value.')
    }
    for (const [key, item] of Object.entries(headers)) {
      if (!/^[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}$/.test(key) || typeof item !== 'string' || /[\r\n\u0000]/.test(item)) {
        invalid('Invalid behavior HTTP header.')
      }
      if (['host', 'connection', 'content-length', 'transfer-encoding', 'cookie'].includes(key.toLowerCase())) invalid('Behavior HTTP header is transport-owned.')
    }
    return freeze({ query, headers, ...(own(value, 'body') ? { body: value.body } : {}) })
  }

  class Operation {
    #controller = new AbortController()
    #timer
    #failure
    #reject
    #cancelled
    #expires
    execution
    constructor(partition, timeoutMs) {
      this.#expires = performance.now() + timeoutMs
      this.#failure = new Promise((_, reject) => { this.#reject = reject })
      this.#failure.catch(() => {})
      this.execution = Object.freeze({ environment, partition, signal: this.#controller.signal, deadline: Date.now() + timeoutMs })
      this.#timer = setTimeout(() => this.cancel('timeout', 'Behavior operation timed out.'), timeoutMs)
    }
    cancel(code, message) {
      if (this.#cancelled) return
      this.#cancelled = new BehaviorError(code, message)
      this.#controller.abort()
      this.#reject(this.#cancelled)
    }
    #check() {
      if (!this.#cancelled && performance.now() >= this.#expires) this.cancel('timeout', 'Behavior operation timed out.')
      if (this.#cancelled) throw this.#cancelled
    }
    async call(action, external = false) {
      this.#check()
      try {
        const value = await Promise.race([Promise.resolve().then(() => {
          this.#check()
          return action()
        }), this.#failure])
        this.#check()
        return value
      } catch (error) {
        if (this.#cancelled) throw this.#cancelled
        if (error instanceof BehaviorError || !external || storageFailure(error)) throw error
        throw new BehaviorError('adapter', 'Behavior adapter operation failed.')
      }
    }
    dispose() {
      clearTimeout(this.#timer)
    }
  }

  class SharedSetups {
    static #key = Symbol.for('tachyon.behavior.setups')
    static acquire(identity, options, configure) {
      globalThis[SharedSetups.#key] ??= new Map()
      const setups = globalThis[SharedSetups.#key]
      if (!setups.has(identity)) setups.set(identity, new Map())
      const selections = setups.get(identity)
      const selection = JSON.stringify([[...options.profiles].sort(), Object.keys(options.properties).sort().map((name) => [name, options.properties[name]])])
      if (!selections.has(selection)) {
        const shared = { configuration: configure(), users: 0 }
        shared.configuration.catch(() => { if (selections.get(selection) === shared) selections.delete(selection) })
        selections.set(selection, shared)
      }
      const shared = selections.get(selection)
      shared.users += 1
      const release = () => {
        shared.users -= 1
        if (shared.users === 0 && selections.get(selection) === shared) selections.delete(selection)
      }
      return { configuration: shared.configuration, release }
    }
  }

  class OpenTransactions {
    static #key = Symbol.for('tachyon.behavior.transactions')
    static #shared = OpenTransactions.#scope()
    static #scope() {
      const Storage = globalThis.process?.getBuiltinModule?.('node:async_hooks')?.AsyncLocalStorage
      if (!Storage) return null
      globalThis[OpenTransactions.#key] ??= new Storage()
      return globalThis[OpenTransactions.#key]
    }
    #local = Object.freeze({})
    current() {
      return OpenTransactions.#shared?.getStore() ?? this.#local
    }
    async within(open, run) {
      if (OpenTransactions.#shared) return OpenTransactions.#shared.run(open, run)
      const outer = this.#local
      this.#local = open
      try { return await run() } finally { this.#local = outer }
    }
  }

  class BehaviorContext {
    #configuration
    #partition
    #signal
    #timeoutMs
    #active = new Set()
    #slots = new Map()
    #evicted = new Set()
    #transactions = new OpenTransactions()
    #disposed = false
    #setup = null
    #onAbort
    constructor(options) {
      this.#partition = options.partition
      this.#signal = options.signal
      this.#timeoutMs = options.timeoutMs ?? 10000
      this.#onAbort = () => this.dispose()
      this.#signal.addEventListener('abort', this.#onAbort, { once: true })
      contexts.add(this)
    }
    async initialize(Configuration, options) {
      const record = records.get(Configuration)
      const identity = record.moduleOwned ? `${moduleId}:${record.name}` : `${environment}:${record.name}`
      this.#setup = SharedSetups.acquire(identity, options,
        () => this.#perform((operation) => operation.call(() => Configuration.configure(operation.execution), true)))
      const configuration = await this.#setup.configuration
      exactKeys(configuration, ['caches', 'http', 'entities', 'authorize', 'authorizeEntity'], 'configuration adapter object')
      if (configuration.authorize !== undefined && typeof configuration.authorize !== 'function') invalid('Behavior configuration authorization must be a function.')
      if (configuration.authorizeEntity !== undefined && typeof configuration.authorizeEntity !== 'function') invalid('Entity authorization must be a function.')
      this.#configuration = Object.freeze({
        caches: this.#adapters(configuration.caches),
        http: this.#adapters(configuration.http),
        entities: this.#adapters(configuration.entities),
        authorize: configuration.authorize,
        authorizeEntity: configuration.authorizeEntity,
      })
      return this
    }
    #adapters(adapters) {
      if (adapters === undefined) return Object.freeze({})
      if (!plain(adapters) || Reflect.ownKeys(adapters).length > 128) invalid('Invalid behavior adapter collection.')
      const result = {}
      for (const name of Reflect.ownKeys(adapters)) {
        if (!identifier(name)) invalid('Invalid behavior adapter name.')
        const descriptor = Object.getOwnPropertyDescriptor(adapters, name)
        if (!descriptor || !own(descriptor, 'value') || !descriptor.value) invalid('Invalid behavior adapter registration.')
        result[name] = descriptor.value
      }
      return Object.freeze(result)
    }
    #adapter(kind, name, method) {
      const adapter = this.#configuration?.[kind]?.[name] ?? builtinAdapter(kind, name)
      if (!adapter || typeof adapter[method] !== 'function') invalid('Required behavior adapter is not configured.')
      return adapter
    }
    #builtin(kind, name) {
      return this.#configuration?.[kind]?.[name] === undefined && builtinAdapter(kind, name) !== undefined
    }
    #assertReady() {
      if (this.#disposed) throw new BehaviorError('disposed', 'Behavior context is disposed.')
      if (this.#signal.aborted) throw new BehaviorError('aborted', 'Behavior operation was aborted.')
      if (this.#active.size >= MAX_ACTIVE) throw new BehaviorError('capacity', 'Behavior context concurrency bound reached.')
    }
    async #perform(action) {
      this.#assertReady()
      const operation = new Operation(this.#partition, this.#timeoutMs)
      this.#active.add(operation)
      try { return await operation.call(() => action(operation)) }
      finally { operation.dispose(); this.#active.delete(operation) }
    }
    async #authorize(operation, record, method, args, intent) {
      const authorize = this.#configuration?.authorize
      if (typeof authorize !== 'function' && this.#builtin('caches', record.cache.name)) return
      if (typeof authorize !== 'function') invalid('Cache access requires an authorization hook.')
      const allowed = await operation.call(() => authorize(Object.freeze({
        operation: intent, cache: record.cache.name, type: record.name, method: method.name,
        keyArguments: keyArguments(method, args),
      }), operation.execution), true)
      if (allowed !== true) throw new BehaviorError('unauthorized', 'Behavior cache access was not authorized.')
    }
    async invoke(owner, record, method, original, input) {
      return this.#perform(async (operation) => {
        const args = methodArguments(input, method)
        if (method.query) return this.#query(operation, method.query, args)
        const call = (revalidate = false) => this.#resilient(operation, record, method, () => this.#result(operation, owner, method, original, args, revalidate))
        if (method.cachePut || method.cacheEvict) return this.#mutate(operation, record, method, args, call)
        if (!method.cacheable) return call()
        await this.#authorize(operation, record, method, args, 'cache:read')
        return this.#cached(operation, record, method, args, call)
      })
    }
    paginate(owner, method, original, input) {
      this.#assertReady()
      const result = Reflect.apply(original, owner, methodArguments(input, method))
      if (result && typeof result.then === 'function') return this.#perform((operation) => operation.call(() => result))
      return result
    }
    #resilient(operation, record, method, run) {
      const transacted = () => this.#transacted(operation, method, run)
      const attempt = () => this.#retried(operation, method, transacted)
      if (!method.concurrencyLimit) return attempt()
      return this.#limited(operation, record, method, attempt)
    }
    async #transacted(operation, method, run) {
      const policy = method.transactional
      const open = this.#transactions.current()
      if (!policy || own(open, policy.adapter)) return run()
      const adapter = this.#adapter('entities', policy.adapter, 'execute')
      const control = (action, transaction) => operation.call(() => adapter.execute(
        freeze(transaction ? { operation: 'transaction', action, transaction } : { operation: 'transaction', action }),
        operation.execution), true)
      const begun = json(await control('begin'), 4096)
      const id = plain(begun) ? begun.transaction : undefined
      if (typeof id !== 'string' || !id.length || id.length > 128) invalid('A transactional adapter must answer a bounded transaction identifier.')
      let value
      try {
        value = await this.#transactions.within(freeze({ ...open, [policy.adapter]: id }), run)
      } catch (error) {
        try { await control('rollback', id) } catch {}
        throw error
      }
      await control('commit', id)
      return value
    }
    async #retried(operation, method, run) {
      const policy = method.retryable
      if (!policy) return run()
      for (let attempt = 0; ; attempt += 1) {
        try { return await run() }
        catch (error) {
          if (attempt >= policy.maxRetries || !retryable(error)) throw error
          const wait = backoff(policy, attempt)
          if (wait > 0) await operation.call(() => new Promise((resolve) => setTimeout(resolve, wait)))
        }
      }
    }
    async #limited(operation, record, method, run) {
      const policy = method.concurrencyLimit
      const key = record.name + '.' + method.name
      let slot = this.#slots.get(key)
      if (!slot) { slot = { active: 0, waiters: [] }; this.#slots.set(key, slot) }
      if (slot.active < policy.limit) slot.active += 1
      else if (policy.policy === 'reject') throw new BehaviorError('capacity', 'Behavior method concurrency limit reached.')
      else {
        const waiter = {}
        try { await operation.call(() => new Promise((resolve) => { waiter.resolve = resolve; slot.waiters.push(waiter) })) }
        catch (error) {
          if (waiter.entered) this.#release(slot, key)
          else slot.waiters = slot.waiters.filter((entry) => entry !== waiter)
          throw error
        }
      }
      try { return await run() }
      finally { this.#release(slot, key) }
    }
    #release(slot, key) {
      const next = slot.waiters.shift()
      if (next) { next.entered = true; next.resolve(); return }
      slot.active -= 1
      if (slot.active === 0) this.#slots.delete(key)
    }
    async #result(operation, owner, method, original, args, revalidate) {
      if (method.mapping) return mapValue(method.mapping, args)
      const metadata = method.httpExchange
      const result = metadata?.generated
        ? exchangeRequest(metadata, args)
        : await operation.call(() => Reflect.apply(original, owner, args))
      if (!metadata) return result
      const descriptor = requestDescriptor(result)
      if (['GET', 'HEAD'].includes(metadata.method) && own(descriptor, 'body')) invalid('Behavior HTTP reads cannot contain a request body.')
      const adapter = this.#adapter('http', metadata.client, 'exchange')
      const shared = method.cacheable && metadata.method === 'GET' ? { publicRead: true, ...(revalidate ? { revalidate: true } : {}) } : {}
      const command = Object.freeze({ ...descriptor, method: metadata.method, path: exchangePath(metadata, args),
        redirect: 'error', credentials: 'omit', signal: operation.execution.signal,
        deadline: operation.execution.deadline, maxResponseBytes: MAX_BYTES, ...shared })
      return json(await operation.call(() => adapter.exchange(command, operation.execution), true))
    }
    restClient(name) {
      if (!identifier(name)) invalid('RestClient requires a configured http adapter name.')
      const build = (method, path) => {
        if (!safePath(path)) invalid('RestClient requires a bounded origin-relative path without query, fragment or backslash.')
        const descriptor = { query: {}, headers: {} }
        const step = {
          query: (values) => { Object.assign(descriptor.query, json(values ?? {}, 4096)); return step },
          header: (key, value) => { descriptor.headers[key] = value; return step },
          body: (value) => {
            if (['GET', 'HEAD'].includes(method)) invalid('Behavior HTTP reads cannot contain a request body.')
            descriptor.body = value
            return step
          },
          retrieve: () => this.#perform(async (operation) => {
            const request = requestDescriptor(descriptor)
            const adapter = this.#adapter('http', name, 'exchange')
            const command = Object.freeze({ ...request, method, path,
              redirect: 'error', credentials: 'omit', signal: operation.execution.signal,
              deadline: operation.execution.deadline, maxResponseBytes: MAX_BYTES })
            return json(await operation.call(() => adapter.exchange(command, operation.execution), true))
          }),
        }
        return Object.freeze(step)
      }
      return Object.freeze(Object.fromEntries(['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']
        .map((method) => [method.toLowerCase(), (path) => build(method, path)])))
    }
    async #query(operation, policy, args) {
      const arity = Math.max(-1, policy.pageArgument ?? -1, ...policy.where.map((predicate) => predicate.argument ?? -1)) + 1
      if (args.length !== arity) invalid('Query requires its exact declared positional arguments.')
      const where = policy.where.map((predicate) => {
        const result = { field: predicate.field, operator: predicate.operator }
        if (own(predicate, 'argument')) {
          if (!own(args, predicate.argument)) invalid('Query argument is missing.')
          result.value = args[predicate.argument]
          const scalar = (value) => value === null || ['string', 'boolean'].includes(typeof value) || (typeof value === 'number' && Number.isFinite(value))
          const valid = predicate.operator === 'in'
            ? Array.isArray(result.value) && result.value.length <= 100 && result.value.every(scalar)
            : scalar(result.value)
          if (!valid) invalid('Query bindings require finite scalar data or a bounded In array.')
        }
        return result
      })
      const single = policy.operation === 'findOne'
      const query = { kind: single ? 'find' : policy.operation, where: json(where, 65536), orderBy: policy.orderBy }
      if (policy.operation === 'find') {
        const raw = policy.pageArgument === null ? {} : args[policy.pageArgument]
        const page = normalizePage(raw, { defaultSize: policy.defaultSize, maxSize: policy.maxSize, sort: policy.fields })
        if (page.sort.length && policy.orderBy.length) invalid('Query sort conflicts with its declared ordering.')
        query.page = page
      }
      if (single) query.page = normalizePage({ size: 1 }, { defaultSize: 1, maxSize: 1, sort: policy.fields })
      const command = freeze({ operation: 'query', entity: policy.entity.name, table: policy.entity.table, idField: policy.entity.id, query, ...this.#joined(policy.entity.adapter) })
      const authorize = this.#configuration?.authorizeEntity
      const builtin = this.#builtin('entities', policy.entity.adapter)
      if (typeof authorize !== 'function' && !builtin) invalid('Store query requires entity authorization.')
      const adapter = this.#adapter('entities', policy.entity.adapter, 'execute')
      if (typeof authorize === 'function' && await operation.call(() => authorize(command, operation.execution), true) !== true) throw new BehaviorError('unauthorized', 'Store query was not authorized.')
      const result = json(await operation.call(() => adapter.execute(command, operation.execution), true))
      if (query.kind === 'find' && (!Array.isArray(result) || result.length > query.page.size || result.some((row) => !plain(row)))) invalid('Query find result requires a bounded array of objects.')
      if (['count', 'delete'].includes(policy.operation) && !integer(result, 0, Number.MAX_SAFE_INTEGER)) invalid('Query count and delete results require a nonnegative safe integer.')
      if (single) return result[0] ?? null
      if (policy.operation === 'exists' && typeof result !== 'boolean') invalid('Query exists result requires a boolean.')
      return result
    }
    async #cached(operation, record, method, args, call) {
      const policy = record.cache
      const key = cacheKey(this.#partition, record, method, args)
      const adapter = this.#adapter('caches', policy.name, 'get')
      const hit = json(await operation.call(() => adapter.get(key, operation.execution), true), policy.maxBytes + 512)
      exactKeys(hit, ['hit', 'value', 'expiresAt'], 'cache response')
      if (typeof hit.hit !== 'boolean') invalid('Invalid behavior cache response.')
      if (hit.hit && this.#liveEntry(hit, policy)) return json(hit.value, policy.maxBytes)
      const value = json(await call(this.#evicted.has(key)), policy.maxBytes)
      this.#evicted.delete(key)
      const writer = this.#adapter('caches', policy.name, 'set')
      const execution = Object.freeze({ ...operation.execution, ...policy, expiresAt: Date.now() + policy.ttlMs })
      await operation.call(() => writer.set(key, json(value, policy.maxBytes), execution), true)
      return value
    }
    async #mutate(operation, record, method, args, call) {
      const mutation = method.cachePut || method.cacheEvict
      const target = record.methods.find((entry) => entry.name === mutation.method)
      const mapped = mutation.arguments.map((index) => {
        if (!own(args, index) || args[index] === undefined) invalid('Cache mutation argument is missing.')
        return args[index]
      })
      const normalized = methodArguments(mapped, { pageable: target.pageable })
      const key = cacheKey(this.#partition, record, target, normalized)
      const policy = record.cache
      const adapter = this.#adapter('caches', policy.name, method.cachePut ? 'set' : 'delete')
      await this.#authorize(operation, record, target, normalized, method.cachePut ? 'cache:put' : 'cache:evict')
      const value = json(await call(), method.cachePut ? policy.maxBytes : MAX_BYTES)
      if (method.cachePut) {
        const execution = Object.freeze({ ...operation.execution, ...policy, expiresAt: Date.now() + policy.ttlMs })
        await operation.call(() => adapter.set(key, json(value, policy.maxBytes), execution), true)
      } else {
        await operation.call(() => adapter.delete(key, operation.execution), true)
      }
      this.#rememberEviction(key)
      return value
    }
    #rememberEviction(key) {
      this.#evicted.delete(key)
      this.#evicted.add(key)
      if (this.#evicted.size > MAX_EVICTED) this.#evicted.delete(this.#evicted.values().next().value)
    }
    #joined(adapter) {
      const open = this.#transactions.current()
      return own(open, adapter) ? { transaction: open[adapter] } : {}
    }
    #liveEntry(hit, policy) {
      if (!Number.isFinite(hit.expiresAt)) invalid('Cached behavior value requires a finite expiry.')
      const now = Date.now()
      if (hit.expiresAt > now + policy.ttlMs) invalid('Cached behavior expiry exceeds the policy bound.')
      return hit.expiresAt > now
    }
    async evict(instance, methodName, input = []) {
      if (!instance || instance.behaviors !== this) invalid('Cache eviction requires the owning behavior context.')
      const record = records.get(Object.getPrototypeOf(instance)?.constructor)
      const method = record?.methods.find((item) => item.name === methodName)
      if (!method?.cacheable) invalid('Cache eviction requires a Cached method.')
      return this.#perform(async (operation) => {
        const args = methodArguments(input, method)
        await this.#authorize(operation, record, method, args, 'cache:evict')
        const key = cacheKey(this.#partition, record, method, args)
        const adapter = this.#adapter('caches', record.cache.name, 'delete')
        await operation.call(() => adapter.delete(key, operation.execution), true)
        this.#rememberEviction(key)
      })
    }
    repository(Type) {
      const record = records.get(Type)
      if (!record?.entity) invalid('Store requires a registered behavior entity.')
      const run = (details) => this.#perform(async (operation) => {
        const adapter = this.#adapter('entities', record.entity.adapter, 'execute')
        const command = freeze({ entity: record.name, table: record.entity.table, idField: record.entity.id, ...details, ...this.#joined(record.entity.adapter) })
        return json(await operation.call(() => adapter.execute(command, operation.execution), true))
      })
      return Object.freeze({
        find: async (page = {}) => run({ operation: 'find', page: normalizePage(page, { defaultSize: 20, maxSize: 100, sort: [record.entity.id] }) }),
        save: async (value) => {
          const data = entityData(value, Type)
          entityId(data[record.entity.id])
          return run({ operation: 'save', value: data })
        },
        delete: async (id) => run({ operation: 'delete', id: entityId(id) }),
      })
    }
    dispose() {
      if (this.#disposed) return
      this.#disposed = true
      this.#setup?.release()
      this.#signal.removeEventListener('abort', this.#onAbort)
      for (const operation of this.#active) operation.cancel('disposed', 'Behavior context is disposed.')
    }
  }
  function declareStorage(record) {
    const storage = environment === 'tac' ? globalThis[Symbol.for('tachyon.tac.storage')] : undefined
    if (!storage) return
    if (record.entity?.adapter === 'indexeddb') storage.declare(record.entity.table, record.entity.id, [])
    for (const method of record.methods) {
      const query = method.query
      if (query?.entity.adapter !== 'indexeddb') continue
      storage.declare(query.entity.table, query.entity.id, [...query.where, ...query.orderBy].map((entry) => entry.field))
    }
  }
  function storageFailure(error) {
    const storage = environment === 'tac' ? globalThis[Symbol.for('tachyon.tac.storage')] : undefined
    return Boolean(storage) && error instanceof storage.failure
  }
  function builtinAdapter(kind, name) {
    const storage = environment === 'tac' ? globalThis[Symbol.for('tachyon.tac.storage')] : undefined
    if (!storage) return undefined
    if (kind === 'caches') return storage.cache(name)
    if (kind === 'entities' && name === 'indexeddb') return storage.entities()
    return undefined
  }
  function entityId(value) {
    if ((typeof value === 'string' && value.length > 0 && value.length <= 256) || Number.isSafeInteger(value)) return value
    invalid('Behavior entity requires a bounded string or integer identifier.')
  }
  function entityData(value, Type) {
    if (value && Object.getPrototypeOf(value) === Type.prototype) {
      const output = Object.create(null)
      for (const key of Reflect.ownKeys(value)) {
        const descriptor = Object.getOwnPropertyDescriptor(value, key)
        if (typeof key !== 'string' || !descriptor || !own(descriptor, 'value')) invalid('Invalid behavior entity fields.')
        if (descriptor.enumerable) Object.defineProperty(output, key, { enumerable: true, value: descriptor.value })
      }
      return json(output)
    }
    const data = json(value)
    if (!plain(data)) invalid('Behavior entity data must be an object.')
    return data
  }

  function validateCache(policy) {
    if (!plain(policy) || !identifier(policy.name) || !integer(policy.ttlMs, 1, 86400000)
      || !integer(policy.maxBytes, 1, MAX_BYTES) || !integer(policy.maxEntries, 1, 10000)) invalid('Invalid behavior cache policy.')
  }
  function validateValues(policy) {
    if (!plain(policy) || Object.keys(policy).length > 64) invalid('Invalid behavior value bindings.')
    for (const [field, descriptor] of Object.entries(policy)) {
      if (!identifier(field) || !plain(descriptor) || !['string', 'boolean', 'integer', 'number'].includes(descriptor.type)
        || typeof descriptor.path !== 'string' || descriptor.path.length > 256
        || !descriptor.path.startsWith(environment + '.') || !/^(tac|yon)(\.[a-z][a-z0-9]*)+$/.test(descriptor.path)) invalid('Invalid behavior value bindings.')
    }
  }
  function validateGateway(policy) {
    const key = typeof policy.key === 'string' ? policy.key.split('.') : []
    if (key.length !== 2 || !key.every(identifier) || !integer(policy.arity, 0, 32)) invalid('Invalid behavior gateway policy.')
  }
  async function callGateway(policy, args) {
    const failed = (kind) => new BehaviorError(kind, 'Gateway call failed.')
    if (args.length !== policy.arity) throw failed('invalid')
    const transport = globalThis.YonGateway
    if (typeof transport?.call !== 'function') throw failed('adapter')
    const answer = await transport.call(policy.key, args)
    if (plain(answer) && own(answer, 'result')) return answer.result
    const kind = ['invalid', 'timeout', 'capacity', 'adapter'].includes(answer?.error) ? answer.error : 'adapter'
    throw failed(kind)
  }
  function validateMethod(method, record) {
    if (!identifier(method.name)) invalid('Invalid behavior method name.')
    if (method.gateway) validateGateway(method.gateway)
    if (method.cacheable) validateCacheable(method, record)
    if (method.cachePut || method.cacheEvict) validateMutation(method, record)
    if (method.query) validateQuery(method)
    if (method.pageable) validatePagePolicy(method.pageable)
    if (method.retryable) validateRetryable(method.retryable)
    if (method.transactional && !identifier(method.transactional.adapter)) invalid('Invalid behavior transaction policy.')
    if (method.concurrencyLimit) validateConcurrencyLimit(method.concurrencyLimit)
    if (method.mapping) validateMapping(method.mapping)
    if (method.httpExchange) {
      const exchange = method.httpExchange
      if (!identifier(exchange.client) || !safePath(exchange.path) || !['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'].includes(exchange.method)) invalid('Invalid behavior HTTP exchange policy.')
      for (const mapping of [exchange.query, exchange.headers, exchange.pathVariables]) {
        if (mapping !== undefined && (!plain(mapping) || Object.values(mapping).some((index) => !integer(index, 0, 31)))) invalid('Invalid request argument mapping.')
      }
      if (exchange.body !== undefined && !integer(exchange.body, 0, 31)) invalid('Invalid request body argument.')
    }
  }
  function validateMapping(policy) {
    if (!plain(policy) || !integer(policy.argument, 0, 31)) invalid('Invalid behavior mapping policy.')
    if (policy.result !== undefined && !identifier(policy.result)) invalid('Invalid behavior mapping result model.')
    validateMappingFields(policy.fields, policy.result === undefined ? 1 : 0, 0)
  }
  function validateMappingFields(fields, least, depth) {
    if (depth > 8 || !Array.isArray(fields) || fields.length < least || fields.length > 256) invalid('Invalid behavior mapping policy.')
    const targets = new Set()
    for (const field of fields) {
      if (!plain(field) || typeof field.target !== 'string' || !field.target || field.target.length > 256) invalid('Invalid behavior mapping target.')
      if (targets.size === targets.add(field.target).size) invalid('Behavior mapping writes one target twice.')
      const constant = own(field, 'constant')
      if (constant === (field.source !== undefined)) invalid('A behavior mapping takes one of source or constant.')
      if (constant) { if (field.constant !== null && !['string', 'number', 'boolean'].includes(typeof field.constant)) invalid('Invalid behavior mapping constant.') }
      else {
        const segments = String(field.source).split('.')
        if (segments.length > 8 || segments.some((name) => !name || name.length > 256)) invalid('Invalid behavior mapping source path.')
      }
      if (field.fields !== undefined) {
        if (constant) invalid('A behavior mapping constant has no nested fields.')
        validateMappingFields(field.fields, 0, depth + 1)
      }
    }
  }
  function mapValue(policy, args) {
    const input = args[policy.argument]
    if (!plain(input) || Array.isArray(input)) invalid('A behavior mapping reads one JSON object argument.')
    return json(mapFields(input, policy.fields))
  }
  function mapFields(input, fields) {
    const output = {}
    for (const field of fields) {
      if (own(field, 'constant')) { output[field.target] = field.constant; continue }
      let value = input
      for (const name of String(field.source).split('.')) value = plain(value) && own(value, name) ? value[name] : null
      output[field.target] = field.fields ? shapedValue(value, field.fields) : value
    }
    return output
  }
  function shapedValue(value, fields) {
    if (Array.isArray(value)) return value.map((item) => shapedValue(item, fields))
    return plain(value) ? mapFields(value, fields) : value
  }
  function validateCacheable(method, record) {
    const keys = method.cacheable.keyArgs
    if (!record.cache || !Array.isArray(keys) || keys.length > 32) invalid('Cached requires a cache policy and bounded key arguments.')
    if (keys.some((index) => !integer(index, 0, 31)) || new Set(keys).size !== keys.length) invalid('Invalid Cached key arguments.')
    if (['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'].includes(method.name)) invalid('Cached cannot wrap HTTP verb endpoints.')
  }
  function validateMutation(method, record) {
    if (method.cacheable || method.pageable || (method.cachePut && method.cacheEvict)
      || ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'].includes(method.name)) invalid('Invalid cache mutation combination.')
    const policy = method.cachePut || method.cacheEvict
    if (!plain(policy) || !identifier(policy.method) || !Array.isArray(policy.arguments)
      || policy.arguments.length > 32 || policy.arguments.some((index) => !integer(index, 0, 31))) invalid('Invalid cache mutation policy.')
    const target = record.methods.find((entry) => entry.name === policy.method)
    if (!target?.cacheable) invalid('Cache mutation requires a source-local Cached target.')
    validateCacheable(target, record)
    if (target.cacheable.keyArgs.some((index) => index >= policy.arguments.length)
      || (target.pageable && target.pageable.argument >= policy.arguments.length)) invalid('Cache mutation mapping omits target arguments.')
  }
  function validateQuery(method) {
    const query = method.query
    if (method.cacheable || method.cachePut || method.cacheEvict || method.pageable || method.httpExchange) invalid('Query cannot combine other method behaviors.')
    if (!plain(query) || !plain(query.entity) || !identifier(query.entity.name)
      || !identifier(query.entity.adapter) || !identifier(query.entity.table) || !identifier(query.entity.id)
      || !['find', 'findOne', 'count', 'exists', 'delete'].includes(query.operation)
      || !Array.isArray(query.fields) || query.fields.length > 64 || !query.fields.includes(query.entity.id)
      || query.fields.some((field) => !identifier(field))
      || !integer(query.defaultSize, 1, 100) || !integer(query.maxSize, query.defaultSize, 100)
      || !(query.pageArgument === null || integer(query.pageArgument, 0, 31))
      || !Array.isArray(query.where) || query.where.length > 8
      || !Array.isArray(query.orderBy) || query.orderBy.length > 5) invalid('Invalid repository query policy.')
    for (const predicate of query.where) {
      const nullary = ['isNull', 'isNotNull'].includes(predicate.operator)
      if (!query.fields.includes(predicate.field) || !['eq', 'ne', 'lt', 'lte', 'gt', 'gte', 'in', 'isNull', 'isNotNull'].includes(predicate.operator)
        || (nullary ? own(predicate, 'argument') : !integer(predicate.argument, 0, 31))) invalid('Invalid query predicate.')
    }
    if (query.orderBy.some((entry) => !query.fields.includes(entry.field) || !['asc', 'desc'].includes(entry.direction))) invalid('Invalid query ordering.')
    if (query.operation !== 'find' && (query.pageArgument !== null || query.orderBy.length)) invalid('FindOne, count, exists and delete cannot request pagination or ordering.')
  }
  function validateRetryable(policy) {
    if (!plain(policy) || !integer(policy.maxRetries, 1, 8) || !integer(policy.delayMs, 0, 60000)
      || !integer(policy.maxDelayMs, policy.delayMs, 600000) || !integer(policy.jitterMs, 0, 60000)
      || typeof policy.multiplier !== 'number' || !(policy.multiplier >= 1 && policy.multiplier <= 10)) invalid('Invalid behavior retry policy.')
  }
  function validateConcurrencyLimit(policy) {
    if (!plain(policy) || !integer(policy.limit, 1, 64) || !['block', 'reject'].includes(policy.policy)) invalid('Invalid behavior concurrency policy.')
  }
  function validatePagePolicy(page) {
    if (!integer(page.argument, 0, 31) || !integer(page.defaultSize, 1, 1000) || !integer(page.maxSize, page.defaultSize, 1000)
      || !Array.isArray(page.sort) || page.sort.length > 32 || page.sort.some((name) => !identifier(name))) invalid('Invalid behavior pagination policy.')
  }
  function validateEntity(entity) {
    if (!identifier(entity.adapter) || !identifier(entity.id) || !identifier(entity.table)) invalid('Invalid behavior entity mapping.')
  }
  function validateRequired(fields) {
    if (!Array.isArray(fields) || fields.length > 32) invalid('Invalid behavior required constructor arguments.')
    if (fields.some((name) => !identifier(name) && name !== 'behaviors') || new Set(fields).size !== fields.length) invalid('Invalid behavior required constructor arguments.')
  }
  function validateMethods(type, record) {
    if (!Array.isArray(record.methods) || record.methods.length > 128) invalid('Invalid behavior method collection.')
    const seen = new Set()
    for (const method of record.methods) {
      validateMethod(method, record)
      if (seen.has(method.name)) invalid('Duplicate behavior method registration.')
      seen.add(method.name)
      const descriptor = Object.getOwnPropertyDescriptor(type.prototype, method.name)
      if (!descriptor || typeof descriptor.value !== 'function' || !descriptor.configurable) invalid('Behavior annotations require an own configurable instance method.')
    }
  }
  function register(raw) {
    if (!raw || typeof raw.type !== 'function' || !identifier(raw.name) || records.has(raw.type)) invalid('Invalid behavior type registration.')
    const { type, ...metadata } = raw
    const record = freeze(json(metadata, 65536))
    if (record.configuration && typeof Object.getOwnPropertyDescriptor(type, 'configure')?.value !== 'function') invalid('Behavior configuration requires static configure().')
    if (record.cache) validateCache(record.cache)
    if (record.entity) validateEntity(record.entity)
    if (record.requiredArgs) validateRequired(record.requiredArgs)
    if (record.values) validateValues(record.values)
    validateMethods(type, record)
    declareStorage(record)
    records.set(type, record)
    return { type, record }
  }
  function install({ type, record }) {
    for (const method of record.methods) {
      const descriptor = Object.getOwnPropertyDescriptor(type.prototype, method.name)
      const original = descriptor.value
      if (method.gateway) {
        const policy = method.gateway
        Object.defineProperty(type.prototype, method.name, { ...descriptor,
          value: async function (...args) { return callGateway(policy, args) } })
        continue
      }
      if (method.mapping && Object.keys(method).every((key) => ['name', 'mapping'].includes(key))) {
        const policy = method.mapping
        Object.defineProperty(type.prototype, method.name, { ...descriptor,
          value: async function (...args) { return mapValue(policy, args) } })
        continue
      }
      if (method.valid && Object.keys(method).every((key) => ['name', 'valid'].includes(key))) {
        const validate = (owner, args) => Reflect.apply(original, owner, methodArguments(args, method))
        const asynchronous = Object.getPrototypeOf(original) === Object.getPrototypeOf(async function () {})
        Object.defineProperty(type.prototype, method.name, { ...descriptor, value: asynchronous
          ? async function (...args) { return validate(this, args) }
          : function (...args) { return validate(this, args) } })
        continue
      }
      const contextFor = (owner) => {
        const context = owner?.behaviors
        if (!contexts.has(context)) invalid('Annotated method requires its source-local behavior context.')
        return context
      }
      const asynchronous = async function (...args) {
        return contextFor(this).invoke(this, record, method, original, args)
      }
      const synchronous = function (...args) {
        return contextFor(this).paginate(this, method, original, args)
      }
      Object.defineProperty(type.prototype, method.name, { ...descriptor,
        value: method.cacheable || method.cachePut || method.cacheEvict || method.httpExchange || method.query
          || method.retryable || method.concurrencyLimit || method.transactional || method.mapping ? asynchronous : synchronous })
    }
  }
  function createOptions(options) {
    exactKeys(options, ['partition', 'signal', 'timeoutMs', 'profiles', 'properties'], 'context options')
    if (typeof options.partition !== 'string' || !options.partition.trim() || encoder.encode(options.partition).byteLength > 256) invalid('Behavior context requires a bounded nonempty partition.')
    if (!signalLike(options.signal)) invalid('Behavior context requires an AbortSignal.')
    if (!integer(options.timeoutMs ?? 10000, 1, 60000)) invalid('Invalid behavior operation timeout.')
    if (options.signal.aborted) throw new BehaviorError('aborted', 'Behavior operation was aborted.')
    return { ...options, profiles: configurationProfiles(options.profiles ?? []), properties: propertySource(options.properties ?? {}, 'configuration') }
  }
  if (!['tac', 'yon'].includes(environment) || !Array.isArray(registrations) || registrations.length > 128) invalid('Invalid behavior registration environment.')
  if (typeof moduleId !== 'string' || !/^[a-f0-9]{64}$/.test(moduleId)) invalid('Behavior registration requires a compiler-generated module identity.')
  registrations.map(register).forEach(install)
  return Object.freeze({
    called: (key) => { calledKeys.add(key) },
    takeCalled: () => {
      const keys = [...calledKeys]
      calledKeys.clear()
      return keys
    },
    create: async (Configuration, options) => {
      if (!records.get(Configuration)?.configuration) invalid('Behaviors.create requires a registered Setup.')
      const selected = createOptions(options)
      selectedConfiguration(records.get(Configuration), selected)
      const context = new BehaviorContext(selected)
      try { return await context.initialize(Configuration, selected) }
      catch (error) { context.dispose(); throw error }
    },
    properties: (Type, values) => bindProperties(Type, values),
    values: (Type, application) => bindValues(Type, application),
    validate: (Type, value) => validateData(records.get(Type)?.validated, value),
    constructWith: (Type, context, mocks) => {
      if (typeof Type !== 'function') invalid('A test unit constructs a class.')
      const fields = records.get(Type)?.requiredArgs ?? []
      if (!contexts.has(context)) invalid('A test unit constructs roles with its own behavior context.')
      const args = fields.map((name) => {
        if (own(mocks, name) && mocks[name] !== undefined) return mocks[name]
        if (name === 'behaviors') return context
        invalid(`A constructed behavior type needs '${name}'; declare it as a Fake.`)
      })
      const instance = Reflect.construct(Type, args)
      for (const name of Object.keys(mocks).filter((name) => !fields.includes(name))) {
        if (!own(instance, name) || instance[name] !== undefined) invalid(`Fake '${name}' names no Needs field on this type.`)
        instance[name] = mocks[name]
      }
      return instance
    },
    construct: (Type, values) => {
      const fields = records.get(Type)?.requiredArgs
      if (!fields) invalid('Behaviors.construct requires NeedsAll.')
      exactKeys(values, fields, 'constructor arguments')
      const args = fields.map((name) => {
        const descriptor = Object.getOwnPropertyDescriptor(values, name)
        if (!descriptor || !own(descriptor, 'value') || descriptor.value === undefined) invalid('Required behavior constructor argument is missing.')
        return descriptor.value
      })
      return Reflect.construct(Type, args)
    },
  })
}

class TacStorageError extends Error {
  name = 'TacStorageError'
}

class TacCacheStorage {
  static namePrefix = 'tachyon-cache:'
  static pathPrefix = '/.tachyon/cache/'
  static expiresHeader = 'x-tachyon-expires-at'
  static usedHeader = 'x-tachyon-used-at'
  static maxPreloadEntries = 10000
  static memory = new Map()
  static adapters = new Map()
  static pending = new Map()
  static draining = null
  static loading = null
  static noticed = false

  static available() {
    try { return globalThis.isSecureContext === true && typeof globalThis.caches?.open === 'function' } catch { return false }
  }

  static notice() {
    if (TacCacheStorage.noticed) return
    TacCacheStorage.noticed = true
    console.info('Tachyon keeps Cached values in memory only on this page, because the browser Cache API needs HTTPS or localhost.')
  }

  static preload() {
    TacCacheStorage.loading ??= TacCacheStorage.load().catch(() => {})
    return TacCacheStorage.loading
  }

  static async load() {
    if (!TacCacheStorage.available()) { TacCacheStorage.notice(); return }
    for (const cacheName of await caches.keys()) {
      if (!cacheName.startsWith(TacCacheStorage.namePrefix)) continue
      const name = cacheName.slice(TacCacheStorage.namePrefix.length)
      const cache = await caches.open(cacheName)
      const found = []
      for (const request of (await cache.keys()).slice(0, TacCacheStorage.maxPreloadEntries)) {
        const entry = await TacCacheStorage.readResponse(name, request, await cache.match(request))
        if (entry) found.push(entry)
        else void cache.delete(request).catch(() => {})
      }
      found.sort((left, right) => left.usedAt - right.usedAt)
      const entries = TacCacheStorage.entries(name)
      for (const entry of found) if (!entries.has(entry.key)) entries.set(entry.key, { value: entry.value, expiresAt: entry.expiresAt })
    }
  }

  static async readResponse(name, request, response) {
    const prefix = `${TacCacheStorage.pathPrefix}${encodeURIComponent(name)}/`
    const path = new URL(request.url).pathname
    const expiresAt = Number(response?.headers.get(TacCacheStorage.expiresHeader))
    if (!response || !path.startsWith(prefix) || !Number.isFinite(expiresAt) || expiresAt <= Date.now()) return null
    try {
      return {
        key: decodeURIComponent(path.slice(prefix.length)),
        value: JSON.parse(await response.text()),
        expiresAt,
        usedAt: Number(response.headers.get(TacCacheStorage.usedHeader)) || 0,
      }
    } catch { return null }
  }

  static entries(name) {
    let entries = TacCacheStorage.memory.get(name)
    if (!entries) { entries = new Map(); TacCacheStorage.memory.set(name, entries) }
    return entries
  }

  static adapter(name) {
    let adapter = TacCacheStorage.adapters.get(name)
    if (adapter) return adapter
    adapter = Object.freeze({
      get: (key) => TacCacheStorage.read(name, key),
      set: (key, value, execution) => TacCacheStorage.write(name, key, value, execution),
      delete: (key) => TacCacheStorage.remove(name, key),
    })
    TacCacheStorage.adapters.set(name, adapter)
    return adapter
  }

  static read(name, key) {
    const entries = TacCacheStorage.entries(name)
    const entry = entries.get(key)
    if (!entry) return { hit: false }
    if (entry.expiresAt <= Date.now()) { TacCacheStorage.remove(name, key); return { hit: false } }
    entries.delete(key)
    entries.set(key, entry)
    return { hit: true, value: entry.value, expiresAt: entry.expiresAt }
  }

  static write(name, key, value, execution) {
    const entries = TacCacheStorage.entries(name)
    const entry = { value, expiresAt: execution.expiresAt }
    entries.delete(key)
    entries.set(key, entry)
    TacCacheStorage.schedule(name, key, entry)
    while (entries.size > execution.maxEntries) {
      const [oldest] = entries.keys()
      TacCacheStorage.remove(name, oldest)
    }
  }

  static remove(name, key) {
    TacCacheStorage.entries(name).delete(key)
    TacCacheStorage.schedule(name, key, null)
  }

  static schedule(name, key, entry) {
    if (!TacCacheStorage.available()) return
    const id = JSON.stringify([name, key])
    TacCacheStorage.pending.delete(id)
    TacCacheStorage.pending.set(id, { name, key, entry })
    TacCacheStorage.draining ??= TacCacheStorage.drain()
  }

  static async drain() {
    try {
      await TacCacheStorage.preload()
      while (TacCacheStorage.pending.size) {
        const [id, change] = TacCacheStorage.pending.entries().next().value
        TacCacheStorage.pending.delete(id)
        try { await TacCacheStorage.persist(change) } catch {}
      }
    } finally { TacCacheStorage.draining = null }
  }

  static async persist({ name, key, entry }) {
    const cache = await caches.open(`${TacCacheStorage.namePrefix}${name}`)
    const url = new URL(`${TacCacheStorage.pathPrefix}${encodeURIComponent(name)}/${encodeURIComponent(key)}`, location.origin)
    if (!entry) { await cache.delete(url); return }
    await cache.put(url, new Response(JSON.stringify(entry.value), {
      headers: {
        'content-type': 'application/json',
        [TacCacheStorage.expiresHeader]: String(entry.expiresAt),
        [TacCacheStorage.usedHeader]: String(Date.now()),
      },
    }))
  }

  static settled() {
    return TacCacheStorage.draining ?? Promise.resolve()
  }
}

class TacEntityStorage {
  static databaseName = 'tachyon-store'
  static scanLimit = 10000
  static maxTransactions = 64
  static wanted = new Map()
  static current = null
  static checked = ''
  static queue = Promise.resolve()
  static transactions = new Map()
  static counter = 0
  static adapter = Object.freeze({ execute: (command) => TacEntityStorage.execute(command) })

  static declare(table, id, fields) {
    const entry = TacEntityStorage.wanted.get(table) ?? { id, indexes: new Set() }
    if (entry.id !== id) {
      throw new TacStorageError(`Two entities keep rows in the IndexedDB store '${table}' but use different id fields ('${entry.id}' and '${id}'). Give one of them its own Table.`)
    }
    for (const field of fields) if (field !== id) entry.indexes.add(field)
    TacEntityStorage.wanted.set(table, entry)
  }

  static schemaKey() {
    return JSON.stringify([...TacEntityStorage.wanted].map(([table, entry]) => [table, entry.id, [...entry.indexes].sort()]).sort())
  }

  static database() {
    const next = TacEntityStorage.queue.then(() => TacEntityStorage.ensure())
    TacEntityStorage.queue = next.catch(() => undefined)
    return next
  }

  static async ensure() {
    const key = TacEntityStorage.schemaKey()
    if (TacEntityStorage.current && TacEntityStorage.checked === key) return TacEntityStorage.current
    if (typeof globalThis.indexedDB?.open !== 'function') throw new TacStorageError('This browser has no IndexedDB, so an entity with the "indexeddb" adapter cannot be stored here.')
    let database = TacEntityStorage.current ?? TacEntityStorage.watch(await TacEntityStorage.open())
    if (TacEntityStorage.missing(database)) {
      const version = database.version + 1
      database.close()
      database = TacEntityStorage.watch(await TacEntityStorage.open(version))
    }
    TacEntityStorage.current = database
    TacEntityStorage.checked = key
    return database
  }

  static open(version) {
    return new Promise((resolve, reject) => {
      const request = version === undefined
        ? indexedDB.open(TacEntityStorage.databaseName)
        : indexedDB.open(TacEntityStorage.databaseName, version)
      request.onupgradeneeded = () => {
        try { TacEntityStorage.upgrade(request.result, request.transaction) } catch (error) { request.transaction.abort(); reject(error) }
      }
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }

  static watch(database) {
    database.onversionchange = () => {
      database.close()
      if (TacEntityStorage.current === database) TacEntityStorage.current = null
    }
    return database
  }

  static missing(database) {
    for (const [table, entry] of TacEntityStorage.wanted) {
      if (!database.objectStoreNames.contains(table)) return true
      const store = database.transaction(table, 'readonly').objectStore(table)
      TacEntityStorage.checkKey(table, store.keyPath, entry.id)
      for (const field of entry.indexes) if (!store.indexNames.contains(field)) return true
    }
    return false
  }

  static upgrade(database, transaction) {
    for (const [table, entry] of TacEntityStorage.wanted) {
      const store = database.objectStoreNames.contains(table)
        ? transaction.objectStore(table)
        : database.createObjectStore(table, { keyPath: entry.id })
      TacEntityStorage.checkKey(table, store.keyPath, entry.id)
      for (const field of entry.indexes) if (!store.indexNames.contains(field)) store.createIndex(field, field)
    }
  }

  static checkKey(table, stored, wanted) {
    if (stored === wanted) return
    throw new TacStorageError(`The IndexedDB store '${table}' keeps its rows by '${stored}', but the entity now uses '${wanted}' as its Id. Tachyon never deletes stored rows, so it stops here. Keep the old Id field, or give the entity a new Table name.`)
  }

  static finished(transaction) {
    return new Promise((resolve, reject) => {
      transaction.oncomplete = () => resolve()
      transaction.onerror = () => reject(transaction.error)
      transaction.onabort = () => reject(transaction.error ?? new Error('The IndexedDB transaction was aborted.'))
    })
  }

  static answer(request) {
    return new Promise((resolve, reject) => {
      request.onsuccess = () => resolve(request.result)
      request.onerror = () => reject(request.error)
    })
  }

  static async execute(command) {
    if (command.operation === 'transaction') return TacEntityStorage.transaction(command)
    TacEntityStorage.declare(command.table, command.idField, [])
    const pending = command.transaction === undefined ? null : TacEntityStorage.transactions.get(command.transaction)
    if (command.transaction !== undefined && !pending) throw new TacStorageError('The IndexedDB transaction is not open.')
    if (command.operation === 'save') return TacEntityStorage.save(command, pending)
    if (command.operation === 'delete') return TacEntityStorage.delete(command, pending)
    if (command.operation === 'find') {
      const rows = await TacEntityStorage.rows(command, [], pending)
      return TacEntityStorage.page(rows, command.page, [], command.idField)
    }
    if (command.operation === 'query') return TacEntityStorage.query(command, pending)
    throw new TacStorageError(`The IndexedDB adapter does not know the operation '${command.operation}'.`)
  }

  static transaction(command) {
    if (command.action === 'begin') {
      if (TacEntityStorage.transactions.size >= TacEntityStorage.maxTransactions) throw new TacStorageError('Too many IndexedDB transactions are open at once.')
      const id = `indexeddb-${++TacEntityStorage.counter}`
      TacEntityStorage.transactions.set(id, new Map())
      return { transaction: id }
    }
    const writes = TacEntityStorage.transactions.get(command.transaction)
    TacEntityStorage.transactions.delete(command.transaction)
    if (!writes) throw new TacStorageError('The IndexedDB transaction is not open.')
    if (command.action === 'commit') return TacEntityStorage.commit(writes).then(() => { TacEntityMirror.apply(writes); return null })
    return null
  }

  static async commit(writes) {
    if (!writes.size) return null
    const database = await TacEntityStorage.database()
    const transaction = database.transaction([...writes.keys()], 'readwrite')
    const done = TacEntityStorage.finished(transaction)
    for (const [table, rows] of writes) {
      const store = transaction.objectStore(table)
      for (const [id, row] of rows) {
        if (row === null) store.delete(id)
        else store.put(row)
      }
    }
    await done
    return null
  }

  static staged(pending, table) {
    let rows = pending.get(table)
    if (!rows) { rows = new Map(); pending.set(table, rows) }
    return rows
  }

  static async save(command, pending) {
    if (pending) { TacEntityStorage.staged(pending, command.table).set(command.value[command.idField], command.value); return command.value }
    const database = await TacEntityStorage.database()
    const transaction = database.transaction(command.table, 'readwrite')
    const done = TacEntityStorage.finished(transaction)
    transaction.objectStore(command.table).put(command.value)
    await done
    TacEntityMirror.apply(new Map([[command.table, new Map([[command.value[command.idField], command.value]])]]))
    return command.value
  }

  static async delete(command, pending) {
    if (pending) { TacEntityStorage.staged(pending, command.table).set(command.id, null); return true }
    const database = await TacEntityStorage.database()
    const transaction = database.transaction(command.table, 'readwrite')
    const done = TacEntityStorage.finished(transaction)
    transaction.objectStore(command.table).delete(command.id)
    await done
    TacEntityMirror.apply(new Map([[command.table, new Map([[command.id, null]])]]))
    return true
  }

  static async rows(command, where, pending) {
    const database = await TacEntityStorage.database()
    const store = database.transaction(command.table, 'readonly').objectStore(command.table)
    const indexed = where.find((predicate) => TacEntityStorage.usesIndex(store, predicate))
    const source = indexed ? store.index(indexed.field) : store
    const range = indexed ? TacEntityStorage.range(indexed) : null
    const found = await TacEntityStorage.answer(source.getAll(range, TacEntityStorage.scanLimit + 1))
    if (found.length > TacEntityStorage.scanLimit) {
      throw new TacStorageError(`The IndexedDB store '${command.table}' has more than ${TacEntityStorage.scanLimit} rows to read for this query. Add a Store query that compares a string or number field, so Tachyon can use that field's index.`)
    }
    const staged = pending?.get(command.table)
    if (!staged) return found
    const merged = new Map(found.map((row) => [row[command.idField], row]))
    for (const [id, row] of staged) {
      if (row === null) merged.delete(id)
      else merged.set(id, row)
    }
    return [...merged.values()]
  }

  static usesIndex(store, predicate) {
    const key = (value) => (typeof value === 'number' && Number.isFinite(value)) || typeof value === 'string'
    return ['eq', 'lt', 'lte', 'gt', 'gte'].includes(predicate.operator) && key(predicate.value) && store.indexNames.contains(predicate.field)
  }

  static range(predicate) {
    const value = predicate.value
    if (predicate.operator === 'eq') return IDBKeyRange.only(value)
    if (predicate.operator === 'lt') return IDBKeyRange.upperBound(value, true)
    if (predicate.operator === 'lte') return IDBKeyRange.upperBound(value)
    if (predicate.operator === 'gt') return IDBKeyRange.lowerBound(value, true)
    return IDBKeyRange.lowerBound(value)
  }

  static matches(row, predicate) {
    const value = row[predicate.field]
    const wanted = predicate.value
    const comparable = (typeof value === 'number' && typeof wanted === 'number') || (typeof value === 'string' && typeof wanted === 'string')
    switch (predicate.operator) {
      case 'eq': return value === wanted || (wanted === null && value === undefined)
      case 'ne': return !(value === wanted || (wanted === null && value === undefined))
      case 'lt': return comparable && value < wanted
      case 'lte': return comparable && value <= wanted
      case 'gt': return comparable && value > wanted
      case 'gte': return comparable && value >= wanted
      case 'in': return wanted.includes(value)
      case 'isNull': return value === null || value === undefined
      case 'isNotNull': return value !== null && value !== undefined
      default: return false
    }
  }

  static compare(left, right) {
    const rank = (value) => value === null || value === undefined ? 0 : typeof value === 'boolean' ? 1 : typeof value === 'number' ? 2 : typeof value === 'string' ? 3 : 4
    if (rank(left) !== rank(right)) return rank(left) - rank(right)
    if (rank(left) === 4) return JSON.stringify(left) < JSON.stringify(right) ? -1 : JSON.stringify(left) > JSON.stringify(right) ? 1 : 0
    return left < right ? -1 : left > right ? 1 : 0
  }

  static page(rows, page, orderBy, idField) {
    const order = page.sort.length
      ? page.sort.map((entry) => { const [field, direction] = entry.split(':'); return { field, direction } })
      : orderBy
    const ordering = order.some((entry) => entry.field === idField) ? order : [...order, { field: idField, direction: 'asc' }]
    const sorted = rows.slice().sort((left, right) => {
      for (const { field, direction } of ordering) {
        const difference = TacEntityStorage.compare(left[field], right[field])
        if (difference) return direction === 'desc' ? -difference : difference
      }
      return 0
    })
    return sorted.slice(page.offset, page.offset + page.size)
  }

  static async query(command, pending) {
    const query = command.query
    const rows = (await TacEntityStorage.rows(command, query.where, pending))
      .filter((row) => query.where.every((predicate) => TacEntityStorage.matches(row, predicate)))
    if (query.kind === 'count') return rows.length
    if (query.kind === 'exists') return rows.length > 0
    return TacEntityStorage.page(rows, query.page, query.orderBy, command.idField)
  }
}

class TacEntityMirror {
  static tables = new Map()
  static keys = new Map()
  static oversized = new Set()
  static transactions = new Map()
  static counter = 0
  static loaded = false
  static loading = null
  static writing = Promise.resolve()

  static preload() {
    TacEntityMirror.loading ??= TacEntityMirror.load().catch((error) => { console.error(error) })
    return TacEntityMirror.loading
  }

  static async load() {
    try { if (typeof globalThis.indexedDB?.open !== 'function') return } catch { return }
    if (!(await TacEntityMirror.databaseExists())) { TacEntityMirror.loaded = true; return }
    const database = await TacEntityStorage.database()
    for (const table of database.objectStoreNames) {
      const store = database.transaction(table, 'readonly').objectStore(table)
      const rows = await TacEntityStorage.answer(store.getAll(null, TacEntityStorage.scanLimit + 1))
      TacEntityMirror.keys.set(table, store.keyPath)
      if (rows.length > TacEntityStorage.scanLimit) { TacEntityMirror.oversized.add(table); continue }
      TacEntityMirror.tables.set(table, new Map(rows.map((row) => [row[store.keyPath], row])))
    }
    TacEntityMirror.loaded = true
  }

  static async databaseExists() {
    if (typeof indexedDB.databases !== 'function') return true
    const databases = await indexedDB.databases()
    return databases.some((database) => database.name === TacEntityStorage.databaseName)
  }

  static execute(command) {
    if (command.operation === 'transaction') return TacEntityMirror.transaction(command)
    const rows = TacEntityMirror.rows(command)
    const pending = command.transaction == null ? null : TacEntityMirror.transactions.get(command.transaction)
    if (command.transaction != null && !pending) throw new TacStorageError('The IndexedDB transaction is not open.')
    if (command.operation === 'save') return TacEntityMirror.write(command, pending, command.value[command.idField], command.value) ?? command.value
    if (command.operation === 'delete') return TacEntityMirror.write(command, pending, command.id, null) ?? true
    const visible = TacEntityMirror.visible(rows, pending?.get(command.table))
    if (command.operation === 'find') return TacEntityStorage.page(visible, command.page, [], command.idField)
    if (command.operation !== 'query') throw new TacStorageError(`The IndexedDB adapter does not know the operation '${command.operation}'.`)
    const query = command.query
    const found = visible.filter((row) => query.where.every((predicate) => TacEntityStorage.matches(row, predicate)))
    if (query.kind === 'count') return found.length
    if (query.kind === 'exists') return found.length > 0
    return TacEntityStorage.page(found, query.page, query.orderBy, command.idField)
  }

  static rows(command) {
    const table = command.table
    if (!TacEntityMirror.loaded) throw new TacStorageError('Tachyon could not load the IndexedDB rows for this page, so this service cannot read its Store.')
    TacEntityStorage.checkKey(table, TacEntityMirror.keys.get(table) ?? command.idField, command.idField)
    const rows = TacEntityMirror.tables.get(table) ?? new Map()
    if (TacEntityMirror.oversized.has(table) || rows.size > TacEntityStorage.scanLimit) {
      throw new TacStorageError(`The IndexedDB store '${table}' has more than ${TacEntityStorage.scanLimit} rows. A WebAssembly or native Tac service reads its Store from memory, so keep this Table below that size or move the Store to a JavaScript or TypeScript service.`)
    }
    TacEntityStorage.declare(table, command.idField, [])
    TacEntityMirror.tables.set(table, rows)
    TacEntityMirror.keys.set(table, command.idField)
    return rows
  }

  static write(command, pending, id, row) {
    if (pending) { TacEntityStorage.staged(pending, command.table).set(id, row); return undefined }
    const writes = new Map([[command.table, new Map([[id, row]])]])
    TacEntityMirror.apply(writes)
    TacEntityMirror.persist(writes)
    return undefined
  }

  static visible(rows, staged) {
    if (!staged) return [...rows.values()]
    const merged = new Map(rows)
    for (const [id, row] of staged) {
      if (row === null) merged.delete(id)
      else merged.set(id, row)
    }
    return [...merged.values()]
  }

  static transaction(command) {
    if (command.action === 'begin') {
      if (TacEntityMirror.transactions.size >= TacEntityStorage.maxTransactions) throw new TacStorageError('Too many IndexedDB transactions are open at once.')
      const id = `indexeddb-memory-${++TacEntityMirror.counter}`
      TacEntityMirror.transactions.set(id, new Map())
      return { transaction: id }
    }
    const writes = TacEntityMirror.transactions.get(command.transaction)
    TacEntityMirror.transactions.delete(command.transaction)
    if (!writes) throw new TacStorageError('The IndexedDB transaction is not open.')
    if (command.action === 'commit' && writes.size) {
      TacEntityMirror.apply(writes)
      TacEntityMirror.persist(writes)
    }
    return null
  }

  static apply(writes) {
    for (const [table, rows] of writes) {
      const mirrored = TacEntityMirror.tables.get(table)
      if (!mirrored) continue
      for (const [id, row] of rows) {
        if (row === null) mirrored.delete(id)
        else mirrored.set(id, row)
      }
    }
  }

  static persist(writes) {
    TacEntityMirror.writing = TacEntityMirror.writing
      .then(() => TacEntityStorage.commit(writes))
      .catch((error) => { console.error(error) })
  }

  static settled() {
    return TacEntityMirror.writing
  }
}

class TacStorageHost {
  static answer(text) {
    try {
      return JSON.stringify({ value: TacStorageHost.run(JSON.parse(text)) ?? null })
    } catch (error) {
      return JSON.stringify({ error: error instanceof TacStorageError ? error.message : 'Tachyon storage failed.' })
    }
  }

  static run(request) {
    if (request.op === 'cache.get') return TacCacheStorage.read(request.cache, request.key)
    if (request.op === 'cache.set') return TacCacheStorage.write(request.cache, request.key, request.value, request)
    if (request.op === 'cache.delete') return TacCacheStorage.remove(request.cache, request.key)
    if (request.op === 'entity') return TacEntityMirror.execute(request.command)
    throw new TacStorageError(`Tachyon storage has no request named '${request.op}'.`)
  }
}

class TacStorage {
  static key = Symbol.for('tachyon.tac.storage')
  static preloadTimeoutMs = 2000
  static loaded = TacStorage.install()

  static install() {
    Object.defineProperty(globalThis, TacStorage.key, {
      configurable: true,
      value: Object.freeze({
        cache: (name) => TacCacheStorage.adapter(name),
        entities: () => TacEntityStorage.adapter,
        declare: (table, id, fields) => TacEntityStorage.declare(table, id, fields),
        settled: () => Promise.all([TacCacheStorage.settled(), TacEntityMirror.settled()]),
        host: (text) => TacStorageHost.answer(text),
        mirror: () => TacEntityMirror.preload(),
        failure: TacStorageError,
      }),
    })
    return TacCacheStorage.preload()
  }

  static ready() {
    return Promise.race([TacStorage.loaded, new Promise((resolve) => setTimeout(resolve, TacStorage.preloadTimeoutMs))])
  }
}

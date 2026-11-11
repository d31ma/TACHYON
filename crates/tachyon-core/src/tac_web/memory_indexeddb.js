class MemoryKeyRange {
  constructor(test) { this.test = test }
  includes(value) { return this.test(value) }
  static only(value) { return new MemoryKeyRange((key) => key === value) }
  static upperBound(value, open) { return new MemoryKeyRange((key) => (open ? key < value : key <= value)) }
  static lowerBound(value, open) { return new MemoryKeyRange((key) => (open ? key > value : key >= value)) }
}

class MemoryNames {
  constructor(names) { this.names = names }
  contains(name) { return this.names().includes(name) }
  [Symbol.iterator]() { return this.names()[Symbol.iterator]() }
}

class MemoryRequest {
  static later(transaction, work) {
    const request = new MemoryRequest()
    transaction.pending += 1
    setTimeout(() => {
      try { request.result = work(); request.onsuccess?.() } catch (error) { request.error = error; request.onerror?.() }
      transaction.done()
    }, 0)
    return request
  }
}

class MemoryIndex {
  constructor(store, keyPath) { this.store = store; this.keyPath = keyPath }
  getAll(range, count) {
    return MemoryRequest.later(this.store.transaction, () => this.store.rows().filter((row) => !range || range.includes(row[this.keyPath])).slice(0, count ?? Infinity))
  }
}

class MemoryStore {
  constructor(transaction, table) { this.transaction = transaction; this.table = table; this.keyPath = table.keyPath }
  get indexNames() { return new MemoryNames(() => [...this.table.indexes.keys()]) }
  rows() { return [...this.table.rows.values()].map((row) => structuredClone(row)) }
  index(name) { return new MemoryIndex(this, this.table.indexes.get(name)) }
  createIndex(name, keyPath) { this.table.indexes.set(name, keyPath) }
  deleteIndex(name) { this.table.indexes.delete(name) }
  getAll(range, count) { return MemoryRequest.later(this.transaction, () => this.rows().filter((row) => !range || range.includes(row[this.keyPath])).slice(0, count ?? Infinity)) }
  put(row) {
    return MemoryRequest.later(this.transaction, () => {
      if (globalThis.memoryIndexedDbRefusesWrites) throw Object.assign(new Error('The quota has been exceeded.'), { name: 'QuotaExceededError' })
      this.table.rows.set(row[this.keyPath], structuredClone(row))
    })
  }
  delete(id) { return MemoryRequest.later(this.transaction, () => { this.table.rows.delete(id) }) }
}

class MemoryTransaction {
  constructor(database) { this.database = database; this.pending = 0; this.error = null; this.finished = false; setTimeout(() => this.done(), 0) }
  objectStore(name) { return new MemoryStore(this, this.database.tables.get(name)) }
  done() {
    if (this.pending > 0) this.pending -= 1
    if (this.pending > 0 || this.finished) return
    this.finished = true
    setTimeout(() => (this.failure ? this.onerror?.() : this.oncomplete?.()), 0)
  }
}

class MemoryDatabase {
  constructor(name, version) { this.name = name; this.version = version; this.tables = new Map() }
  get objectStoreNames() { return new MemoryNames(() => [...this.tables.keys()]) }
  createObjectStore(name, options) { this.tables.set(name, { keyPath: options.keyPath, rows: new Map(), indexes: new Map() }); return new MemoryStore(this.upgrading, this.tables.get(name)) }
  transaction() {
    const transaction = new MemoryTransaction(this)
    const objectStore = transaction.objectStore.bind(transaction)
    transaction.objectStore = (name) => {
      const store = objectStore(name)
      const put = store.put.bind(store)
      store.put = (row) => { const request = put(row); request.onerror = () => { transaction.failure = true; transaction.error = request.error }; return request }
      return store
    }
    return transaction
  }
  close() {}
}

class MemoryIndexedDb {
  static databases = new Map()

  static install() {
    globalThis.IDBKeyRange = MemoryKeyRange
    globalThis.indexedDB = {
      databases: async () => [...MemoryIndexedDb.databases.keys()].map((name) => ({ name })),
      open: (name, version) => MemoryIndexedDb.open(name, version),
    }
  }

  static seed(name, table, keyPath, partition, rows) {
    const database = MemoryIndexedDb.databases.get(name) ?? new MemoryDatabase(name, 1)
    const indexes = new Map(partition ? [['tachyon:partition', partition]] : [])
    database.tables.set(table, { keyPath, rows: new Map(rows.map((row) => [row[keyPath], row])), indexes })
    MemoryIndexedDb.databases.set(name, database)
  }

  static rows(name, table) {
    return [...(MemoryIndexedDb.databases.get(name)?.tables.get(table)?.rows.values() ?? [])]
  }

  static open(name, version) {
    const request = new MemoryRequest()
    setTimeout(() => {
      let database = MemoryIndexedDb.databases.get(name)
      const upgrade = !database || (version !== undefined && version > database.version)
      database ??= new MemoryDatabase(name, 0)
      MemoryIndexedDb.databases.set(name, database)
      request.result = database
      if (upgrade) {
        database.version = version ?? Math.max(1, database.version)
        request.transaction = new MemoryTransaction(database)
        database.upgrading = request.transaction
        request.onupgradeneeded?.()
      }
      request.onsuccess?.()
    }, 0)
    return request
  }
}

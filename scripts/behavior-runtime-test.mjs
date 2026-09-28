import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { once } from 'node:events'
import vm from 'node:vm'

class BehaviorRuntimeTest {
  static runtimePath = new URL('../crates/tachyon-core/src/behavior/runtime.js', import.meta.url)
  static runtimeSource = null
  static createBehaviors = null
  static moduleId = 'a'.repeat(64)
  static passed = 0
}

await import('./behavior-extensions-test.mjs')
BehaviorRuntimeTest.runtimeSource = await readFile(BehaviorRuntimeTest.runtimePath, 'utf8')
BehaviorRuntimeTest.createBehaviors = vm.runInThisContext(`${BehaviorRuntimeTest.runtimeSource}\n__tachyon_create_behaviors__`)
async function test(name, run) {
  await run()
  BehaviorRuntimeTest.passed += 1
  process.stdout.write(`ok ${BehaviorRuntimeTest.passed} - ${name}\n`)
}

await test('monotonic deadlines prevent writes after a body blocks timer delivery', async () => {
  let writes = 0
  class Configuration { static configure() { return { authorize: () => true, caches: { data: { set() { writes++ } } } } } }
  class Service {
    constructor(behaviors) { this.behaviors = behaviors }
    async read(id) { return id }
    async update(id) { const until = performance.now() + 40; while (performance.now() < until) {} return id }
  }
  const factory = BehaviorRuntimeTest.createBehaviors([
    { type: Configuration, name: 'Configuration', configuration: true, methods: [] },
    { type: Service, name: 'Service', cache: { name: 'data', ttlMs: 1000, maxBytes: 1024, maxEntries: 8 }, methods: [
      { name: 'read', cacheable: { keyArgs: [0] } },
      { name: 'update', cachePut: { method: 'read', arguments: [0], target: { method: 'read', cacheable: { keyArgs: [0] } } } },
    ] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await factory.create(Configuration, { partition: 'tenant', signal: new AbortController().signal, timeoutMs: 10 })
  try { await assert.rejects(new Service(context).update(7), { code: 'timeout' }); assert.equal(writes, 0) }
  finally { context.dispose() }
})

for (const environment of ['tac', 'yon']) {
  await test(`${environment}: cache mutations share normalized read keys and commit only after success`, async () => {
    const entries = new Map()
    const events = []
    let allowed = true
    class Configuration {
      static configure() { return {
        caches: { items: {
          get: (key) => entries.get(key) ?? { hit: false },
          set: (key, value, execution) => { events.push('set'); entries.set(key, { hit: true, value, expiresAt: execution.expiresAt }) },
          delete: (key) => { events.push('delete'); entries.delete(key) },
        } },
        authorize: (call) => { events.push(call.operation); return allowed },
      } }
    }
    class Items {
      constructor(behaviors) { this.behaviors = behaviors }
      async find(page) { events.push('read'); return { value: 'original', page } }
      async put(value, page) { events.push('body'); if (value === 'fail') throw Error('failed'); return { value, page } }
      async remove(page) { events.push('body'); return true }
    }
    const read = { name: 'find', cacheable: { keyArgs: [0] }, pageable: { argument: 0, defaultSize: 20, maxSize: 100, sort: ['id'] } }
    const target = { method: 'find', cacheable: read.cacheable, pageable: read.pageable }
    const Behaviors = BehaviorRuntimeTest.createBehaviors([
      { type: Configuration, name: 'Configuration', configuration: true, methods: [] },
      { type: Items, name: 'Items', cache: { name: 'items', ttlMs: 30000, maxBytes: 4096, maxEntries: 8 }, methods: [
        read,
        { name: 'put', cachePut: { method: 'find', arguments: [1], target } },
        { name: 'remove', cacheEvict: { method: 'find', arguments: [0], target } },
      ] },
    ], environment, BehaviorRuntimeTest.moduleId)
    const context = await Behaviors.create(Configuration, { partition: 'tenant', signal: new AbortController().signal })
    const items = new Items(context)
    try {
      await items.find({})
      events.length = 0
      await items.put('changed', {})
      assert.deepEqual(events, ['cache:put', 'body', 'set'])
      assert.equal((await items.find({ page: 0, size: 20 })).value, 'changed')
      await assert.rejects(items.put('fail', {}), /failed/)
      assert.equal((await items.find({})).value, 'changed')
      events.length = 0
      allowed = false
      await assert.rejects(items.remove({}), /authorized/)
      assert.deepEqual(events, ['cache:evict'])
      allowed = true
      events.length = 0
      await items.remove({})
      assert.deepEqual(events, ['cache:evict', 'body', 'delete'])
      assert.equal(entries.size, 0)
      events.length = 0
      await assert.rejects(items.put('invalid', { size: 101 }), /pagination/)
      assert.deepEqual(events, [])
    } finally { context.dispose() }
  })
}

function fixture(environment = 'yon', overrides = {}, moduleId = BehaviorRuntimeTest.moduleId, factory = BehaviorRuntimeTest.createBehaviors) {
  const entries = new Map()
  const counts = { configured: 0, authorized: 0, called: 0, gets: 0, sets: 0 }
  const cache = {
    async get(key) { counts.gets += 1; return entries.get(key) || { hit: false } },
    async set(key, value, execution) {
      counts.sets += 1
      assert.equal(execution.maxEntries, 8)
      entries.set(key, { hit: true, value, expiresAt: execution.expiresAt })
    },
    async delete(key) { entries.delete(key) },
  }
  class Configuration {
    static async configure(execution) {
      counts.configured += 1
      assert.equal(execution.environment, environment)
      return {
        caches: { products: cache },
        authorize: async () => { counts.authorized += 1; return true },
        ...overrides,
      }
    }
  }
  class Products {
    constructor(behaviors) { this.behaviors = behaviors }
    async list(page) { counts.called += 1; return { page, called: counts.called } }
    async fail() { throw new Error('private failure content') }
    async stream() { return (function* () { yield 1 })() }
  }
  const Behaviors = factory([
    { type: Configuration, name: 'Configuration', configuration: true, methods: [] },
    {
      type: Products, name: 'Products', configuration: false,
      cache: { name: 'products', ttlMs: 30000, maxBytes: 4096, maxEntries: 8 },
      methods: [
        { name: 'list', pageable: { argument: 0, defaultSize: 20, maxSize: 100, sort: ['id', 'name'] }, cacheable: { keyArgs: [0] } },
        { name: 'fail', cacheable: { keyArgs: [] } },
        { name: 'stream', cacheable: { keyArgs: [] } },
      ],
    },
  ], environment, moduleId)
  return { Behaviors, Configuration, Products, entries, counts }
}

await test('compiler module identity isolates identical service names sharing an external cache', async () => {
  const shared = new Map()
  const adapter = {
    get: (key) => shared.get(key) || { hit: false },
    set: (key, value, policy) => shared.set(key, { hit: true, value, expiresAt: policy.expiresAt }),
    delete: (key) => shared.delete(key),
  }
  const first = fixture('yon', { caches: { products: adapter } }, '1'.repeat(64))
  const second = fixture('yon', { caches: { products: adapter } }, '2'.repeat(64))
  const options = { partition: 'same-user', signal: new AbortController().signal }
  const left = await first.Behaviors.create(first.Configuration, options)
  const right = await second.Behaviors.create(second.Configuration, options)
  const one = new first.Products(left)
  const two = new second.Products(right)
  await one.list({ page: 1 })
  await two.list({ page: 1 })
  assert.equal(first.counts.called, 1)
  assert.equal(second.counts.called, 1)
  assert.equal(shared.size, 2)
  await left.evict(one, 'list', [{ page: 1 }])
  await two.list({ page: 1 })
  assert.equal(second.counts.called, 1)
  assert.equal(shared.size, 1)
  left.dispose()
  right.dispose()
})

await test('module identity is required, bounded and compiler-shaped', async () => {
  for (const moduleId of [undefined, '', 'a', 'A'.repeat(64), 'f'.repeat(65), 'g'.repeat(64), {}]) {
    assert.throws(() => BehaviorRuntimeTest.createBehaviors([], 'yon', moduleId), /module identity/i)
  }
})

for (const environment of ['tac', 'yon']) {
  await test(`${environment}: registration performs no configuration or IO`, async () => {
    const f = fixture(environment)
    assert.equal(f.counts.configured, 0)
    await assert.rejects(new f.Products().list({}), /context/i)
    assert.equal(f.counts.configured, 0)
  })
  await test(`${environment}: authorization on hits, canonical pagination, isolation, eviction`, async () => {
    const f = fixture(environment)
    const signal = new AbortController().signal
    const left = await f.Behaviors.create(f.Configuration, { partition: 'tenant-a', signal })
    const right = await f.Behaviors.create(f.Configuration, { partition: 'tenant-b', signal })
    const one = new f.Products(left)
    const two = new f.Products(right)
    const first = await one.list({ page: 2, size: 10, sort: ['name:desc'] })
    assert.deepEqual(first.page, { page: 2, size: 10, offset: 20, sort: ['name:desc'] })
    assert.deepEqual(await one.list({ sort: ['name:desc'], size: 10, page: 2 }), first)
    assert.equal(f.counts.called, 1)
    assert.equal(f.counts.authorized, 2)
    await two.list({ page: 2, size: 10, sort: ['name:desc'] })
    assert.equal(f.counts.called, 2)
    await left.evict(one, 'list', [{ page: 2, size: 10, sort: ['name:desc'] }])
    await one.list({ page: 2, size: 10, sort: ['name:desc'] })
    assert.equal(f.counts.called, 3)
    left.dispose()
    right.dispose()
  })
}

await test('pagination rejects invalid, excessive, unknown, and unsafe inputs', async () => {
  const f = fixture()
  const context = await f.Behaviors.create(f.Configuration, { partition: 'public', signal: new AbortController().signal })
  const products = new f.Products(context)
  for (const value of [{ page: -1 }, { page: '1' }, { size: 0 }, { size: 101 }, { page: 1000000 }, { sort: ['secret:asc'] }, { sort: ['id:evil'] }, { unknown: true }, null]) {
    await assert.rejects(products.list(value), /pagination/i)
  }
  assert.equal(f.counts.gets, 0)
  assert.deepEqual((await products.list({})).page, { page: 0, size: 20, offset: 0, sort: [] })
  context.dispose()
})

await test('Pageable-only preserves synchronous results and bounds asynchronous results', async () => {
  class Config { static configure() { return {} } }
  class Pages {
    constructor(behaviors) { this.behaviors = behaviors }
    sync(page) { return page }
    async asynchronous(page) { return page }
    async never(_page) { return new Promise(() => {}) }
  }
  const policy = { argument: 0, defaultSize: 20, maxSize: 100, sort: ['id'] }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Pages, name: 'Pages', configuration: false,
      methods: ['sync', 'asynchronous', 'never'].map((name) => ({ name, pageable: policy })) },
  ], 'tac', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'a', timeoutMs: 10, signal: new AbortController().signal })
  const pages = new Pages(context)
  assert.deepEqual(pages.sync({ size: 5 }), { page: 0, size: 5, offset: 0, sort: [] })
  assert.deepEqual(await pages.asynchronous({ page: 2, sort: ['id'] }), { page: 2, size: 20, offset: 40, sort: ['id:asc'] })
  assert.throws(() => pages.sync({ page: -1 }), /pagination/i)
  await assert.rejects(pages.never({}), /timed out/i)
  context.dispose()
  assert.throws(() => pages.sync({}), /disposed/i)
})

await test('cache requires authorization and rejects non-true authorization', async () => {
  for (const authorize of [undefined, () => false, () => 'yes', () => Promise.reject(new Error('secret'))]) {
    const f = fixture('yon', { authorize })
    const context = await f.Behaviors.create(f.Configuration, { partition: 'private', signal: new AbortController().signal })
    await assert.rejects(new f.Products(context).list({}), (error) => !error.message.includes('secret'))
    assert.equal(f.counts.gets, 0)
    assert.equal(f.counts.called, 0)
    context.dispose()
  }
})

await test('authorization receives immutable selected arguments and can revoke an existing hit', async () => {
  let allowed = true
  const observed = []
  const f = fixture('yon', { authorize: (intent, execution) => {
    assert.equal(execution.partition, 'user-a')
    assert.equal(Object.isFrozen(intent.keyArguments), true)
    assert.equal(Object.isFrozen(intent.keyArguments[0]), true)
    observed.push(intent.keyArguments[0].page)
    return allowed && intent.keyArguments[0].page === 1
  } })
  const context = await f.Behaviors.create(f.Configuration, { partition: 'user-a', signal: new AbortController().signal })
  const products = new f.Products(context)
  await products.list({ page: 1 })
  await products.list({ page: 1 })
  assert.equal(f.counts.called, 1)
  await assert.rejects(products.list({ page: 2 }), /not authorized/i)
  allowed = false
  await assert.rejects(products.list({ page: 1 }), /not authorized/i)
  assert.equal(f.counts.gets, 2)
  assert.deepEqual(observed, [1, 1, 2, 1])
  context.dispose()
})

await test('cache expires, clones results, excludes failures/streams and bounds poisoned hits', async () => {
  const f = fixture()
  const context = await f.Behaviors.create(f.Configuration, { partition: 'public', signal: new AbortController().signal })
  const products = new f.Products(context)
  const first = await products.list({})
  first.page.size = 99
  assert.equal((await products.list({})).page.size, 20)
  const entry = [...f.entries.values()][0]
  entry.expiresAt = Date.now() - 1
  await products.list({})
  assert.equal(f.counts.called, 2)
  await assert.rejects(products.fail())
  await assert.rejects(products.stream(), /JSON/i)
  assert.equal(f.counts.sets, 2)
  const current = [...f.entries.values()][0]
  current.value = 'a'.repeat(5000)
  await assert.rejects(products.list({}), /bound/i)
  context.dispose()
})

await test('context rejects missing partition, unknown configuration and aborts creation', async () => {
  const f = fixture()
  const signal = new AbortController().signal
  await assert.rejects(f.Behaviors.create(f.Configuration, { partition: '', signal }), /partition/i)
  await assert.rejects(f.Behaviors.create(class Other {}, { partition: 'a', signal }), /Setup/)
  const aborted = new AbortController()
  aborted.abort()
  await assert.rejects(f.Behaviors.create(f.Configuration, { partition: 'a', signal: aborted.signal }), /abort/i)
})

await test('timeouts and owner disposal abort adapter work and sanitize errors', async () => {
  let seenSignal
  const f = fixture('tac', { caches: { products: { get: async (_key, execution) => { seenSignal = execution.signal; return new Promise(() => {}) }, set() {}, delete() {} } } })
  const context = await f.Behaviors.create(f.Configuration, { partition: 'a', timeoutMs: 20, signal: new AbortController().signal })
  await assert.rejects(new f.Products(context).list({}), /timed out/i)
  assert.equal(seenSignal.aborted, true)
  const pending = new f.Products(context).list({})
  context.dispose()
  await assert.rejects(pending, /abort|disposed/i)
  await assert.rejects(new f.Products(context).list({}), /disposed/i)
  context.dispose()
})

await test('concurrent operations are bounded and abort without shared listener growth', async () => {
  const signals = []
  const f = fixture('yon', { caches: { products: {
    get: async (_key, execution) => { signals.push(execution.signal); return new Promise(() => {}) },
    set() {}, delete() {},
  } } })
  const abort = new AbortController()
  const context = await f.Behaviors.create(f.Configuration, { partition: 'a', signal: abort.signal })
  const products = new f.Products(context)
  const pending = Array.from({ length: 64 }, () => products.list({}))
  const settled = Promise.allSettled(pending)
  await assert.rejects(products.list({}), /concurrency bound/i)
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(signals.length, 64)
  abort.abort()
  assert.equal((await settled).every((result) => result.status === 'rejected'), true)
  assert.equal(signals.every((signal) => signal.aborted), true)
})

await test('method-result and key boundaries reject cycles, getters, nonfinite values and oversize data', async () => {
  let getterCalled = false
  class Config {
    static configure() {
      return { authorize: () => true, caches: { data: { get: () => ({ hit: false }), set() {}, delete() {} } } }
    }
  }
  class Service {
    constructor(behaviors) { this.behaviors = behaviors }
    async identity(value) { return value }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Service, name: 'Service', configuration: false, cache: { name: 'data', ttlMs: 1, maxBytes: 4096, maxEntries: 1 },
      methods: [{ name: 'identity', cacheable: { keyArgs: [0] } }] },
  ], 'tac', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'a', signal: new AbortController().signal })
  const service = new Service(context)
  const cycle = {}; cycle.self = cycle
  const accessor = { get value() { getterCalled = true; return 'secret' } }
  for (const value of [cycle, accessor, Infinity, NaN, undefined, BigInt(3), 'z'.repeat(5000), JSON.parse('{"__proto__":{"polluted":true}}')]) {
    await assert.rejects(service.identity(value), /JSON|bound|missing|unsafe/i)
  }
  assert.equal(getterCalled, false)
  assert.equal(Object.prototype.polluted, undefined)
  context.dispose()
})

await test('configuration failures are sanitized and caller cancellation is honored', async () => {
  for (const configure of [() => { throw new Error('secret-value') }, () => new Promise(() => {})]) {
    class Config { static configure = configure }
    const Behaviors = BehaviorRuntimeTest.createBehaviors([{ type: Config, name: 'Config', configuration: true, methods: [] }], 'tac', BehaviorRuntimeTest.moduleId)
    await assert.rejects(Behaviors.create(Config, { partition: 'a', timeoutMs: 10, signal: new AbortController().signal }),
      (error) => ['adapter', 'timeout'].includes(error.code) && !error.message.includes('secret-value'))
  }
  class Config { static configure() { return new Promise(() => {}) } }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([{ type: Config, name: 'Config', configuration: true, methods: [] }], 'yon', BehaviorRuntimeTest.moduleId)
  const controller = new AbortController()
  const creating = Behaviors.create(Config, { partition: 'a', signal: controller.signal })
  controller.abort()
  await assert.rejects(creating, /disposed|abort/i)
})

await test('configuration and missing adapters fail closed without invoking application methods', async () => {
  const invalidConfiguration = fixture('tac', { unexpected: 'backend' })
  await assert.rejects(invalidConfiguration.Behaviors.create(invalidConfiguration.Configuration,
    { partition: 'a', signal: new AbortController().signal }), /configuration/i)
  const noCache = fixture('yon', { caches: {} })
  const context = await noCache.Behaviors.create(noCache.Configuration, { partition: 'a', signal: new AbortController().signal })
  await assert.rejects(new noCache.Products(context).list({}), /adapter is not configured/i)
  assert.equal(noCache.counts.called, 0)
  context.dispose()
})

await test('one deadline includes authorization, lookup and result storage', async () => {
  for (const costs of [[25, 0, 0], [15, 15, 0], [10, 10, 10]]) {
    let elapsed = 0
    const executions = []
    const factory = vm.compileFunction(`${BehaviorRuntimeTest.runtimeSource}\nreturn __tachyon_create_behaviors__`,
      ['performance', 'setTimeout', 'clearTimeout'])(
      { now: () => elapsed }, () => 0, () => {})
    const advance = (execution, cost) => { executions.push(execution); elapsed += cost }
    const f = fixture('yon', {
      authorize: async (_call, execution) => { advance(execution, costs[0]); return true },
      caches: { products: {
        async get(_key, execution) { advance(execution, costs[1]); return { hit: false } },
        async set(_key, _value, execution) { advance(execution, costs[2]) },
      } },
    }, BehaviorRuntimeTest.moduleId, factory)
    const context = await f.Behaviors.create(f.Configuration, { partition: 'a', timeoutMs: 25, signal: new AbortController().signal })
    try {
      await assert.rejects(new f.Products(context).list({}), { code: 'timeout' })
      const phaseCount = costs[0] === 25 ? 1 : costs[1] === 15 ? 2 : 3
      assert.equal(executions.length, phaseCount)
      assert.equal(f.counts.called, phaseCount === 3 ? 1 : 0)
      for (const execution of executions) {
        assert.equal(execution.signal, executions[0].signal)
        assert.equal(execution.deadline, executions[0].deadline)
        assert.equal(execution.signal.aborted, true)
      }
    } finally { context.dispose() }
  }
})

await test('required argument factory constructs only exact declared fields', async () => {
  class Service { constructor(behaviors, repository) { this.behaviors = behaviors; this.repository = repository } }
  class Plain {}
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Service, name: 'Service', configuration: false, requiredArgs: ['behaviors', 'repository'], methods: [] },
  ], 'tac', BehaviorRuntimeTest.moduleId)
  const context = { marker: true }
  const repository = { find: () => [] }
  const instance = Behaviors.construct(Service, { repository, behaviors: context })
  assert.equal(instance.behaviors, context)
  assert.equal(instance.repository, repository)
  for (const value of [{ behaviors: context }, { behaviors: context, repository: undefined }, { behaviors: context, repository, extra: true }]) {
    assert.throws(() => Behaviors.construct(Service, value), /argument/i)
  }
  assert.throws(() => Behaviors.construct(Plain, {}), /NeedsAll/i)
  assert.throws(() => Behaviors.construct(Service, Object.create({ behaviors: context, repository })), /argument/i)
})

await test('metadata rejects unsafe outbound paths, mutable policy, duplicate and endpoint caching', async () => {
  class Client { async request() { return {} } }
  for (const path of ['//host/path', '/../private', '/a/../private', '/%2e%2e/private', '/%252e%252e/private', '/path?query=x', '/\\host', '/%2fhost']) {
    assert.throws(() => BehaviorRuntimeTest.createBehaviors([{ type: Client, name: 'Client', configuration: false,
      methods: [{ name: 'request', httpExchange: { client: 'transport', method: 'GET', path } }],
    }], 'tac', BehaviorRuntimeTest.moduleId), /HTTP exchange/i)
  }
  class Endpoint { GET() { return {} } }
  assert.throws(() => BehaviorRuntimeTest.createBehaviors([{ type: Endpoint, name: 'Endpoint', configuration: false,
    cache: { name: 'data', ttlMs: 1, maxBytes: 256, maxEntries: 1 }, methods: [{ name: 'GET', cacheable: { keyArgs: [] } }],
  }], 'yon', BehaviorRuntimeTest.moduleId), /verb endpoints/i)
  assert.throws(() => BehaviorRuntimeTest.createBehaviors([{ type: Client, name: 'Client', configuration: false,
    methods: [{ name: 'request' }, { name: 'request' }],
  }], 'tac', BehaviorRuntimeTest.moduleId), /Duplicate/i)
})

await test('repository maps explicit CRUD operations without DDL', async () => {
  const rows = new Map()
  const operations = []
  class Product {}
  class Config {
    static configure() {
      return { entities: { database: { async execute(command, execution) {
        assert.equal(execution.partition, 'tenant')
        assert.equal(command.entity, 'Product')
        assert.equal(command.table, 'products')
        assert.equal(command.idField, 'id')
        operations.push(command.operation)
        if (command.operation === 'save') { rows.set(command.value.id, command.value); return command.value }
        if (command.operation === 'delete') return rows.delete(command.id)
        return [...rows.values()].slice(command.page.offset, command.page.offset + command.page.size)
      } } } }
    }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Product, name: 'Product', configuration: false, entity: { adapter: 'database', id: 'id', table: 'products' }, methods: [] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  const repository = context.repository(Product)
  assert.deepEqual(await repository.save({ id: 42, title: 'Chair' }), { id: 42, title: 'Chair' })
  assert.deepEqual(await repository.find({ page: 0, size: 10 }), [{ id: 42, title: 'Chair' }])
  assert.equal(await repository.delete(42), true)
  assert.deepEqual(operations, ['save', 'find', 'delete'])
  await assert.rejects(repository.save({ title: 'Missing id' }), /identifier/i)
  await assert.rejects(repository.delete({ $where: 'bad' }), /identifier/i)
  assert.throws(() => context.repository(class Other {}), /entity/i)
  context.dispose()
})

await test('WebCall executes through a real bounded local HTTP adapter', async () => {
  let requests = 0
  const server = createServer((request, response) => {
    requests += 1
    response.setHeader('Content-Type', 'application/json')
    if (request.url === '/slow') return
    response.end(JSON.stringify({ method: request.method, url: request.url, header: request.headers['x-test'] }))
  })
  server.listen(0, '127.0.0.1')
  await once(server, 'listening')
  try {
    const origin = `http://127.0.0.1:${server.address().port}`
    class Config {
      static configure() { return { http: { catalog: { async exchange(command) {
        assert.equal(command.redirect, 'error')
        assert.equal(command.credentials, 'omit')
        assert.equal(command.maxResponseBytes, 1048576)
        const url = new URL(command.path, origin)
        for (const [key, value] of Object.entries(command.query)) url.searchParams.set(key, String(value))
        const response = await fetch(url, command)
        return response.json()
      } } } } }
    }
    class Client {
      constructor(behaviors) { this.behaviors = behaviors }
      async products(query) { return { query, headers: { 'x-test': 'present' } } }
      async override() { return { path: 'https://untrusted.invalid/' } }
      async slow() { return {} }
    }
    const metadata = { client: 'catalog', method: 'GET', path: '/products' }
    const Behaviors = BehaviorRuntimeTest.createBehaviors([
      { type: Config, name: 'Config', configuration: true, methods: [] },
      { type: Client, name: 'Client', configuration: false, methods: [
        { name: 'products', httpExchange: metadata }, { name: 'override', httpExchange: metadata },
        { name: 'slow', httpExchange: { ...metadata, path: '/slow' } },
      ] },
    ], 'tac', BehaviorRuntimeTest.moduleId)
    assert.equal(requests, 0)
    const context = await Behaviors.create(Config, { partition: 'public', signal: new AbortController().signal })
    const client = new Client(context)
    assert.deepEqual(await client.products({ search: 'a b' }), { method: 'GET', url: '/products?search=a+b', header: 'present' })
    await assert.rejects(client.override(), /descriptor/i)
    assert.equal(requests, 1)
    context.dispose()
    const deadlineContext = await Behaviors.create(Config, { partition: 'public', timeoutMs: 30, signal: new AbortController().signal })
    await assert.rejects(new Client(deadlineContext).slow(), /timed out/i)
    deadlineContext.dispose()
  } finally {
    server.closeAllConnections()
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()))
  }
})

await test('Retry re-runs a failing call and gives up on its declared bound', async () => {
  let attempts = 0
  class Config { static configure() { return {} } }
  class Orders {
    constructor(behaviors) { this.behaviors = behaviors }
    async flaky(id) { attempts += 1; if (attempts < 3) throw Error('transient'); return id }
    async broken() { attempts += 1; throw Error('permanent') }
  }
  const retryable = { maxRetries: 3, delayMs: 0, multiplier: 1, maxDelayMs: 1000, jitterMs: 0 }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Orders, name: 'Orders', methods: [{ name: 'flaky', retryable }, { name: 'broken', retryable }] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    assert.equal(await new Orders(context).flaky(7), 7)
    assert.equal(attempts, 3)
    attempts = 0
    await assert.rejects(new Orders(context).broken(), /permanent/)
    assert.equal(attempts, 4, 'one attempt plus three retries')
  } finally { context.dispose() }
})

await test('Retry never retries an authoring, authorization or deadline refusal', async () => {
  let attempts = 0
  class Config { static configure() { return { authorize: () => false, caches: { items: { get: () => ({ hit: false }) } } } } }
  class Items {
    constructor(behaviors) { this.behaviors = behaviors }
    async find(id) { attempts += 1; return id }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Items, name: 'Items', cache: { name: 'items', ttlMs: 1000, maxBytes: 1024, maxEntries: 8 },
      methods: [{ name: 'find', cacheable: { keyArgs: [0] }, retryable: { maxRetries: 4, delayMs: 0, multiplier: 1, maxDelayMs: 10, jitterMs: 0 } }] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    await assert.rejects(new Items(context).find(1), { code: 'unauthorized' })
    assert.equal(attempts, 0, 'an unauthorized read is a decision, not a fault')
  } finally { context.dispose() }
})

await test('MaxAtOnce blocks or rejects beyond its declared slots', async () => {
  let peak = 0
  let active = 0
  const release = []
  class Config { static configure() { return {} } }
  class Gate {
    constructor(behaviors) { this.behaviors = behaviors }
    async hold() {
      active += 1
      peak = Math.max(peak, active)
      await new Promise((resolve) => release.push(resolve))
      active -= 1
      return true
    }
    async refuse() { return new Promise((resolve) => release.push(() => resolve(true))) }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Gate, name: 'Gate', methods: [
      { name: 'hold', concurrencyLimit: { limit: 2, policy: 'block' } },
      { name: 'refuse', concurrencyLimit: { limit: 1, policy: 'reject' } },
    ] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    const gate = new Gate(context)
    const calls = [gate.hold(), gate.hold(), gate.hold(), gate.hold()]
    await new Promise((resolve) => setTimeout(resolve, 10))
    assert.equal(peak, 2, 'never more than the declared slots run at once')
    while (release.length) release.shift()()
    await new Promise((resolve) => setTimeout(resolve, 10))
    while (release.length) release.shift()()
    assert.deepEqual(await Promise.all(calls), [true, true, true, true])
    assert.equal(peak, 2)

    const first = gate.refuse()
    await assert.rejects(gate.refuse(), { code: 'capacity' })
    release.shift()()
    assert.equal(await first, true)
    const again = gate.refuse()
    await new Promise((resolve) => setTimeout(resolve, 5))
    assert.equal(release.length, 1, 'the finished call returned its slot')
    release.shift()()
    assert.equal(await again, true)
  } finally { context.dispose() }
})

await test('a blocked MaxAtOnce waiter is bounded by the operation deadline', async () => {
  const release = []
  class Config { static configure() { return {} } }
  class Gate {
    constructor(behaviors) { this.behaviors = behaviors }
    async hold() { return new Promise((resolve) => release.push(() => resolve(true))) }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Gate, name: 'Gate', methods: [{ name: 'hold', concurrencyLimit: { limit: 1, policy: 'block' } }] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', timeoutMs: 40, signal: new AbortController().signal })
  try {
    const gate = new Gate(context)
    const held = gate.hold().catch((error) => error.code)
    await assert.rejects(gate.hold(), { code: 'timeout' }, 'a waiter cannot outlive its operation deadline')
    assert.equal(await held, 'timeout')
    release.length = 0
    const after = gate.hold()
    await new Promise((resolve) => setTimeout(resolve, 5))
    assert.equal(release.length, 1, 'the next call acquired the slot immediately')
    release.shift()()
    assert.equal(await after, true)
  } finally { context.dispose() }
})

await test('AllOrNothing commits on success, rolls back on failure, and marks the work in between', async () => {
  const log = []
  let sequence = 0
  class Config {
    static configure() {
      return { entities: { data: { execute: (command) => {
        log.push(command.operation === 'transaction' ? `${command.action}${command.transaction ? ':' + command.transaction : ''}` : `${command.operation}@${command.transaction ?? 'none'}`)
        if (command.operation === 'transaction' && command.action === 'begin') return { transaction: `t${++sequence}` }
        return command.operation === 'find' ? [] : { ok: true }
      } } } }
    }
  }
  class Orders {
    constructor(behaviors) { this.behaviors = behaviors }
    async place(fail) {
      await this.behaviors.repository(Order).save({ id: 'one' })
      if (fail) throw Error('domain rejected the order')
      return 'placed'
    }
    async audit() { return 'audited' }
  }
  class Order { }
  const transactional = { adapter: 'data' }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Order, name: 'Order', entity: { adapter: 'data', id: 'id', table: 'orders' }, methods: [] },
    { type: Orders, name: 'Orders', methods: [
      { name: 'place', transactional },
      { name: 'audit', transactional, retryable: { maxRetries: 1, delayMs: 0, multiplier: 1, maxDelayMs: 10, jitterMs: 0 } },
    ] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    const orders = new Orders(context)
    assert.equal(await orders.place(false), 'placed')
    assert.deepEqual(log, ['begin', 'save@t1', 'commit:t1'])
    log.length = 0
    await assert.rejects(orders.place(true), /domain rejected/)
    assert.deepEqual(log, ['begin', 'save@t2', 'rollback:t2'], 'a failed body rolls its transaction back')
    log.length = 0
    assert.equal(await orders.audit(), 'audited')
    assert.deepEqual(log, ['begin', 'commit:t3'])
    log.length = 0
    await context.repository(Order).save({ id: 'two' })
    assert.deepEqual(log, ['save@none'])
  } finally { context.dispose() }
})

await test('a transactional adapter that answers no identifier is refused before the body runs', async () => {
  let bodies = 0
  class Config {
    static configure() { return { entities: { data: { execute: () => ({ transaction: '' }) } } } }
  }
  class Orders {
    constructor(behaviors) { this.behaviors = behaviors }
    async place() { bodies += 1; return 'placed' }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Orders, name: 'Orders', methods: [{ name: 'place', transactional: { adapter: 'data' } }] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    await assert.rejects(new Orders(context).place(), { code: 'invalid' })
    assert.equal(bodies, 0)
  } finally { context.dispose() }
})

await test('RestClient calls the same bounded adapter WebCall uses', async () => {
  const seen = []
  const server = createServer((request, response) => {
    seen.push(`${request.method} ${request.url}`)
    let body = ''
    request.on('data', (chunk) => { body += chunk })
    request.on('end', () => {
      response.writeHead(200, { 'content-type': 'application/json' })
      response.end(JSON.stringify({ method: request.method, url: request.url, sent: body, header: request.headers['x-test'] ?? null }))
    })
  })
  server.listen(0, '127.0.0.1')
  await once(server, 'listening')
  const { port } = server.address()
  try {
    class Config {
      static configure() {
        return { http: { catalog: { exchange: async (command) => {
          assert.equal(command.redirect, 'error')
          assert.equal(command.credentials, 'omit')
          assert.equal(command.maxResponseBytes, 1048576)
          const url = new URL(command.path, `http://127.0.0.1:${port}`)
          for (const [key, value] of Object.entries(command.query)) url.searchParams.set(key, String(value))
          const answer = await fetch(url, { method: command.method, headers: command.headers,
            body: Object.hasOwn(command, 'body') ? JSON.stringify(command.body) : undefined, signal: command.signal })
          return answer.json()
        } } } }
      }
    }
    const Behaviors = BehaviorRuntimeTest.createBehaviors([{ type: Config, name: 'Config', configuration: true, methods: [] }], 'yon', BehaviorRuntimeTest.moduleId)
    const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
    try {
      const client = context.restClient('catalog')
      const read = await client.get('/products').query({ page: 2 }).header('X-Test', 'present').retrieve()
      assert.equal(read.method, 'GET')
      assert.equal(read.url, '/products?page=2')
      assert.equal(read.header, 'present')
      const written = await client.post('/products').body({ sku: 'anvil' }).retrieve()
      assert.equal(written.method, 'POST')
      assert.deepEqual(JSON.parse(written.sent), { sku: 'anvil' })
      assert.throws(() => client.get('/products').body({}), /body/i)
      assert.throws(() => client.get('/products?page=1'), /path/i)
      assert.throws(() => context.restClient('missing adapter'), /adapter/i)
      await assert.rejects(context.restClient('absent').get('/products').retrieve(), /adapter/i)
      await assert.rejects(client.get('/products').header('Host', 'attacker.example').retrieve(), /header/i)
      assert.deepEqual(seen, ['GET /products?page=2', 'POST /products'])
    } finally { context.dispose() }
  } finally {
    server.closeAllConnections()
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()))
  }
})

await test('a request method fills its path variables from its arguments', async () => {
  class Config {
    static configure() {
      return { http: { catalog: { exchange: (command) => ({ path: command.path, query: command.query }) } } }
    }
  }
  class Catalog {
    constructor(behaviors) { this.behaviors = behaviors }
    async find(id, view) {}
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Catalog, name: 'Catalog', methods: [
      { name: 'find', httpExchange: { client: 'catalog', method: 'GET', path: '/products/{id}', pathVariables: { id: 0 }, query: { view: 1 }, generated: true } },
    ] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    const catalog = new Catalog(context)
    assert.deepEqual(await catalog.find(7, 'full'), { path: '/products/7', query: { view: 'full' } })
    assert.deepEqual(await catalog.find('anvil-2', 'short'), { path: '/products/anvil-2', query: { view: 'short' } })
    await assert.rejects(catalog.find('../secret', 'full'), /path variable/i)
    await assert.rejects(catalog.find({ id: 1 }, 'full'), /path variable/i)
  } finally { context.dispose() }
})

await test('a declared WebCall is a signature whose request comes from its arguments', async () => {
  let bodies = 0
  class Config {
    static configure() {
      return { http: { catalog: { exchange: (command) => ({ method: command.method, path: command.path, query: command.query, headers: command.headers, body: command.body ?? null }) } } }
    }
  }
  class Catalog {
    constructor(behaviors) { this.behaviors = behaviors }
    async all(page, tenant) { bodies += 1 }
    async save(product) { bodies += 1 }
  }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: Catalog, name: 'Catalog', methods: [
      { name: 'all', httpExchange: { client: 'catalog', method: 'GET', path: '/products', query: { page: 0 }, headers: { 'X-Tenant': 1 }, generated: true } },
      { name: 'save', httpExchange: { client: 'catalog', method: 'POST', path: '/products', body: 0, generated: true } },
    ] },
  ], 'yon', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'tenant', signal: new AbortController().signal })
  try {
    const catalog = new Catalog(context)
    assert.deepEqual(await catalog.all(2, 'acme'), {
      method: 'GET', path: '/products', query: { page: 2 }, headers: { 'X-Tenant': 'acme' }, body: null,
    })
    assert.deepEqual(await catalog.save({ sku: 'anvil' }), {
      method: 'POST', path: '/products', query: {}, headers: {}, body: { sku: 'anvil' },
    })
    assert.equal(bodies, 0, 'a declared exchange never calls its empty body')
    await assert.rejects(catalog.all(), /argument is missing/i)
  } finally { context.dispose() }
})

await test('a Mapping is the whole method: a field plan over one JSON argument', async () => {
  let bodies = 0
  class Config { static configure() { return {} } }
  class ProductConverter {
    constructor(behaviors) { this.behaviors = behaviors }
    async toSummary(product) { bodies += 1 }
  }
  const plan = { argument: 0, fields: [
    { source: 'detail.id', target: 'productId' },
    { source: 'name', target: 'title' },
    { source: 'missing.deep', target: 'absent' },
    { target: 'kind', constant: 'product' },
  ] }
  const Behaviors = BehaviorRuntimeTest.createBehaviors([
    { type: Config, name: 'Config', configuration: true, methods: [] },
    { type: ProductConverter, name: 'ProductConverter', methods: [{ name: 'toSummary', mapping: plan }] },
  ], 'tac', BehaviorRuntimeTest.moduleId)
  const context = await Behaviors.create(Config, { partition: 'catalog', signal: new AbortController().signal })
  try {
    const mapper = new ProductConverter(context)
    assert.deepEqual(await mapper.toSummary({ detail: { id: 7 }, name: 'Anvil', extra: 'dropped' }), {
      productId: 7, title: 'Anvil', absent: null, kind: 'product',
    })
    assert.equal(bodies, 0, 'a declared mapping never calls its empty body')
    await assert.rejects(mapper.toSummary(['not an object']), /one JSON object argument/i)
  } finally { context.dispose() }
})

await test('an invalid mapping plan is refused at registration', async () => {
  class Mapper { async map(value) { return value } }
  for (const mapping of [
    { argument: 0, fields: [] },
    { argument: 32, fields: [{ source: 'a', target: 'b' }] },
    { argument: 0, fields: [{ source: 'a', constant: 1, target: 'b' }] },
    { argument: 0, fields: [{ target: 'b' }] },
    { argument: 0, fields: [{ source: 'a', target: 'b' }, { source: 'c', target: 'b' }] },
    { argument: 0, fields: [{ source: 'a.b.c.d.e.f.g.h.i', target: 'b' }] },
    { argument: 0, fields: [{ source: 'a', target: '' }] },
  ]) {
    assert.throws(() => BehaviorRuntimeTest.createBehaviors([{ type: Mapper, name: 'Mapper', methods: [{ name: 'map', mapping }] }], 'tac', BehaviorRuntimeTest.moduleId),
      /mapping/i, JSON.stringify(mapping))
  }
})

process.stdout.write(`Behavior runtime: ${BehaviorRuntimeTest.passed} checks passed.\n`)

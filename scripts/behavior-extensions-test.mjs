import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import vm from 'node:vm'

class BehaviorExtensionsTest {
  static runtimeSource = null
  static create = null
  static moduleId = 'c'.repeat(64)
  static passed = 0
}

BehaviorExtensionsTest.runtimeSource = await readFile(new URL('../crates/tachyon-core/src/behavior/runtime.js', import.meta.url), 'utf8')
BehaviorExtensionsTest.create = vm.runInThisContext(`${BehaviorExtensionsTest.runtimeSource}\n__tachyon_create_behaviors__`)
async function test(name, action) { await action(); console.log(`ok extension ${++BehaviorExtensionsTest.passed} - ${name}`) }

for (const environment of ['tac', 'yon']) {
  await test(`${environment}: Value returns a validated snapshot and assignment is explicit`, async () => {
    let constructed = 0
    class Settings { limit = 0; constructor() { constructed++ } }
    const path = `${environment}.limit`
    const factory = BehaviorExtensionsTest.create([{ type: Settings, name: 'Settings', values: {
      limit: { path, type: 'integer', default: 20, min: 1, max: 100 },
    }, methods: [] }], environment, BehaviorExtensionsTest.moduleId)
    const input = { [environment]: { limit: 25, unrelated: [1, null] } }
    const values = factory.values(Settings, input)
    assert.equal(constructed, 0)
    assert.deepEqual(values, { limit: 25 })
    assert(Object.isFrozen(values))
    const settings = Object.assign(new Settings(), values)
    assert.equal(settings.limit, 25)
    input[environment].limit = 101
    assert.throws(() => Object.assign(settings, factory.values(Settings, input)), { code: 'configuration-properties' })
    assert.equal(settings.limit, 25)
    assert.deepEqual(factory.values(Settings, { [environment]: {} }), { limit: 20 })
    assert.throws(() => factory.values(Settings, {}), { code: 'configuration-properties' })
    for (const limit of [null, [], ['25'], {}, { count: 25 }]) assert.throws(() => factory.values(Settings, { [environment]: { limit } }), { code: 'configuration-properties' })
    assert.throws(() => factory.values(class Unregistered {}, {}), { code: 'configuration-properties' })
    assert.throws(() => factory.values(Settings, { get properties() { throw Error('not evaluated') } }), { code: 'configuration-properties' })
  })

  await test(`${environment}: typed explicit properties and inactive configuration never execute a factory`, async () => {
    const prefix = environment.toUpperCase() + '_CATALOG_'
    let configured = 0
    class Settings {}
    class Configuration { static configure() { configured++; return {} } }
    const schema = { prefix, fields: {
      size: { name: 'SIZE', type: 'integer', default: 20, min: 1, max: 100 },
      enabled: { name: 'ENABLED', type: 'boolean', default: false },
      title: { name: 'TITLE', type: 'string', default: 'Products' },
      factor: { name: 'FACTOR', type: 'number', default: 0.5 },
    } }
    const factory = BehaviorExtensionsTest.create([
      { type: Settings, name: 'Settings', configurationProperties: schema, methods: [] },
      { type: Configuration, name: 'Configuration', configuration: true, profiles: ['test', 'development'], conditionalOnProperty: { name: prefix + 'ENABLED', havingValue: 'true', matchIfMissing: false }, methods: [] },
    ], environment, BehaviorExtensionsTest.moduleId)
    const values = { [prefix + 'SIZE']: '25', [prefix + 'ENABLED']: 'true', [prefix + 'FACTOR']: '1e-2' }
    assert.deepEqual(factory.properties(Settings, values), { enabled: true, factor: 0.01, size: 25, title: 'Products' })
    const nested = { [environment]: { catalog: { size: 25, enabled: true, factor: 0.01 } } }
    assert.deepEqual(factory.properties(Settings, nested), { enabled: true, factor: 0.01, size: 25, title: 'Products' })
    assert.deepEqual(factory.properties(Settings, {}), { enabled: false, factor: 0.5, size: 20, title: 'Products' })
    const invalid = [
      { [prefix + 'SIZE']: '101' }, { [prefix + 'SIZE']: '1.0' }, { [prefix + 'SIZE']: '1e1' },
      { [prefix + 'SIZE']: '+1' }, { [prefix + 'SIZE']: ' 1' }, { [prefix + 'SIZE']: '01' },
      { [prefix + 'SIZE']: '1\n' }, { [prefix + 'SIZE\n']: '1' },
      { [prefix + 'ENABLED']: 'yes' }, { [prefix + 'FACTOR']: '1e100' },
      { [prefix + 'TYPO']: '20' }, { OTHER_SIZE: '20' }, { [prefix + 'TITLE']: 'x'.repeat(4097) },
    ]
    for (const input of invalid) assert.throws(() => factory.properties(Settings, input), { code: 'configuration-properties' })
    const options = { partition: 'tenant', signal: new AbortController().signal, profiles: ['production'], properties: values }
    await assert.rejects(factory.create(Configuration, options), { code: 'inactive-configuration' })
    options.profiles = ['test']
    await assert.rejects(factory.create(Configuration, { ...options, properties: {} }), { code: 'inactive-configuration' })
    assert.equal(configured, 0)
    const context = await factory.create(Configuration, options)
    assert.equal(configured, 1)
    context.dispose()
    const nestedContext = await factory.create(Configuration, { ...options, properties: nested })
    nestedContext.dispose()
    class OptionalConfiguration { static configure() { throw Error('non-scalar value activated configuration') } }
    const optional = BehaviorExtensionsTest.create([{ type: OptionalConfiguration, name: 'OptionalConfiguration', configuration: true, conditionalOnProperty: { name: prefix + 'ENABLED', havingValue: 'true', matchIfMissing: true }, methods: [] }], environment, BehaviorExtensionsTest.moduleId)
    for (const enabled of [null, [], {}, { other: true }]) {
      await assert.rejects(optional.create(OptionalConfiguration, { partition: 'test', signal: new AbortController().signal, properties: { [environment]: { catalog: { enabled } } } }), { code: 'inactive-configuration' })
    }
    await assert.rejects(factory.create(Configuration, { ...options, profiles: ['test', 'test'] }), { code: 'configuration' })
    await assert.rejects(factory.create(Configuration, { ...options, profiles: ['test\n'] }), { code: 'configuration' })
    await assert.rejects(factory.create(Configuration, { ...options, profiles: Array(1) }), { code: 'configuration' })
  })

  await test(`${environment}: validation is pure, bounded, Unicode-aware and precedes method bodies`, async () => {
    class Input {}
    let calls = 0
    class Service { accept(value) { calls++; return value } async save(value) { calls++; return value } }
    const schema = { fields: { title: { notNull: true, notBlank: true, size: { min: 2, max: 4 } }, price: { min: 0, max: 100 }, tags: { size: { min: 0, max: 2 } } } }
    const factory = BehaviorExtensionsTest.create([
      { type: Input, name: 'Input', validated: schema, methods: [] },
      { type: Service, name: 'Service', methods: [{ name: 'accept', valid: { argument: 0, type: 'Input', schema } }, { name: 'save', valid: { argument: 0, type: 'Input', schema } }] },
    ], environment, BehaviorExtensionsTest.moduleId)
    const input = { title: 'éé', price: 10, tags: ['one'], extra: true }
    const output = factory.validate(Input, input)
    assert.deepEqual(output, input)
    assert.notEqual(output, input)
    output.tags.push('two')
    assert.equal(input.tags.length, 1)
    assert.deepEqual(new Service().accept(input), input)
    assert.equal(calls, 1)
    const rejected = new Service().save({})
    assert(rejected instanceof Promise)
    await assert.rejects(rejected, { code: 'validation' })
    for (const [value, path] of [[{}, 'title.notNull'], [{ title: '\u0085\u00a0' }, 'title.notBlank'], [{ title: 'ééé' }, 'title.size'], [{ title: 'good', price: '10' }, 'price.min'], [{ title: 'good', price: 101 }, 'price.max'], [{ title: 'good', tags: [1, 2, 3] }, 'tags.size']]) {
      assert.throws(() => new Service().accept(value), (error) => error.code === 'validation' && error.path === path && !error.message.includes(JSON.stringify(value)))
    }
    assert.equal(calls, 1)
    assert.deepEqual(factory.validate(Input, { title: 'good', price: null }), { title: 'good', price: null })
    assert.throws(() => factory.validate(Input, []), { code: 'validation', path: '$.object' })
    assert.throws(() => factory.validate(Input, { get title() { throw Error('should not run') } }), { code: 'validation', path: '$.json' })
  })

  await test(`${environment}: structured queries authorize before execution and validate results`, async () => {
    let allowed = true
    let executed = 0
    let answer = [{ id: 7 }]
    let captured
    class Configuration { static configure() { return {
      authorizeEntity(command) { captured = command; return allowed },
      entities: { data: { execute() { executed++; return answer } } },
    } } }
    class Products { constructor(behaviors) { this.behaviors = behaviors } async findById() { throw Error('generated body must not run') } }
    const query = { entity: { name: 'Product', adapter: 'data', id: 'id', table: 'products' }, fields: ['id'], defaultSize: 20, maxSize: 100, operation: 'find', where: [{ field: 'id', operator: 'eq', argument: 0 }], pageArgument: null, orderBy: [{ field: 'id', direction: 'asc' }] }
    const factory = BehaviorExtensionsTest.create([
      { type: Configuration, name: 'Configuration', configuration: true, methods: [] },
      { type: Products, name: 'Products', methods: [{ name: 'findById', query }] },
    ], environment, BehaviorExtensionsTest.moduleId)
    const context = await factory.create(Configuration, { partition: 'tenant', signal: new AbortController().signal })
    try {
      const products = new Products(context)
      assert.deepEqual(await products.findById(7), [{ id: 7 }])
      assert.equal(captured.query.where[0].value, 7)
      assert(Object.isFrozen(captured.query.where[0]))
      allowed = false
      await assert.rejects(products.findById(7), { code: 'unauthorized' })
      assert.equal(executed, 1)
      allowed = true
      await assert.rejects(products.findById({ id: 7 }), /scalar/)
      assert.equal(executed, 1)
      for (const value of [[1], Array.from({ length: 21 }, () => ({ id: 7 })), true]) {
        answer = value
        await assert.rejects(products.findById(7), /result/)
      }
    } finally { context.dispose() }
  })
}

await test('yon: Value section strings use the shared UTF-8 byte bound', async () => {
  class Settings {}
  const factory = BehaviorExtensionsTest.create([{ type: Settings, name: 'Settings', values: {
    redis: { path: 'yon.redis.url', type: 'string', default: 'local' },
  }, methods: [] }], 'yon', BehaviorExtensionsTest.moduleId)
  for (const text of ['x'.repeat(4096), 'é'.repeat(2048)]) {
    assert.equal(factory.values(Settings, { yon: { redis: { url: text } } }).redis, text)
  }
  for (const text of ['x'.repeat(4097), 'é'.repeat(2049)]) {
    assert.throws(() => factory.values(Settings, { yon: { redis: { url: text } } }), (error) =>
      error.code === 'configuration-properties' && !error.message.includes(text))
  }
  assert.throws(() => factory.values(Settings, { yon: { datasource: { password: 'x'.repeat(4097) } } }), { code: 'configuration-properties' })
})
console.log(`Behavior extensions: ${BehaviorExtensionsTest.passed} checks passed.`)

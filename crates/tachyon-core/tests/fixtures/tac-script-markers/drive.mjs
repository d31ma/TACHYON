import { readFile } from 'node:fs/promises'

console.info = () => {}
globalThis.indexedDB = { databases: async () => [], open: () => ({}) }
;(0, eval)(await readFile('./tac-storage.js', 'utf8'))

const commands = []
let transactions = 0
const answer = (command) => {
  if (command.operation === 'transaction') return command.action === 'begin' ? { transaction: `t${++transactions}` } : {}
  if (command.operation === 'save') return command.value
  return command.query.kind === 'count' ? 2 : [{ code: 'b6', status: 'lent' }]
}
globalThis.fetch = async (url, init) => {
  const command = JSON.parse(init.body)
  commands.push(command)
  return new Response(JSON.stringify(answer(command)))
}

const { default: ApplicationEvents } = await import('./application-events.js')
const failures = []
const bus = new ApplicationEvents({ changed: () => {}, failed: () => failures.push('failed') })
const open = async (name) => {
  const module = await import(`./.tachyon/services/${name}.js`)
  const signal = new AbortController().signal
  const owner = typeof module.__tachyon_create_owner__ === 'function'
    ? await module.__tachyon_create_owner__({}, signal, undefined)
    : new module.default({})
  module.__tachyon_own_context__?.(owner, signal)
  bus.attach(owner, module.__tachyon_event_plan__?.types[0] ?? {}, { lifecycle: true, view: {}, applicationEventPublisher: true })
  await bus.initialize(owner)
  return owner
}
const outcome = (run) => Promise.resolve().then(run).then(
  (value) => ({ value }),
  (error) => ({ error: error?.code ?? 'error', path: error?.path ?? null, message: String(error?.message ?? error) }),
)
const report = {}

const catalog = await open('CatalogService')
const first = await catalog.find('anvil')
const again = await catalog.find('anvil')
const written = await catalog.write('anvil', 99)
const updated = await catalog.find('anvil')
await catalog.invalidate('anvil')
report.cache = { first, again, written, updated, cleared: await catalog.find('anvil') }

const orders = await open('OrdersService')
const gate = await open('GateService')
const rejected = await Promise.all([outcome(() => gate.once('first')), outcome(() => gate.once('second'))])
const blocked = await Promise.all([gate.queued('a'), gate.queued('b')])
report.resilience = {
  submitted: await orders.submit('o1'),
  rejected,
  blocked,
  steps: gate.steps,
  exhausted: await outcome(() => gate.unreachable()),
  tries: gate.tries,
}

const order = await open('OrderService')
const inputs = {
  accepted: { name: 'Tea', level: 2 },
  NotEmpty: { name: '   ', level: 2 },
  Length: { name: 'x'.repeat(81), level: 2 },
  Min: { name: 'Tea', level: 0 },
  Max: { name: 'Tea', level: 10 },
}
report.validation = {}
for (const [rule, value] of Object.entries(inputs)) report.validation[rule] = await outcome(() => order.accept(value))

const settings = await open('SettingsService')
report.settings = {
  bound: await outcome(() => settings.catalog(35)),
  bounds: [await outcome(() => settings.catalog(0)), await outcome(() => settings.catalog(101)), await outcome(() => settings.catalog(100))],
  values: [
    await outcome(() => settings.values({ tac: { catalog: { title: 'Shop' } } })),
    await outcome(() => settings.values({ tac: { other: { size: 1 } } })),
  ],
  gates: [
    await outcome(() => settings.open(['public'], 'true')),
    await outcome(() => settings.open([], 'true')),
    await outcome(() => settings.open(['public'], 'false')),
    await outcome(() => settings.open(['public'], '')),
  ],
}

const shelf = await open('ShelfService')
report.store = {
  found: await shelf.report(),
  oversized: await outcome(() => shelf.oversized()),
  pages: [
    await outcome(() => shelf.page({})),
    await outcome(() => shelf.page({ page: 2, sort: ['code:desc'] })),
    await outcome(() => shelf.page({ size: 101 })),
    await outcome(() => shelf.page({ sort: ['title'] })),
  ],
  shelved: await outcome(() => shelf.shelve('b1', 'active')),
  refused: await outcome(() => shelf.refuse('b2')),
  commands,
}

const ledger = await open('LedgerService')
const till = await open('TillService')
const started = [...till.steps]
till.post(25)
const returned = [...ledger.heard]
await new Promise((resolve) => setTimeout(resolve, 10))
bus.dispose(till)
report.events = { returned, settled: ledger.heard, started, stopped: till.steps, failures }

console.log(JSON.stringify(report))
process.exit(0)

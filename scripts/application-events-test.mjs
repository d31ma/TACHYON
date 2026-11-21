#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'

class Budget {
  static bytes = 20480
}

async function loadApplicationEvents() {
  const source = await readFile(new URL('../crates/tachyon-core/src/template/application-events.js', import.meta.url), 'utf8')
  assert.ok(Buffer.byteLength(source) <= Budget.bytes, `application-events.js must stay within ${Budget.bytes} bytes`)
  const query = await readFile(new URL('../crates/tachyon-core/src/template/tac-stream-query.js', import.meta.url), 'utf8')
  const moduleUrl = `data:text/javascript;base64,${Buffer.from(`${source}\n${query}\nexport { TacApplicationEvents }`).toString('base64')}`
  const { TacApplicationEvents } = await import(moduleUrl)
  return TacApplicationEvents
}

class Counts {
  static failures = 0
  static changes = 0
}

class Publisher {
  applicationEventPublisher
  send(value) { this.applicationEventPublisher.publishEvent('catalog.changed', value, true); return value }
  async sendAsync(value) { this.applicationEventPublisher.publishEvent('catalog.changed', value); return value }
  reject() { throw new Error('private failure') }
}
class Listener {
  static calls = []
  initialize() { Listener.calls.push('initialize') }
  receive(value) { Listener.calls.push(value); if (value && typeof value === 'object') value.changed = true }
  destroy() { Listener.calls.push('destroy') }
}

async function checkLifecycleAndPublication(bus, resources) {
  const calls = Listener.calls
  const publisher = new Publisher()
  bus.attach(publisher, { resources: [{ field: 'applicationEventPublisher', name: 'applicationEventPublisher' }] }, resources)
  const first = new Listener()
  const second = new Listener()
  const metadata = { post_construct: 'initialize', pre_destroy: 'destroy', resources: [{ field: 'lifecycle', name: 'lifecycle' }, { field: 'view', name: 'view' }], listeners: [{ method: 'receive', event: 'catalog.changed', replay: false }] }
  bus.attach(first, metadata, resources)
  bus.attach(second, metadata, resources)
  await bus.initialize(first)
  await bus.initialize(first)
  assert.deepEqual(calls, ['initialize'], 'OnStart executes once')
  assert.equal(first.view, resources.view)
  assert.equal(first.lifecycle.aborted, false)
  assert.equal(Object.keys(first).includes('view'), false)
  assert.equal('tac' in first, false)
  const value = { count: 1 }
  assert.equal(publisher.send(value), value, 'publisher preserves identity of successful return')
  assert.equal(value.changed, undefined)
  assert.notEqual(calls[1], calls[2], 'each listener owns a clone')
  assert.throws(() => publisher.reject(), /private failure/)
  assert.equal(calls.length, 3, 'failed publisher emits nothing')
  assert.equal(await publisher.sendAsync(7), 7)
  assert.deepEqual(calls.slice(-2), [7, 7])
  const late = new Listener()
  bus.attach(late, { listeners: [{ method: 'receive', event: 'catalog.changed', replay: true }] }, resources)
  await bus.initialize(late)
  assert.deepEqual(calls.at(-1), { count: 1, changed: true }, 'only explicitly retained value replays')
  bus.dispose(first)
  bus.dispose(first)
  assert.equal(first.lifecycle.aborted, true)
  assert.equal(calls.filter(value => value === 'destroy').length, 1)
  const before = calls.length
  publisher.send(8)
  assert.equal(calls.length, before + 2, 'disposed listener is removed exactly once')
  return publisher
}

function checkInvalidPayloads(bus) {
  for (const invalid of [undefined, Infinity, NaN, new Date(), { bad: undefined }, { bad() {} }, 'x'.repeat(393217)]) {
    assert.throws(() => bus.publish('invalid', invalid))
}
const cycle = {}; cycle.self = cycle
assert.throws(() => bus.publish('invalid', cycle))
assert.throws(() => bus.publish('invalid', new Array(2)), /holes/)
const extendedArray = [1]; extendedArray.extra = 2
assert.throws(() => bus.publish('invalid', extendedArray), /indexed/)
assert.throws(() => bus.publish('invalid', ['x'.repeat(196609), 'y'.repeat(196608)]), /393216 bytes/)
assert.doesNotThrow(() => bus.publish('largest', 'x'.repeat(393214)))
assert.throws(() => bus.publish('invalid', 'x'.repeat(393215)), /393216 bytes; send less data/)
const accessor = { get secret() { throw new Error('must not execute') } }
assert.throws(() => bus.publish('invalid', accessor), /data properties/)
}

async function checkAsyncDisposal(bus, resources) {
  let resolve
  const asynchronous = { async receive() { await new Promise(done => { resolve = done }) } }
  bus.attach(asynchronous, { listeners: [{ method: 'receive', event: 'async' }] }, resources)
  bus.publish('async', null)
  assert.equal(bus.pending, 1)
  bus.dispose(asynchronous)
  await new Promise(done => setTimeout(done, 0))
  assert.equal(bus.pending, 0, 'disposal releases async admission without waiting for user work')
  const changedBefore = Counts.changes
  resolve()
  await new Promise(done => setTimeout(done, 0))
  assert.equal(Counts.changes, changedBefore, 'late async completions do not rerender disposed owners')
}

function checkRecursionBound(bus, resources, publisher) {
  const cyclic = {
    applicationEventPublisher: null,
    send(value) { this.applicationEventPublisher.publishEvent('cycle', value) },
    receive(value) { this.send(value) },
}
bus.attach(cyclic, {
  resources: [{ field: 'applicationEventPublisher', name: 'applicationEventPublisher' }],
  listeners: [{ method: 'receive', event: 'cycle' }],
}, resources)
cyclic.send(null)
assert.ok(Counts.failures > 0, 'delivery recursion is bounded and observable')
bus.dispose(publisher)
assert.throws(() => publisher.send(null), /disposed/, 'a disposed owner cannot publish')
}

async function checkListenerIsolation(TacApplicationEvents, resources) {
  const isolated = new TacApplicationEvents({ changed: () => Counts.changes++, failed: () => Counts.failures++ })
  const failureBefore = Counts.failures
  let successfulDeliveries = 0
  for (const listener of [
    { receive() { throw new Error('private sync') } },
    { async receive() { throw new Error('private async') } },
    { receive() { successfulDeliveries++ } },
  ]) isolated.attach(listener, { listeners: [{ method: 'receive', event: 'failure' }] }, resources)
  isolated.publish('failure', null)
  await new Promise(done => setTimeout(done, 0))
  assert.equal(successfulDeliveries, 1, 'failing listeners cannot suppress later listeners')
  assert.equal(Counts.failures, failureBefore + 2)
  isolated.publish('not-retained', 1)
  const notRetained = { receive() { throw new Error('non-retained event replayed') } }
  isolated.attach(notRetained, { listeners: [{ method: 'receive', event: 'not-retained', replay: true }] }, resources)
  await isolated.initialize(notRetained)
  assert.equal(Counts.failures, failureBefore + 2, 'events do not retain by default')
}

function checkBounds(TacApplicationEvents, resources) {
  const bounded = new TacApplicationEvents({ changed: () => {}, failed: () => {} })
  const resourceOwner = {}
  let requestedRenders = 0
  bounded.attach(resourceOwner, { resources: [{ field: 'scheduler', name: 'taskScheduler' }] }, {
    taskScheduler: { render() { requestedRenders++ } },
  })
  resourceOwner.scheduler.render()
  bounded.dispose(resourceOwner)
  assert.throws(() => resourceOwner.scheduler.render(), /disposed/)
  assert.equal(requestedRenders, 1, 'disposed resource references cannot start new runtime work')
  for (let index = 0; index < 128; index++) bounded.attach({ receive() {} }, { listeners: [{ method: 'receive', event: `event-${index}` }] }, resources)
  assert.throws(() => bounded.attach({ receive() {} }, { listeners: [{ method: 'receive', event: 'overflow' }] }, resources), /limit/)
  assert.equal(bounded.listenerCount, 128, 'failed registrations do not leak admission')
  for (let index = 0; index < 128; index++) bounded.publish(`retained-${index}`, null, { retain: true })
  assert.throws(() => bounded.publish('retained-overflow', null, { retain: true }), /limit/)
  assert.equal(bounded.retained.size, 128)
}

class OrderListener {
  static received = []
  static shipped = []
  placed(order) { OrderListener.received.push(order.id); return order.id === 'returns' ? [{ id: 'one' }, null, { id: 'two' }] : undefined }
  recorded() { return { event: 'OrderShipped', value: { id: 'record' } } }
  shipped(order) { OrderListener.shipped.push(order.id) }
}

function orderModels() {
  return {
    OrderPlaced: { open: false, fields: { id: '!string', total: '!integer', note: '?string', lines: '[]Line' } },
    Line: { open: true, fields: { sku: '!string' } },
  }
}

function checkModelPayloads(TacApplicationEvents, resources) {
  let failures = 0
  const bus = new TacApplicationEvents({ changed: () => {}, failed: () => failures++ })
  const listener = new OrderListener()
  bus.attach(listener, { listeners: [
    { method: 'placed', events: ['OrderPlaced'], models: orderModels(), returns: 'OrderShipped' },
    { method: 'shipped', events: ['OrderShipped'], models: { OrderShipped: { open: false, fields: { id: '!string' } } } },
    { method: 'recorded', events: ['OrderPlaced'] },
  ] }, resources)
  for (const fitting of [
    { id: 'a', total: 1 },
    { id: 'b', total: 2, note: null, lines: [{ sku: 's', extra: true }] },
  ]) bus.publish('OrderPlaced', fitting)
  assert.deepEqual(OrderListener.received, ['a', 'b'], 'payloads that fit the listener model are delivered')
  assert.equal(failures, 0)
  for (const misfit of [
    { id: 'c' },
    { id: 'd', total: 1.5 },
    { id: 'e', total: 1, other: 1 },
    { id: 'f', total: 1, lines: [{}] },
    { id: 'g', total: 1, note: 3 },
    [{ id: 'h', total: 1 }],
    'i',
  ]) bus.publish('OrderPlaced', misfit)
  assert.deepEqual(OrderListener.received, ['a', 'b'], 'payloads that do not fit are not delivered')
  assert.equal(failures, 7, 'each payload that does not fit marks a listener failure')
  assert.deepEqual(OrderListener.shipped, [], 'an {event, value} record is not published again')
  bus.publish('OrderPlaced', { id: 'returns', total: 1 })
  assert.deepEqual(OrderListener.shipped, ['one', 'two'], 'a returned list publishes each model value under the return model name')
}

class FakeEventSource {
  static opened = []
  constructor(url) {
    this.url = url
    this.closed = false
    this.listeners = new Map()
    FakeEventSource.opened.push(this)
  }
  addEventListener(type, listener) { this.listeners.set(type, listener) }
  close() { this.closed = true }
  send(data, lastEventId = '') { this.onmessage({ data, lastEventId }) }
  emit(type, data) { this.listeners.get(type)?.({ data, lastEventId: '' }) }
  fail() { this.onerror({}) }
  end() { this.listeners.get('end')({}) }
}

class TicksClient {
  static received = []
  streamTicks(tick) { TicksClient.received.push(tick.level) }
}

class TicksListener {
  static heard = []
  heard(tick) { TicksListener.heard.push(tick.level) }
}

async function checkStreamRequests(TacApplicationEvents, resources) {
  const timers = []
  const realSetTimeout = globalThis.setTimeout
  globalThis.EventSource = FakeEventSource
  globalThis.setTimeout = (run, delay) => { timers.push({ run, delay }); return timers.length }
  try {
    let failures = 0, lastFailure = []
    const bus = new TacApplicationEvents({ changed: () => {}, failed: (error, ...stream) => { failures++; lastFailure = [error?.message, ...stream] } })
    const models = { Tick: { open: false, fields: { sku: '!string', level: '!integer' } } }
    const client = new TicksClient()
    const listener = new TicksListener()
    bus.attach(listener, { listeners: [{ method: 'heard', events: ['Tick'], models }] }, resources)
    bus.attach(client, { streams: [{ method: 'streamTicks', path: '/ticks', event: 'Tick', models }] }, resources)
    assert.equal(FakeEventSource.opened.length, 0, 'a stream waits until its owner starts')
    await bus.initialize(client)
    const first = FakeEventSource.opened[0]
    assert.equal(first.url, '/ticks', 'the stream opens at the StreamRequest path')
    first.send('{"sku":"A","level":3}')
    first.send('{"sku":"A","level":"high"}')
    first.send('not json')
    assert.deepEqual(TicksClient.received, [3], 'only messages that fit the model reach the method')
    assert.deepEqual(TicksListener.heard, [3], 'each message is published as its model event')
    assert.equal(failures, 2, 'a message that does not fit counts as a failure')
    first.fail()
    assert.equal(first.closed, true, 'an error closes the stream')
    const backoff = timers.filter((timer) => [1000, 2000, 4000, 8000, 16000].includes(timer.delay)).map((timer) => timer.delay)
    assert.deepEqual(backoff, [1000], 'the first reconnect waits one second')
    timers.at(-1).run()
    const second = FakeEventSource.opened[1]
    assert.equal(second.url, '/ticks', 'the stream reconnects')
    for (let attempt = 0; attempt < 6; attempt += 1) {
      FakeEventSource.opened.at(-1).fail()
      timers.filter((timer) => [1000, 2000, 4000, 8000, 16000].includes(timer.delay)).at(-1).run()
    }
    const delays = timers.filter((timer) => [1000, 2000, 4000, 8000, 16000].includes(timer.delay)).map((timer) => timer.delay)
    assert.deepEqual(delays, [1000, 2000, 4000, 8000, 16000], 'the first five reconnects back off')
    FakeEventSource.opened.at(-1).fail()
    const slow = timers.at(-1)
    assert.ok(slow.delay >= 30000 && slow.delay <= 35000, 'after five fast tries the stream keeps trying every 30 to 35 seconds')
    const beforeSlow = FakeEventSource.opened.length
    slow.run()
    assert.equal(FakeEventSource.opened.length, beforeSlow + 1, 'a slow try opens the stream again')
    const client2 = new TicksClient()
    bus.attach(client2, { streams: [{ method: 'streamTicks', path: '/more', event: 'Tick', models }] }, resources)
    await bus.initialize(client2)
    const ended = FakeEventSource.opened.at(-1)
    ended.end()
    assert.equal(ended.closed, true, 'the end event closes the stream without a reconnect')
    const client3 = new TicksClient()
    bus.attach(client3, { streams: [{ method: 'streamTicks', path: '/owned', event: 'Tick', models }] }, resources)
    await bus.initialize(client3)
    const owned = FakeEventSource.opened.at(-1)
    bus.dispose(client3)
    assert.equal(owned.closed, true, 'the stream closes with its owner')
    const named = new TicksClient()
    bus.attach(named, { streams: [{ method: 'streamTicks', path: '/.tachyon/reconnect/0/500/30000/1000/.tachyon/event/state/named', event: 'Tick', models }] }, resources)
    await bus.initialize(named)
    const state = FakeEventSource.opened.at(-1)
    assert.equal(state.url, '/named', 'a named stream opens at its server path')
    assert.equal(state.onmessage, undefined, 'a named stream does not read unnamed messages')
    state.emit('state', '{"sku":"N","level":7}')
    assert.deepEqual(TicksClient.received.at(-1), 7, 'a message of the named event type reaches the method')
    assert.deepEqual(TicksListener.heard.at(-1), 7, 'a message of the named event type is published')
    const large = `{"sku":"N","level":8,"note":"${'x'.repeat(70000)}"}`
    state.emit('state', large)
    assert.deepEqual(lastFailure, ['Application event exceeds 64 KiB.', '/named', 'state', large], 'a message over 64 KiB gives its error, stream path, event name and text to the failure handler')
    assert.deepEqual(TicksClient.received.at(-1), 7, 'a message over 64 KiB reaches no method')
    const quiet = timers.length
    state.emit('heartbeat', '')
    assert.equal(timers.at(-1).delay, 60000, 'a heartbeat restarts the sixty-second silence timer')
    assert.ok(timers.length > quiet, 'a heartbeat counts as a sign of life')
    assert.deepEqual(TicksClient.received.at(-1), 7, 'a heartbeat reaches no method')
    bus.dispose(named)
    const queried = new TicksClient()
    const failedBefore = failures
    bus.attach(queried, { streams: [{ method: 'streamTicks', path: '/.tachyon/event/state/.tachyon/query/StatsWindow/stats/{window}', event: 'Tick', models }] }, resources)
    const openedBefore = FakeEventSource.opened.length
    await bus.initialize(queried)
    assert.equal(FakeEventSource.opened.length, openedBefore, 'a stream with a query model waits for its first value')
    bus.publish('StatsWindow', { window: '1 h', limit: 20, note: null })
    const firstWindow = FakeEventSource.opened.at(-1)
    assert.equal(firstWindow.url, '/stats/1%20h?limit=20', 'the first value fills the path and the query')
    assert.equal(FakeEventSource.opened.length, openedBefore + 1, 'the first value opens one stream')
    bus.publish('StatsWindow', { window: '1 h', limit: 20 })
    assert.equal(FakeEventSource.opened.length, openedBefore + 1, 'the same parameters keep the open stream')
    bus.publish('StatsWindow', { window: '24h', limit: 20 })
    const secondWindow = FakeEventSource.opened.at(-1)
    assert.equal(firstWindow.closed, true, 'new parameters close the old stream first')
    assert.equal(secondWindow.url, '/stats/24h?limit=20', 'new parameters open the stream again')
    secondWindow.emit('state', '{"sku":"Q","level":24}')
    assert.equal(TicksClient.received.at(-1), 24, 'the reopened stream delivers its messages')
    bus.publish('StatsWindow', { window: { nested: true } })
    assert.equal(failures, failedBefore + 1, 'a parameter that is not text, a number or a boolean is a failure')
    assert.equal(secondWindow.closed, false, 'a bad value keeps the open stream')
    secondWindow.end()
    bus.publish('StatsWindow', { window: '24h', limit: 20 })
    const thirdWindow = FakeEventSource.opened.at(-1)
    assert.notEqual(thirdWindow, secondWindow, 'publishing the same value reopens a stream the server ended')
    assert.equal(thirdWindow.url, '/stats/24h?limit=20', 'the reopened stream keeps its parameters')
    bus.dispose(queried)
    assert.equal(thirdWindow.closed, true, 'the query stream closes with its owner')
    const later = new TicksClient()
    bus.publish('LaterWindow', { window: '7d' }, { retain: true })
    bus.attach(later, { streams: [{ method: 'streamTicks', path: '/.tachyon/query/LaterWindow/stats/{window}', event: 'Tick', models }] }, resources)
    await bus.initialize(later)
    assert.equal(FakeEventSource.opened.at(-1).url, '/stats/7d', 'a retained value opens the stream at once')
    bus.dispose(later)
  } finally {
    globalThis.setTimeout = realSetTimeout
    delete globalThis.EventSource
  }
}

async function checkTopicStreams(TacApplicationEvents, resources) {
  const timers = []
  const realSetTimeout = globalThis.setTimeout
  globalThis.EventSource = FakeEventSource
  globalThis.setTimeout = (run, delay) => { timers.push({ run, delay }); return timers.length }
  try {
    const bus = new TacApplicationEvents({ changed: () => {}, failed: () => {} })
    const models = { Tick: { open: false, fields: { sku: '!string', level: '!integer' } } }
    const topic = { method: '', path: '/.tachyon/topics/tick', event: 'Tick', models, topic: true }
    const first = new TicksListener()
    const second = new TicksListener()
    TicksListener.heard.length = 0
    const opened = FakeEventSource.opened.length
    for (const owner of [first, second]) {
      bus.attach(owner, { listeners: [{ method: 'heard', events: ['Tick'], models }], streams: [topic] }, resources)
      await bus.initialize(owner)
    }
    assert.equal(FakeEventSource.opened.length, opened + 1, 'listeners of one topic share one stream')
    const stream = FakeEventSource.opened.at(-1)
    assert.equal(stream.url, '/.tachyon/topics/tick', 'a topic listener opens the topic stream')
    stream.send('{"sku":"A","level":3}', '7')
    assert.deepEqual(TicksListener.heard, [3, 3], 'each listener hears a topic record once')
    stream.fail()
    timers.at(-1).run()
    assert.equal(FakeEventSource.opened.at(-1).url, '/.tachyon/topics/tick?position=8', 'a topic stream resumes after its last record')
    bus.dispose(first)
    assert.equal(FakeEventSource.opened.at(-1).closed, false, 'the topic stream stays open while a listener remains')
    bus.dispose(second)
    assert.equal(FakeEventSource.opened.at(-1).closed, true, 'the topic stream closes with its last listener')
  } finally {
    globalThis.setTimeout = realSetTimeout
    delete globalThis.EventSource
  }
}

class FakeWebSocket {
  static opened = []
  constructor(url) {
    this.url = String(url)
    this.readyState = 0
    this.sent = []
    this.closed = false
    FakeWebSocket.opened.push(this)
  }
  connect() { this.readyState = 1; this.onopen({}) }
  send(text) { this.sent.push(text) }
  close() { this.closed = true }
  receive(data) { this.onmessage({ data }) }
  drop(code) { this.readyState = 3; this.onclose({ code }) }
}

class ChatClient {
  static received = []
  receive(message) { ChatClient.received.push(message.text) }
}

async function checkSocketRequests(TacApplicationEvents, resources) {
  const timers = []
  const realSetTimeout = globalThis.setTimeout
  globalThis.WebSocket = FakeWebSocket
  globalThis.location = { href: 'https://shop.example/orders' }
  globalThis.setTimeout = (run, delay) => { timers.push({ run, delay }); return timers.length }
  try {
    let failures = 0
    const bus = new TacApplicationEvents({ changed: () => {}, failed: () => failures++ })
    const models = { ChatMessage: { open: false, fields: { text: '!string' } } }
    const client = new ChatClient()
    bus.attach(client, { streams: [{ method: 'receive', path: '/chat', event: 'ChatMessage', socket: true, models }] }, resources)
    assert.equal(FakeWebSocket.opened.length, 0, 'a socket waits until its owner starts')
    await bus.initialize(client)
    const first = FakeWebSocket.opened[0]
    assert.equal(first.url, 'wss://shop.example/chat', 'the socket opens at the SocketRequest path on the page origin')
    await client.send({ text: 'early' })
    assert.deepEqual(first.sent, [], 'a message waits while the socket connects')
    first.connect()
    assert.deepEqual(first.sent, ['{"text":"early"}'], 'waiting messages are sent when the socket opens')
    await client.send({ text: 'hi' })
    assert.deepEqual(first.sent, ['{"text":"early"}', '{"text":"hi"}'], 'send writes JSON text')
    first.receive('{"text":"hello"}')
    first.receive('{"words":1}')
    assert.deepEqual(ChatClient.received, ['hello'], 'only messages that fit the model reach the method')
    assert.equal(failures, 1, 'a message that does not fit counts as a failure')
    await assert.rejects(client.send({ bad: () => {} }), TypeError, 'send refuses values that are not JSON')
    first.drop(1011)
    const backoff = timers.filter((timer) => timer.delay === 1000)
    assert.equal(backoff.length, 1, 'a failed socket reconnects after one second')
    backoff[0].run()
    const second = FakeWebSocket.opened[1]
    second.connect()
    second.drop(1000)
    assert.equal(timers.filter((timer) => timer.delay === 2000).length, 0, 'a normal close does not reconnect')
    await client.send({ text: 'again' })
    const third = FakeWebSocket.opened[2]
    third.connect()
    assert.deepEqual(third.sent, ['{"text":"again"}'], 'send opens a closed socket again')
    bus.dispose(client)
    assert.equal(third.closed, true, 'the socket closes with its owner')
    await assert.rejects(client.send({ text: 'late' }), /disposed/, 'a disposed owner cannot send')
  } finally {
    globalThis.setTimeout = realSetTimeout
    delete globalThis.WebSocket
    delete globalThis.location
  }
}

async function main() {
  const TacApplicationEvents = await loadApplicationEvents()
  const bus = new TacApplicationEvents({ changed: () => Counts.changes++, failed: () => Counts.failures++ })
  const resources = { view: { id: 'owner' }, lifecycle: null, applicationEventPublisher: null }
  const publisher = await checkLifecycleAndPublication(bus, resources)
  checkInvalidPayloads(bus)
  await checkAsyncDisposal(bus, resources)
  checkRecursionBound(bus, resources, publisher)
  await checkListenerIsolation(TacApplicationEvents, resources)
  checkBounds(TacApplicationEvents, resources)
  checkModelPayloads(TacApplicationEvents, resources)
  await checkStreamRequests(TacApplicationEvents, resources)
  await checkTopicStreams(TacApplicationEvents, resources)
  await checkSocketRequests(TacApplicationEvents, resources)
  console.log('Application events runtime passed: lifecycle, resources, publication, isolation, replay, disposal, cancellation, payload and recursion bounds, model payloads, returned models, StreamRequest streams with named events, heartbeats and query parameters, shared topic streams and SocketRequest sockets.')
}

await main()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { chromium } from 'playwright'

class EventTest {
  static suffix = { js: 'Js', ts: 'Ts' }
  static manifest = { tac: { visibility: { check: false } } }
}

function write(root, name, source) {
  const file = path.join(root, name)
  mkdirSync(path.dirname(file), { recursive: true })
  writeFileSync(file, source)
}

class EventModels {
  static typescript = {
    Counter: `export interface Counter {
  value: integer
}
`,
    Order: `export interface Order {
  status: string
  total: integer
}
`,
    Moved: `export interface Moved {
  id: string
}
`,
    Invoice: `export interface Invoice {
  total: integer
}
`,
  }

  static imports(extension) {
    return Object.keys(EventModels.typescript)
      .map(name => extension === 'ts' ? `import type { ${name} } from '/models/${name}.ts'\n` : `/** @import { ${name} } from '/models/${name}.ts' */\n`)
      .join('')
  }
}

function typed(extension, name, type) {
  return extension === 'ts' ? `const ${name}: ${type}` : `/** @type {${type}} */\n    const ${name}`
}

function listener(extension, parameter, returns = '') {
  if (extension === 'ts') return { doc: '', parameter: `value: ${parameter}`, returns: returns ? `: ${returns}` : '' }
  const tags = [`@param {${parameter}} value`]
  if (returns) tags.push(`@returns {${returns}}`)
  return { doc: `/**\n   * ${tags.join('\n   * ')}\n   */\n  `, parameter: 'value', returns: '' }
}

function serviceSource(extension) {
  const name = `Event${EventTest.suffix[extension]}Service`
  const counter = listener(extension, 'Counter')
  const moved = listener(extension, 'Moved')
  const large = listener(extension, 'Order', 'Invoice')
  const any = listener(extension, 'Order')
  const unpaid = listener(extension, 'Order', extension === 'ts' ? 'Promise<Invoice>' : 'Invoice')
  const invoiced = listener(extension, 'Invoice')
  return `${EventModels.imports(extension)}
@Service()
export default class ${name} {
  @Needs
  view;
  @Needs
  lifecycle;
  @Needs
  taskScheduler;
  applicationEventPublisher;
  result = 0;
  tag = '';
  visible = true;
  _trace = [];
  @OnStart()
  initialize() {
    this.#requireValidOwner();
    this.tag = this.view.tagName;
    globalThis.initializations = this.#countAfter(globalThis.initializations);
    this.publish();
  }
  #requireValidOwner() {
    const problems = ['tac' in this, !this.view.querySelector('#result'), this.lifecycle.aborted];
    if (problems.includes(true)) throw new Error('invalid owner');
  }
  #countAfter(count) { return (count || 0) + 1; }
  publish() {
    ${typed(extension, 'counter', 'Counter')} = { value: this.result + 1 };
    this.applicationEventPublisher.publishEvent(counter, true);
  }
  ${counter.doc}@OnEvent
  receive(${counter.parameter}) { this.result = value.value; }
  reject() { throw new Error('private rejected publication'); }
  exercise() {
    this._trace = [];
    ${typed(extension, 'large', 'Order')} = { status: 'paid', total: 12 };
    ${typed(extension, 'small', 'Order')} = { status: 'paid', total: 3 };
    ${typed(extension, 'open', 'Order')} = { status: 'open', total: 50 };
    ${typed(extension, 'broken', 'Order')} = JSON.parse('{"status":"paid","total":"many"}');
    ${typed(extension, 'shipped', 'Moved')} = { id: 'a' };
    ${typed(extension, 'returned', 'Moved')} = { id: 'b' };
    this.applicationEventPublisher.publishEvent(large);
    this.applicationEventPublisher.publishEvent(small);
    this.applicationEventPublisher.publishEvent(open);
    this.applicationEventPublisher.publishEvent(broken);
    this.applicationEventPublisher.publishEvent(shipped);
    this.applicationEventPublisher.publishEvent(returned);
    return this._trace;
  }
  ${moved.doc}@OnEvent
  moved(${moved.parameter}) { this._trace.push('moved:' + value.id); }
  ${large.doc}@OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})
  @Order(5)
  large(${large.parameter})${large.returns} { this._trace.push('large:' + value.total); return { total: value.total }; }
  ${any.doc}@OnEvent
  @Order(-1)
  any(${any.parameter}) { this._trace.push('any:' + value.total); }
  ${unpaid.doc}@OnEvent({"condition":"value.status != 'paid'"})
  @InBackground()
  async unpaid(${unpaid.parameter})${unpaid.returns} { this._trace.push('unpaid:' + value.total); return { total: -1 }; }
  ${invoiced.doc}@OnEvent
  invoiced(${invoiced.parameter}) { this._trace.push('invoice:' + value.total); }
  @OnStop()
  destroy() { globalThis.pageDestroyed = this.lifecycle.aborted; }
}`
}

function writeServiceFixtures(root) {
  write(root, 'client/manifest.json', JSON.stringify(EventTest.manifest))
  for (const [name, source] of Object.entries(EventModels.typescript)) {
    write(root, `client/models/${name}.ts`, source)
  }
  for (const extension of ['js', 'ts']) {
    const service = `event${EventTest.suffix[extension]}`;
    write(root, `client/pages/${extension}/tac.html`, `<event-${extension} />`)
    write(root, `client/components/event/${extension}/tac.html`, `<main><p id="result">{${service}.result}</p><p id="element">{${service}.tag}</p><button id="publish" on:click="${service}.publish()">Publish</button><button id="hide" on:click="${service}.visible = false">Hide</button><logic :if="${service}.visible"><event-panel /></logic><event-failure /></main>`)
    write(root, `client/services/Event${EventTest.suffix[extension]}Service.${extension}`, serviceSource(extension))
  }
}

function writeSharedComponents(root) {
  write(root, 'client/components/event/panel/tac.html', '<p id="panel">{eventPanel.value}</p><button id="delegated">Delegated</button>')
  write(root, 'client/services/EventPanelService.js', `/** @import { Counter } from '/models/Counter.ts' */
@Service()
@Lifetime({"value":"prototype"})
export default class EventPanelService {
  @Needs
  view;
  @Needs
  lifecycle;
  value = 0;
  /** @param {Counter} value */
  @OnEvent({"replay":true})
  receive(value) { this.value = value.value; globalThis.panelDeliveries = this.#countAfter(globalThis.panelDeliveries); }
  @OnStart()
  initialize() {
    globalThis.panelInitializations = this.#countAfter(globalThis.panelInitializations);
    globalThis.panelElement = this.view;
    this.view.dataset.initialized = 'preserved';
    this.view.classList.add('application-owned');
    this.view.addEventListener('click', () => this.#countDelegated(), {signal: this.lifecycle});
  }
  @OnStop()
  destroy() { globalThis.panelDestroyed = this.lifecycle.aborted; }
  #countDelegated() { globalThis.delegatedDeliveries = this.#countAfter(globalThis.delegatedDeliveries); }
  #countAfter(count) { return (count || 0) + 1; }
}`)
  write(root, 'client/components/event/failure/tac.html', '<p id="failure">{eventFailure.label}</p>')
  write(root, 'client/services/EventFailureService.js', `
@Service()
@Lifetime({"value":"prototype"})
export default class EventFailureService {
  label = 'Preserved failure content';
  @OnStart()
  initialize() {
    globalThis.failureInitializations = this.#countAfter(globalThis.failureInitializations);
    throw new Error('private initialization failure');
  }
  @OnStop()
  destroy() { globalThis.failureCleanups = this.#countAfter(globalThis.failureCleanups); }
  #countAfter(count) { return (count || 0) + 1; }
}`)
}

function serveDist(root) {
  return createServer((request, response) => {
    try {
      const relative = decodeURIComponent(new URL(request.url, 'http://localhost').pathname).replace(/^\/+/, '')
      assert.ok(!relative.split('/').includes('..'))
      const file = relative.includes('.') ? relative : path.join(relative, 'index.html')
      response.setHeader('content-type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.json') ? 'application/json' : 'text/html')
      response.end(readFileSync(path.join(root, 'dist', file)))
    } catch { response.writeHead(404); response.end() }
  })
}

async function checkPage(browser, port, extension) {
  const service = `event${EventTest.suffix[extension]}`
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', error => errors.push(error.message))
  await page.goto(`http://127.0.0.1:${port}/${extension}`)
  await page.waitForFunction(() => document.querySelector('#result')?.textContent === '1' && document.querySelector('#panel')?.textContent === '1')
  await page.waitForFunction(() => document.querySelector('#failure')?.parentElement.dataset.tachyonMountError === 'activation_failed')
  assert.equal(await page.locator('#element').textContent(), 'TACHYON-COMPONENT', 'a service element resource is the host of the first component naming it')
  assert.deepEqual(await page.evaluate((service) => [globalThis.initializations, globalThis.panelInitializations, 'tac' in __tachyonTac.service(service), 'tac' in __tachyonTac], service), [1, 1, false, false])
  await page.locator('#publish').click()
  await page.waitForFunction(() => document.querySelector('#panel')?.textContent === '2')
  assert.deepEqual(await page.evaluate(() => [globalThis.initializations, globalThis.panelInitializations]), [1, 1], 'rerender does not rerun OnStart')
  assert.deepEqual(await page.evaluate(() => {
    const panel = document.querySelector('#panel').parentElement;
    const failure = document.querySelector('#failure').parentElement;
    return [panel.dataset.initialized, panel.classList.contains('application-owned'), panel.dataset.tachyonActive,
      failure.dataset.tachyonActive, failure.dataset.tachyonMountError, globalThis.failureInitializations, globalThis.failureCleanups];
  }), ['preserved', true, 'true', 'false', 'activation_failed', 1, 1], 'rerender preserves owner attributes and terminal initialization failure')
  await page.locator('#delegated').click()
  assert.deepEqual(await page.evaluate(() => [globalThis.delegatedDeliveries, globalThis.panelElement === document.querySelector('#panel').parentElement]), [1, true], 'resource element identity and delegated listeners survive rerender')
  await page.evaluate((service) => { try { __tachyonTac.service(service).reject() } catch {} }, service)
  assert.equal(await page.locator('#result').textContent(), '2')
  assert.equal(await page.evaluate(() => document.documentElement.dataset.tachyonSignalError), undefined)
  assert.deepEqual(await page.evaluate(async (service) => {
    const trace = __tachyonTac.service(service).exercise()
    await new Promise((resolve) => setTimeout(resolve, 50))
    return trace
  }, service), ['any:12', 'large:12', 'invoice:12', 'any:3', 'any:50', 'moved:a', 'moved:b', 'unpaid:50'],
  'typed listeners, a refused payload that does not fit its model, condition, Order, Async and the returned model')
  assert.equal(await page.evaluate(() => document.querySelector('[data-tachyon-signal-error]')?.dataset.tachyonSignalError), 'listener_failed',
    'a payload that does not fit the listener model marks a listener failure')
  await page.locator('#hide').click()
  await page.waitForFunction(() => globalThis.panelDestroyed === true)
  const deliveries = await page.evaluate(() => globalThis.panelDeliveries)
  await page.evaluate((service) => __tachyonTac.service(service).publish(), service)
  assert.equal(await page.evaluate(() => globalThis.panelDeliveries), deliveries)
  await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true })))
  assert.equal(await page.evaluate(() => globalThis.pageDestroyed), undefined, 'bfcache suspension is not disposal')
  await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false })))
  assert.equal(await page.evaluate(() => globalThis.pageDestroyed), true)
  assert.deepEqual(errors, [])
  await page.close()
}

async function main() {
  const binary = process.env.TAC_BIN
  assert.ok(binary, 'Set TAC_BIN to the newly compiled ty binary')
  const root = mkdtempSync(path.join(tmpdir(), 'tac-events-'))
  writeServiceFixtures(root)
  writeSharedComponents(root)
  let server
  let browser
  try {
    const built = spawnSync(binary, ['build', root], { encoding: 'utf8', timeout: 120000,
      env: { ...process.env, PATH: `${path.resolve('node_modules/.bin')}${path.delimiter}${process.env.PATH || ''}` } })
    assert.equal(built.status, 0, built.stdout + built.stderr)
    server = serveDist(root)
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
    browser = await chromium.launch({ headless: true })
    for (const extension of ['js', 'ts']) await checkPage(browser, server.address().port, extension)
    console.log('Tac event browser contract passed: JS/TS typed listeners and model publishing, model payload checks, injected publisher, resources, DOM-ready initialization, retained replay, conditions, Order, Async, returned models, exact disposal, cancellation and public-context removal.')
  } finally {
    await browser?.close()
    server?.closeAllConnections()
    if (server) await new Promise(resolve => server.close(resolve))
    rmSync(root, { recursive: true, force: true })
  }
}

await main()

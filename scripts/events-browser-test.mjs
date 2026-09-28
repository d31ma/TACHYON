#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { spawnSync } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, statSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

class EventsGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static project = path.join(tmpdir(), 'ty-events-gate');
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' };
}

function write(relative, contents) {
  const file = path.join(EventsGate.project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

function writeFixture() {
  rmSync(EventsGate.project, { recursive: true, force: true });
  write('client/pages/tac.html', '<events-page />\n');
  write('client/components/events/page/tac.html', `<main aria-label="Events">
  <output id="count">{events.count}</output>
  <button on:click="events.increment()">Add one</button>
  <button on:click="events.addBy(5)">Add five</button>
  <input id="name" on:input="events.rename()">
  <p id="greeting">{events.greeting}</p>
  <input id="typed" on:input="events.record($event.target.value)">
  <p id="echo">{events.echo}</p>
  <input id="labelled" on:input="events.labelled('email', $event.target.value)">
  <p id="pair">{events.pair}</p>
  <button id="raw" on:click="events.describe($event)">Describe</button>
  <p id="described">{events.described}</p>
  <button id="order" on:click="events.order(12)">Order</button>
  <button id="small" on:click="events.order(3)">Small order</button>
  <p id="trace">{events.trace}</p>
</main>
`);
  write('client/services/EventsService.js', `/**
 * @typedef {Object} Order
 * @property {integer} total
 */
/**
 * @typedef {Object} Invoice
 * @property {integer} total
 */
@Service()
export default class EventsService {
  static ANY = 'any:'
  applicationEventPublisher
  count = 0
  greeting = '-'
  echo = '-'
  pair = '-'
  described = '-'
  trace = '-'
  increment() { this.count += 1 }
  addBy(_event, amount) { this.count += amount }
  rename(event) { this.greeting = 'Hello ' + event.target.value }
  record(_event, value) { this.echo = String(value) }
  labelled(_event, field, value) { this.pair = field + '=' + String(value) }
  describe(_event, passed) { this.described = this.#typeOf(passed) }
  order(_event, total) {
    this.trace = ''
    /** @type {Order} */
    const order = { total }
    this.applicationEventPublisher.publishEvent(order)
  }
  /**
   * @param {Order} value
   * @returns {Invoice}
   */
  @OnEvent({"condition":"value.total >= 10"})
  @Order(2)
  large(value) { this.#note('large:' + value.total); return { total: value.total } }
  /** @param {Order} value */
  @OnEvent
  @Order(1)
  any(value) { this.#note(EventsService.ANY + value.total) }
  /** @param {Invoice} value */
  @OnEvent
  @Order(1)
  invoiced(value) { this.#note(EventsService.ANY + value.total) }
  /** @param {Order} value */
  @OnEvent
  @InBackground()
  async audit(value) { this.#note('audit:' + value.total) }
  #typeOf(passed) { return passed?.type ?? 'missing' }
  #note(entry) { this.trace = this.trace ? this.trace + ',' + entry : entry }
}
`);
}

function buildFixture() {
  const built = spawnSync(EventsGate.binary, ['build', EventsGate.project], { encoding: 'utf8' });
  if (built.status !== 0) {
    console.error(built.stderr || built.stdout);
    process.exit(1);
  }
}

function serveDist() {
  const root = path.join(EventsGate.project, 'dist');
  return createServer((request, response) => {
    let file = path.join(root, decodeURIComponent(request.url.split('?')[0]));
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': EventsGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' });
    createReadStream(file).pipe(response);
  });
}

function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exitCode = 1;
}

function expect(actual, wanted, label) {
  if (actual === wanted) console.log(`    ok   ${label}: ${actual}`);
  else fail(`${label}: expected ${wanted}, got ${actual}`);
}

async function checkEvents(page, origin, errors) {
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  expect(await page.textContent('#count'), '0', 'client-rendered value');

  await page.click('text=Add one');
  await page.waitForTimeout(150);
  expect(await page.textContent('#count'), '1', 'handler with no arguments');

  await page.click('text=Add five');
  await page.waitForTimeout(150);
  expect(await page.textContent('#count'), '6', 'handler with a literal argument');

  await page.fill('#name', 'Ada');
  await page.waitForTimeout(200);
  expect(await page.textContent('#greeting'), 'Hello Ada', 'handler receiving the event');

  await page.fill('#typed', 'Grace');
  await page.waitForTimeout(200);
  expect(await page.textContent('#echo'), 'Grace', '$event.target.value argument');

  await page.fill('#labelled', 'ada@example.test');
  await page.waitForTimeout(200);
  expect(await page.textContent('#pair'), 'email=ada@example.test', 'literal beside $event path');

  await page.click('#raw');
  await page.waitForTimeout(200);
  expect(await page.textContent('#described'), 'click', 'bare $event argument');

  await page.click('#order');
  await page.waitForTimeout(200);
  expect(await page.textContent('#trace'), 'any:12,large:12,any:12,audit:12', 'injected publisher, typed listeners, condition, Order, Async, returned model');
  await page.click('#small');
  await page.waitForTimeout(200);
  expect(await page.textContent('#trace'), 'any:3,audit:3', 'condition skips the listener it guards');

  if (errors.length) fail(`console errors: ${errors.join(' | ')}`);
  else console.log('    ok   no console errors');
}

async function main() {
  writeFixture();
  buildFixture();
  const server = serveDist();

  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
  page.on('pageerror', (error) => errors.push(String(error)));

  try {
    await checkEvents(page, origin, errors);
  } finally {
    await browser.close();
    server.close();
  }

  console.log(process.exitCode ? 'FAIL: Tac client event gate' : 'PASS: Tac client event gate');
}

await main();

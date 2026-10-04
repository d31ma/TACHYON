#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { spawnSync } from 'node:child_process';
import { createReadStream, existsSync, mkdirSync, statSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';

class TopicsGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static project = path.join(tmpdir(), 'ty-topics-gate');
  static topicPath = '/.tachyon/topics/order-placed';
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' };
  static streams = [];
  static requests = [];
  static auditServices = {
    javascript: {
      'client/services/AuditService.js': `@Service()
export default class AuditService {
  seen = 0
  /** @param {OrderPlaced} order */
  @OnEvent
  count(order) { this.seen += 1 }
}
`,
    },
    lua: {
      'client/models/OrderPlaced.lua': `-- @Topic
---@class OrderPlaced
---@field id string
OrderPlaced = {}
`,
      'client/services/AuditService.lua': `-- @Service
---@class AuditService
AuditService = {
  ---@type integer
  seen = 0,
}

---@param order OrderPlaced
---@return nil
-- @OnEvent
function AuditService:count(order)
  self.seen = self.seen + 1
end
`,
    },
  };
}

function write(relative, contents) {
  const file = path.join(TopicsGate.project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

function writeFixture(language) {
  rmSync(TopicsGate.project, { recursive: true, force: true });
  write('client/pages/tac.html', '<orders-page />\n');
  write('client/components/orders/page/tac.html', `<main aria-label="Orders">
  <p id="placed">{orders.placed}</p>
  <p id="audited">{audit.seen}</p>
</main>
`);
  write('client/models/OrderPlaced.js', `/**
 * @Topic
 * @typedef {Object} OrderPlaced
 * @property {string} id
 */
export {}
`);
  write('client/services/OrdersService.js', `@Service()
export default class OrdersService {
  placed = '-'
  /** @param {OrderPlaced} order */
  @OnEvent
  record(order) { this.placed = this.#withId(order.id) }
  #withId(id) { return this.placed === '-' ? id : this.placed + ',' + id }
}
`);
  for (const [relative, contents] of Object.entries(TopicsGate.auditServices[language])) write(relative, contents);
}

function buildFixture() {
  const built = spawnSync(TopicsGate.binary, ['build', TopicsGate.project], { encoding: 'utf8' });
  if (built.status !== 0) {
    console.error(built.stderr || built.stdout);
    process.exit(1);
  }
}

function serve() {
  const root = path.join(TopicsGate.project, 'dist');
  return createServer((request, response) => {
    const [pathname] = request.url.split('?');
    if (pathname === TopicsGate.topicPath) {
      TopicsGate.requests.push(request.url);
      response.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-store' });
      response.write(': open\n\n');
      TopicsGate.streams.push(response);
      return;
    }
    let file = path.join(root, decodeURIComponent(pathname));
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': TopicsGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' });
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

async function checkTopics(page, origin, errors) {
  await page.goto(`${origin}/`, { waitUntil: 'load' });
  await page.waitForSelector('#placed');
  for (let attempt = 0; attempt < 50 && TopicsGate.streams.length === 0; attempt += 1) await page.waitForTimeout(100);
  expect(TopicsGate.requests.length, 1, 'two listeners of one topic open one stream');
  expect(TopicsGate.requests[0], TopicsGate.topicPath, 'the page opens the topic stream by itself');
  for (const stream of TopicsGate.streams) {
    stream.write('id: 0\ndata: {"id":"A-1"}\n\n');
    stream.write('id: 1\ndata: {"id":"A-2"}\n\n');
    stream.write('id: 2\ndata: {"id":7}\n\n');
  }
  await page.waitForFunction(() => document.querySelector('#placed')?.textContent === 'A-1,A-2', null, { timeout: 5000, polling: 100 }).catch(() => {});
  expect(await page.textContent('#placed'), 'A-1,A-2', 'each record that fits the model reaches the listener');
  expect(await page.textContent('#audited'), '2', 'a second listener hears the same records once');
  const unexpected = errors.filter((error) => !error.includes('EventSource'));
  if (unexpected.length) fail(`console errors: ${unexpected.join(' | ')}`);
  else console.log('    ok   no console errors');
}

async function checkLanguage(language) {
  console.log(`  ${language} AuditService`);
  TopicsGate.streams = [];
  TopicsGate.requests = [];
  writeFixture(language);
  buildFixture();
  const server = serve();
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
  page.on('pageerror', (error) => errors.push(String(error)));
  try {
    await checkTopics(page, origin, errors);
  } finally {
    await browser.close();
    for (const stream of TopicsGate.streams) stream.end();
    server.close();
  }
}

async function main() {
  for (const language of Object.keys(TopicsGate.auditServices)) await checkLanguage(language);
  console.log(process.exitCode ? 'FAIL: Tac topic event gate' : 'PASS: Tac topic event gate');
}

await main();

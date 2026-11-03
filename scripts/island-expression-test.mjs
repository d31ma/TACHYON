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

class IslandExpressionGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static project = path.join(tmpdir(), 'ty-island-expression-gate');
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' };
}

function write(relative, contents) {
  const file = path.join(IslandExpressionGate.project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

function writeFixture() {
  rmSync(IslandExpressionGate.project, { recursive: true, force: true });
  write('client/pages/tac.html', '<panel-status />\n');
  write('client/components/panel/status/tac.html', `<section>
  <p id="method">{panelStatus.loadingState()}</p>
  <p id="field">{panelStatus.label}</p>
  <p id="maths">{panelStatus.count + 1}</p>
  <p id="ternary">{panelStatus.count > 3 ? 'many' : 'few'}</p>
  <p id="nested">{panelStatus.report.title}</p>
  <p id="chained">{panelStatus.report.rows.length}</p>
  <p id="awaited">{await panelStatus.note()}</p>
  <p id="live">{panelStatus.count}</p>
  <logic :if="panelStatus.count > 0">
    <ul id="rows"><loop :for="row of panelStatus.report.rows"><li>{row}</li></loop></ul>
  </logic>
  <logic else><p id="empty">empty</p></logic>
  <button id="bump" on:click="panelStatus.count += 1">bump</button>
  <button id="reset" on:click="panelStatus.reset()">reset</button>
  <input id="typed" aria-label="Label" on:input="panelStatus.label = $event.target.value">
  <p id="typedOut">{panelStatus.label}</p>
  <section id="lexical-scope">
    <loop :for="account of panelStatus.accounts">
      <p class="outer-before">{account.domain}</p>
      <loop :for="role of panelStatus.roles">
        <p class="two-level" :data-pair="panelStatus.value(account, role)">{panelStatus.value(account, role)}</p>
        <logic :if="panelStatus.value(account, role) === 'example.com:inbox'">
          <p class="matched-pair">{panelStatus.value(account, role)}</p>
        </logic>
        <scope-probe :label="panelStatus.value(account, role)" />
        <button class="select-pair" on:click="panelStatus.select(account.domain, role.id)">select</button>
        <loop :for="folder of panelStatus.foldersFor(account, role)">
          <p class="three-level">{panelStatus.triple(account, role, folder)}</p>
        </loop>
      </loop>
      <loop :for="account of panelStatus.shadowAccounts">
        <p class="shadowed-account">{account.domain}</p>
      </loop>
      <p class="outer-after">{account.domain}</p>
    </loop>
    <output id="selection">{panelStatus.selection}</output>
  </section>
</section>
`);
  write('client/services/PanelStatusService.js', `@Service()
export default class PanelStatusService {
  static _MISSING = 'missing'
  label = 'ready'
  count = 6
  report = { title: 'Report', rows: [1, 2, 3] }
  accounts = [{ domain: 'example.com' }, { domain: 'work.test' }]
  roles = [{ id: 'inbox' }]
  shadowAccounts = [{ domain: 'shadow.test' }]
  selection = 'none'
  loadingState() { return 'live' }
  async note() { return 'awaited' }
  reset() { this.count = 0 }
  value(account, role) { return \`\${this.#domainOf(account)}:\${this.#idOf(role)}\` }
  foldersFor(account, role) {
    const given = this.#bothGiven(account, role)
    return this.#listWhen(given, { id: this.#folderId(account) })
  }
  triple(account, role, folder) {
    return \`\${this.#domainOf(account)}:\${this.#idOf(role)}:\${this.#idOf(folder)}\`
  }
  select(_event, domain, role) { this.selection = \`\${domain}:\${role}\` }
  #domainOf(account) { return account?.domain || PanelStatusService._MISSING }
  #idOf(item) { return item?.id || PanelStatusService._MISSING }
  #bothGiven(account, role) { return Boolean(account && role) }
  #folderId(account) { return account?.domain === 'example.com' ? 'primary' : 'archive' }
  #listWhen(given, item) {
    if (!given) return []
    return [item]
  }
}
`);
  write('client/components/scope/probe/tac.html', '<p class="component-prop">{label}</p>\n');
}

function buildFixture() {
  const built = spawnSync(IslandExpressionGate.binary, ['build', IslandExpressionGate.project], { encoding: 'utf8' });
  if (built.status !== 0) {
    console.error(built.stderr || built.stdout);
    process.exit(1);
  }
}

function serveDist() {
  const root = path.join(IslandExpressionGate.project, 'dist');
  return createServer((request, response) => {
    let file = path.join(root, decodeURIComponent(request.url.split('?')[0]));
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': IslandExpressionGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' });
    createReadStream(file).pipe(response);
  });
}

function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exitCode = 1;
}

function expect(actual, wanted, label) {
  if (actual === wanted) console.log(`    ok   ${label}: ${actual}`);
  else fail(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`);
}

async function checkPage(page, origin, errors) {
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
  await page.waitForFunction(() => document.documentElement.dataset.tachyonRendered === 'client', null, { timeout: 5000 });

  expect(await page.textContent('#method'), 'live', 'method call on the service');
  expect(await page.textContent('#field'), 'ready', 'service field');
  expect(await page.textContent('#maths'), '7', 'arithmetic over a service field');
  expect(await page.textContent('#ternary'), 'many', 'conditional over a service field');
  expect(await page.textContent('#nested'), 'Report', 'nested property');
  expect(await page.textContent('#chained'), '3', 'property of a nested array');
  expect(await page.textContent('#awaited'), 'awaited', 'awaited service method');
  expect((await page.locator('#rows li').allTextContents()).join(','), '1,2,3', 'client loop renders');
  expect(
    (await page.locator('.two-level').allTextContents()).join(','),
    'example.com:inbox,work.test:inbox',
    'two-level loops retain outer bindings',
  );
  expect(
    (await page.locator('.two-level').evaluateAll((nodes) => nodes.map((node) => node.dataset.pair))).join(','),
    'example.com:inbox,work.test:inbox',
    'dynamic attributes retain outer bindings',
  );
  expect(
    (await page.locator('.matched-pair').allTextContents()).join(','),
    'example.com:inbox',
    'conditions retain outer bindings',
  );
  expect(
    (await page.locator('.component-prop').allTextContents()).join(','),
    'example.com:inbox,work.test:inbox',
    'component properties retain caller bindings',
  );
  expect(
    (await page.locator('.three-level').allTextContents()).join(','),
    'example.com:inbox:primary,work.test:inbox:archive',
    'three-level loops retain every parent binding',
  );
  expect(
    (await page.locator('.outer-before').allTextContents()).join(','),
    'example.com,work.test',
    'outer binding before a shadowing loop',
  );
  expect(
    (await page.locator('.shadowed-account').allTextContents()).join(','),
    'shadow.test,shadow.test',
    'nearest loop binding shadows its parent',
  );
  expect(
    (await page.locator('.outer-after').allTextContents()).join(','),
    'example.com,work.test',
    'shadowed binding does not leak into a sibling',
  );

  await page.locator('.select-pair').nth(1).click();
  await page.waitForTimeout(150);
  expect(await page.textContent('#selection'), 'work.test:inbox', 'event arguments retain parent bindings');

  expect(await page.textContent('#live'), '6', 'initial value');
  await page.click('#bump');
  await page.waitForFunction(() => document.querySelector('#live').textContent === '7');
  expect(await page.textContent('#live'), '7', 'compound assignment updates the view');
  await page.click('#bump');
  await page.waitForTimeout(100);
  expect(await page.textContent('#live'), '8', 'assignment accumulates');

  await page.click('#reset');
  await page.waitForTimeout(100);
  expect(await page.textContent('#live'), '0', 'service method updates the view');
  expect(await page.textContent('#maths'), '1', 'every expression refreshes, not just one');
  expect(await page.locator('#rows').count(), 0, 'client conditional removes its branch');
  expect(await page.textContent('#empty'), 'empty', 'client else branch renders');

  await page.fill('#typed', 'typed value');
  await page.waitForTimeout(150);
  expect(await page.textContent('#typedOut'), 'typed value', 'assignment from $event');

  expect(
    await page.getAttribute('tachyon-component', 'data-tachyon-active'),
    'true',
    'component mounted',
  );
  expect(await page.getAttribute('tachyon-component', 'data-tachyon-mount-error'), null, 'no mount error');
  expect(errors.length, 0, `no console errors${errors.length ? `: ${errors[0]}` : ''}`);
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
    await checkPage(page, origin, errors);
  } finally {
    await browser.close();
    server.close();
  }

  if (!process.exitCode) console.log('\nTac client expression gate passed');
}

await main();

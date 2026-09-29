#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

class BehaviorGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static frameworkPrefix = 'TAC_';
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static fixtureFiles = [
    'setups/ApplicationAdaptersSetup.js',
    'helpers/ApplicationSettings.js',
    'helpers/ApplicationValues.js',
    'helpers/BehaviorChecksHelper.js',
    'clients/CatalogClient.js',
    'helpers/CatalogHelper.js',
    'setups/CatalogSetup.js',
    'helpers/InputService.js',
    'helpers/ProductInput.js',
    'entities/ProductEntity.js',
    'stores/ProductQueriesStore.js',
  ];
}

function write(project, name, contents) {
  const destination = path.join(project, name);
  mkdirSync(path.dirname(destination), { recursive: true });
  writeFileSync(destination, contents);
}

function writeBehaviorProviders(project, extension) {
  const directory = path.join(BehaviorGate.repo, 'crates/tachyon-core/tests/fixtures/behavior/authored/js/server');
  for (const relative of BehaviorGate.fixtureFiles) {
    const source = readFileSync(path.join(directory, relative), 'utf8')
      .replaceAll('TAC_', BehaviorGate.frameworkPrefix);
    write(project, `client/${relative.replace(/\.js$/, `.${extension}`)}`, withPrivateMembers(source, extension));
  }
}

function withPrivateMembers(source, extension) {
  if (extension === 'js') return source;
  return source.replace(/^(\s*)static _/gm, '$1private static _');
}

function writePublishedExample(project, features) {
  const published = features.features.find((feature) => feature.id === 'behavior-annotations');
  assert.ok(published, 'published behavior example exists');
  for (const file of published.files.filter((file) => file.name.startsWith('client/'))) {
    write(project, file.name.startsWith('client/pages/') ? file.name.replace('client/pages/', 'client/pages/published/') : file.name, file.code);
  }
}

function writeBehaviorServices(project) {
  const suffix = { js: 'Js', ts: 'Ts' };
  const privateStatic = { js: 'static', ts: 'private static' };
  for (const extension of ['js', 'ts']) {
    writeBehaviorProviders(project, extension);
    write(project, `client/pages/${extension}/tac.html`, `<behaviors-${extension} />`);
    write(project, `client/components/behaviors/${extension}/tac.html`, `<main><p id="result">{behaviors${suffix[extension]}.result}</p></main>`);
    write(project, `client/services/Behaviors${suffix[extension]}Service.${extension}`, `
@Service()
export default class Behaviors${suffix[extension]}Service {
  ${privateStatic[extension]} _NOT_THE_HOST = 'Service owner root is not the component host';
  @Needs
  view;
  @Needs
  lifecycle;
  result = 'pending';
  @OnStart()
  async initialize() {
    this.#requireConnected(this.view);
    this.#requireNotBody(this.view);
    this.lifecycle.addEventListener('abort', () => this.#markAborted(), { once: true });
    this.result = JSON.stringify(await this.#exercise(this.lifecycle));
  }
  #requireConnected(root) {
    if (!root?.isConnected) throw new Error(Behaviors${suffix[extension]}Service._NOT_THE_HOST);
  }
  #requireNotBody(root) {
    if (root === document.body) throw new Error(Behaviors${suffix[extension]}Service._NOT_THE_HOST);
  }
  #markAborted() {
    document.documentElement.dataset.pageBehaviorAborted = 'yes';
  }
  async #exercise(signal) {
    try { return await BehaviorChecksHelper.exerciseBehaviors(signal); }
    catch (error) { document.documentElement.dataset.behaviorFailure = String(error); throw error; }
  }
}`);
  }
}

function writePendingFixtures(project) {
  write(project, 'client/pages/tac.html', '<gate-page />');
  write(project, 'client/components/gate/page/tac.html', '<main><button id="hide" on:click="gatePage.show = false">Hide</button><logic :if="gatePage.show"><pending-request /></logic></main>');
  write(project, 'client/services/GatePageService.js', "@Service()\nexport default class GatePageService { show = true; }");
  write(project, 'client/components/pending/request/tac.html', '<p>{pendingRequest.label}</p>');
  write(project, 'client/setups/SlowAdaptersSetup.js', `
@Setup()
class SlowAdaptersSetup {
  static configure() {
    return { http: { slow: { exchange(request) {
      document.documentElement.dataset.behaviorStarted = 'yes';
      return new Promise((_resolve, reject) => request.signal.addEventListener('abort', () => {
        document.documentElement.dataset.behaviorAborted = 'yes';
        reject(new Error('cancelled'));
      }, { once: true }));
    } } } };
  }
}
`);
  write(project, 'client/clients/SlowClient.js', `
@HTTPClient({"adapter":"slow"})
class SlowClient {
  @GetRequest("/slow")
  async request() {}
}
`);
  write(project, 'client/services/PendingRequestService.js', `
@Service()
@Lifetime({"value":"prototype"})
export default class PendingRequestService {
  @Needs
  lifecycle;
  label = 'Pending request';
  @OnStart()
  async initialize() {
    const signal = this.lifecycle;
    const context = await Behaviors.create(SlowAdaptersSetup, { partition: 'public-test', signal, timeoutMs: 10000 });
    Object.assign(new SlowClient(), { behaviors: context }).request().catch(() => {});
  }
}`);
}

function buildProject(project) {
  const built = spawnSync(BehaviorGate.binary, ['build', project], {
    encoding: 'utf8', timeout: 120000,
    env: { ...process.env, PATH: `${path.join(BehaviorGate.repo, 'node_modules/.bin')}${path.delimiter}${process.env.PATH ?? ''}` },
  });
  assert.equal(built.status, 0, `${built.stdout}\n${built.stderr}\n${built.error ?? ''}`);
}

function serveDist(project) {
  const output = path.join(project, 'dist');
  return createServer((request, response) => {
    try {
      const url = new URL(request.url, 'http://localhost');
      const relative = decodeURIComponent(url.pathname).replace(/^\/+/, '');
      const name = path.resolve(output, relative.endsWith('/') || !relative ? `${relative}index.html` : relative);
      assert.ok(name.startsWith(`${output}${path.sep}`));
      const file = path.extname(name) ? name : path.join(name, 'index.html');
      response.setHeader('Content-Type', file.endsWith('.js') ? 'text/javascript' : file.endsWith('.json') ? 'application/json' : 'text/html');
      response.end(readFileSync(file));
    } catch {
      response.writeHead(404).end();
    }
  });
}

async function checkServices(page, origin) {
  for (const extension of ['js', 'ts']) {
    const response = await page.goto(`${origin}/${extension}/`);
    assert.ok(!(await response.text()).includes('<p id="result">pending</p>'), 'Tac must not SSR');
    await page.waitForFunction(() => document.querySelector('#result')?.textContent?.startsWith('{') || document.documentElement.dataset.behaviorFailure);
    assert.equal(await page.evaluate(() => document.documentElement.dataset.behaviorFailure), undefined);
    const result = JSON.parse(await page.locator('#result').textContent());
    assert.deepEqual(result.first, result.cached);
    assert.equal(result.fresh.calls, 2);
    assert.equal(result.page.offset, 10);
    assert.equal(result.remote.path, '/products');
    assert.equal(result.saved.operation, 'save');
    assert.equal(result.deleted.operation, 'delete');
    assert.deepEqual(result.configuration.settings, { label: 'Products', enabled: true, limit: 12, ratio: 1.5 });
    assert.equal(result.configuration.inactive, 'inactive-configuration');
    assert.deepEqual(result.configuration.selectionErrors, ['inactive-configuration', 'inactive-configuration']);
    assert.equal(result.configuration.propertyError, 'configuration-properties');
    assert.equal(result.configuration.calls, 1);
    assert.deepEqual(result.validation.input, { id: 7, name: 'é', price: 5, extra: true, validated: true });
    assert.deepEqual(result.validation.constraintPaths, ['id.notNull', 'name.size', 'price.min', 'price.max']);
    assert.equal(result.validation.path, 'name.notBlank');
    assert.equal(result.validation.calls, 1);
    assert.equal(result.validation.copyPreserved, true);
    await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true })));
    assert.equal(await page.evaluate(() => document.documentElement.dataset.pageBehaviorAborted), undefined);
    await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false })));
    assert.equal(await page.evaluate(() => document.documentElement.dataset.pageBehaviorAborted), 'yes');
  }
}

async function checkPublishedExample(page, origin) {
  await page.goto(`${origin}/published/`);
  await page.waitForFunction(() => document.body.textContent.includes('Next page begins at 20'));
}

async function checkUnmountCancellation(page, origin, errors) {
  await page.goto(origin);
  await page.waitForFunction(() => document.documentElement.dataset.behaviorStarted === 'yes');
  await page.locator('#hide').click();
  await page.waitForFunction(() => document.documentElement.dataset.behaviorAborted === 'yes');
  assert.deepEqual(errors, [], 'no browser runtime failures');
}

async function main() {
  const project = mkdtempSync(path.join(tmpdir(), 'tac-behaviors-'));
  const features = JSON.parse(readFileSync(path.join(BehaviorGate.repo, 'website/client/shared/data/features.json'), 'utf8'));
  let browser;
  let server;
  try {
    writePublishedExample(project, features);
    writeBehaviorServices(project);
    writePendingFixtures(project);
    buildProject(project);
    server = serveDist(project);
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await checkServices(page, origin);
    await checkPublishedExample(page, origin);
    await checkUnmountCancellation(page, origin, errors);
    console.log('Tac behavior annotations: JS + TS services, page lifecycle, service execution and unmount cancellation passed.');
  } finally {
    await browser?.close();
    if (server) {
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    }
    rmSync(project, { recursive: true, force: true });
  }
}

await main();

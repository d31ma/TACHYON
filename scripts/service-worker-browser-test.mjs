#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { chromium } from 'playwright';
import { createServer } from 'node:https';
import { spawnSync } from 'node:child_process';
import { cpSync, createReadStream, existsSync, mkdirSync, mkdtempSync, readFileSync, statSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

class WorkerGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty');
  static project = mkdtempSync(path.join(tmpdir(), 'ty-service-worker-gate-'));
  static host = 'tachyon.test';
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json' };
}

class ServerState {
  static requests = 0;
  static apiRequests = 0;
  static apiPolicy = 'public, max-age=60';
  static apiVary = '';
  static replaceStylesheet = false;
}

function write(relative, contents) {
  const file = path.join(WorkerGate.project, relative);
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

function writeFixture() {
  write('client/pages/tac.html', '<offline-page />\n');
  write('client/components/offline/page/tac.html', '<main aria-label="Offline"><h1 id="title">{workerPage.title}</h1></main>\n');
  write('client/components/offline/page/tac.css', '#title { color: rgb(0, 128, 0) }\n');
  write('client/remotes/RuntimeClient.js', `@HTTPClient
class RuntimeClient {
  @GetRequest("/api/runtime")
  async read() {}
}`);
  write('client/remotes/ListedClient.js', `@HTTPClient
@CacheSettings({"name":"listed","ttlMs":60000,"maxBytes":65536,"maxEntries":64})
class ListedClient {
  @Cached
  @GetRequest("/api/listed")
  async list() {}

  @ClearCache({"method":"list"})
  @PostRequest("/api/listed")
  async save(@RequestBody product) {}
}`);
  write('client/services/WorkerPageService.js', `@Service
export default class WorkerPageService {
  @Needs
  runtimeClient;
  @Needs
  listedClient;
  title = 'Cached page';
}`);
  write('client/manifest.json', JSON.stringify({ tac: { cache: [{ path: '/api/cache-first', policy: 'cache-first' }, { path: '/api/never', policy: 'no-store' }, { path: '/api/*', policy: 'network-first' }] } }));
}

const rootDelegate = () => { globalThis.__root = (name) => __tachyonTac.service(name); };

function buildFixture() {
  const built = spawnSync(WorkerGate.binary, ['build', WorkerGate.project], { encoding: 'utf8' });
  if (built.status !== 0) {
    console.error(built.stderr || built.stdout);
    process.exit(1);
  }
}

function issueCertificate() {
  const key = path.join(WorkerGate.project, 'key.pem');
  const certificate = path.join(WorkerGate.project, 'cert.pem');
  const issued = spawnSync('openssl', [
    'req', '-x509', '-newkey', 'rsa:2048', '-nodes',
    '-keyout', key, '-out', certificate, '-days', '1',
    '-subj', `/CN=${WorkerGate.host}`, '-addext', `subjectAltName=DNS:${WorkerGate.host}`,
  ], { encoding: 'utf8' });
  if (issued.status !== 0) {
    console.error(`cannot issue a test certificate: ${issued.stderr}`);
    process.exit(1);
  }
  return { key, certificate };
}

function createWorkerServer(key, certificate) {
  const root = path.join(WorkerGate.project, 'dist');
  return createServer({ key: readFileSync(key), cert: readFileSync(certificate) }, (request, response) => {
    ServerState.requests += 1;
    if (request.url === '/.tachyon/components.css' && ServerState.replaceStylesheet) {
      response.writeHead(200, { 'content-type': 'text/css' });
      response.end('#title { color: red }');
      return;
    }
    if (request.url.startsWith('/api/') || request.url.startsWith('/account.html')) {
      ServerState.apiRequests += 1;
      const json = request.url === '/api/runtime' || request.url === '/api/listed';
      response.setHeader('content-type', json ? 'application/json' : 'text/plain');
      response.setHeader('cache-control', ServerState.apiPolicy);
      if (ServerState.apiVary) response.setHeader('vary', ServerState.apiVary);
      response.end(request.url.startsWith('/api/oversize') ? 'x'.repeat(300000) : String(ServerState.apiRequests));
      return;
    }
    let file = path.join(root, decodeURIComponent(request.url.split('?')[0]));
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': WorkerGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' });
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

async function openContext(origin, name = 'profile') {
  const profile = path.join(WorkerGate.project, name);
  const context = await chromium.launchPersistentContext(profile, {
    ignoreHTTPSErrors: true,
    args: [
      `--host-resolver-rules=MAP ${WorkerGate.host} 127.0.0.1`,
      '--ignore-certificate-errors',
    ],
  });
  await context.addInitScript(rootDelegate);
  context.setDefaultTimeout(15_000);
  context.setDefaultNavigationTimeout(15_000);
  return context;
}

async function checkRegistration(page, origin) {
  await page.goto(`${origin}/`, { waitUntil: 'networkidle' });

  const controller = await page.evaluate(async () => {
    const registration = await Promise.race([
      navigator.serviceWorker.ready,
      new Promise((_resolve, reject) => setTimeout(() => reject(new Error('worker never became ready')), 8000)),
    ]);
    await new Promise((resolve) => {
      if (navigator.serviceWorker.controller) return resolve();
      navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true });
      setTimeout(resolve, 5000);
    });
    return {
      scope: new URL(registration.scope).pathname,
      script: new URL(registration.active?.scriptURL ?? '').pathname,
      controlled: Boolean(navigator.serviceWorker.controller),
    };
  });
  expect(controller.script, '/tachyon-sw.js', 'worker served from the root');
  expect(controller.scope, '/', 'scope covers every page');
  expect(controller.controlled, true, 'page is controlled');

  await page.reload({ waitUntil: 'networkidle' });
  const cachedPaths = await page.evaluate(async () => {
    const names = await caches.keys();
    const cache = await caches.open(names.find((name) => name.startsWith('tachyon-static-')));
    return (await cache.keys()).map((request) => new URL(request.url).pathname).sort();
  });
  expect(cachedPaths.includes('/'), true, `document cached (${cachedPaths.join(' ')})`);
  expect(cachedPaths.some((entry) => entry.endsWith('.css')), true, 'stylesheet cached');
}

async function checkApiCaching(page, context, origin) {
  const read = (url, options = {}) => page.evaluate(async ({ url, options }) => {
    try { return await (await fetch(url, options)).text(); } catch { return 'network-failed'; }
  }, { url, options });
  const isCached = (url) => page.evaluate(async (url) => {
    const cache = await caches.open((await caches.keys()).find((name) => name.startsWith('tachyon-static-')));
    return Boolean(await cache.match(url));
  }, `${origin}${url}`);
  const publicRead = { credentials: 'omit' };
  await read('/api/cache-first', publicRead);
  expect(await isCached('/api/cache-first'), true, 'declared anonymous public read cached');
  const beforeHit = ServerState.apiRequests;
  await read('/api/cache-first', publicRead);
  expect(ServerState.apiRequests, beforeHit, 'declared cache-first answers without network');
  await read('/api/cache-first', { ...publicRead, cache: 'no-cache' });
  expect(ServerState.apiRequests, beforeHit + 1, 'a no-cache read asks the network on a cache-first path');
  expect(await isCached('/api/cache-first'), true, 'a no-cache read keeps the fresh public answer');
  await read('/api/cache-first');
  expect(ServerState.apiRequests, beforeHit + 2, 'credentialed cache-first read must use network');
  expect(await isCached('/api/cache-first'), false, 'credentialed read evicts matching shared cache');
  for (const options of [
    { credentials: 'include' },
    { credentials: 'omit', headers: { authorization: 'Bearer test-only' } },
    { credentials: 'omit', cache: 'no-store' },
  ]) {
    await read('/api/cache-first', publicRead);
    await read('/api/cache-first', options);
    expect(await isCached('/api/cache-first'), false, `private request not cached: ${JSON.stringify(options)}`);
  }
  for (const policy of ['private, max-age=60', 'no-store']) {
    ServerState.apiPolicy = 'public, max-age=60';
    await read('/api/privacy', publicRead);
    expect(await isCached('/api/privacy'), true, 'public response seeded before policy changes');
    ServerState.apiPolicy = policy;
    await read('/api/privacy', publicRead);
    expect(await isCached('/api/privacy'), false, `response ${policy} evicts stale public response`);
  }
  ServerState.apiPolicy = 'public, max-age=60';
  await read('/api/privacy', publicRead);
  ServerState.apiVary = 'accept-language';
  await read('/api/privacy', publicRead);
  expect(await isCached('/api/privacy'), false, 'Vary response evicts shared cache');
  ServerState.apiVary = '';
  await read('/api/never', publicRead);
  expect(await isCached('/api/never'), false, 'declared no-store not persisted');
  await read('/api/oversize', publicRead);
  expect(await isCached('/api/oversize'), false, 'oversized API response not persisted');
  await read('/account.html');
  expect(await isCached('/account.html'), false, 'arbitrary credentialed HTML is not a packaged asset');
  await page.evaluate(async () => {
    const cache = await caches.open((await caches.keys()).find((name) => name.startsWith('tachyon-static-')));
    await cache.delete('/.tachyon/components.css');
  });
  ServerState.replaceStylesheet = true;
  await read('/.tachyon/components.css');
  expect(await isCached('/.tachyon/components.css'), false, 'changed bytes at a packaged path fail fingerprint validation');
  ServerState.replaceStylesheet = false;
  await page.evaluate(async () => {
    const cache = await caches.open((await caches.keys()).find((name) => name.startsWith('tachyon-static-')));
    await cache.put('/.tachyon/components.css', new Response('#title { color: red }'));
  });
  expect((await read('/.tachyon/components.css')).includes('rgb(0, 128, 0)'), true, 'cache hits also validate packaged fingerprints (component stylesheet)');
  expect(await isCached('/.tachyon/components.css'), true, 'valid stylesheet recached');
  await page.evaluate(() => __root('workerPage').runtimeClient.read());
  expect(await isCached('/api/runtime'), false, 'worker cannot override the default Client no-store policy');
  await checkCachedClient(page, isCached);
  await read('/api/cache-first', publicRead);
  await context.setOffline(true);
  expect(await read('/api/cache-first', publicRead) !== 'network-failed', true, 'anonymous cached API works offline');
  expect(await read('/api/cache-first', { credentials: 'omit', headers: { authorization: 'Bearer test-only' } }), 'network-failed', 'authorized read cannot fall back to shared response');
  await context.setOffline(false);
}

async function checkCachedClient(page, isCached) {
  const listed = () => page.evaluate(() => __root('workerPage').listedClient.list());
  const before = ServerState.apiRequests;
  await listed();
  expect(await isCached('/api/listed'), true, 'worker keeps a Cached HTTPClient read');
  await listed();
  expect(ServerState.apiRequests, before + 1, 'Cached HTTPClient read answers from its cache');
  await page.evaluate(() => __root('workerPage').listedClient.save({ name: 'anvil' }));
  expect(await isCached('/api/listed'), false, 'a successful write evicts the worker entry');
  await listed();
  expect(ServerState.apiRequests, before + 3, 'ClearCache makes the next Cached read ask again');
}

async function checkOfflineDocument(page, context) {
  const before = ServerState.requests;
  await context.setOffline(true);
  await page.reload({ waitUntil: 'load' });
  expect(await page.textContent('#title'), 'Cached page', 'document served offline');
  expect(
    await page.evaluate(() => getComputedStyle(document.getElementById('title')).color),
    'rgb(0, 128, 0)',
    'stylesheet served offline',
  );
  await context.setOffline(false);
  console.log(`    note server saw ${ServerState.requests - before} request(s) while offline`);
}

async function checkDeployReload(page, origin) {
  const service = path.join(WorkerGate.project, 'client/services/WorkerPageService.js');
  writeFileSync(service, readFileSync(service, 'utf8').replace("'Cached page'", "'Deployed page'"));
  buildFixture();
  let loads = 0;
  const counted = () => { loads += 1; };
  page.on('load', counted);
  try {
    await page.goto(`${origin}/`, { waitUntil: 'load' });
    await page.waitForFunction(() => document.getElementById('title')?.textContent === 'Deployed page', undefined, { timeout: 10_000 }).catch(() => {});
    await page.waitForTimeout(1500);
    expect(loads, 2, 'a page the old worker served reloads once when the new worker takes control');
    expect(await page.textContent('#title'), 'Deployed page', 'the reloaded page runs the new build');
    await page.reload({ waitUntil: 'load' });
    await page.waitForTimeout(1500);
    expect(loads, 3, 'a page the new worker served does not reload again');
  } finally { page.off('load', counted); }
}

async function checkNoWorkerRegistration(context, origin, port) {
  const loopbackPage = await context.newPage();
  await loopbackPage.goto(`https://127.0.0.1:${port}/`, { waitUntil: 'networkidle' });
  await loopbackPage.waitForTimeout(600);
  expect(
    await loopbackPage.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length),
    0,
    'no worker registered on a loopback host',
  );
  await loopbackPage.close();
  const nativeContext = await context.browser().newContext({ ignoreHTTPSErrors: true });
  try {
    await nativeContext.addInitScript(() => {
      globalThis.__tachyonNativeHostCall = async () => JSON.stringify({ value: { fields: [], methods: [] } });
    });
    const nativePage = await nativeContext.newPage();
    await nativePage.goto(`${origin}/`, { waitUntil: 'networkidle' });
    expect(await nativePage.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length),
      0, 'hosted native page does not register a service worker');
  } finally { await nativeContext.close(); }
}

class AppsDeploy {
  static site = path.join(WorkerGate.project, 'apps-site');
  static shell = path.join(AppsDeploy.site, 'home');
  static chat = path.join(AppsDeploy.site, 'chat');
  static served = path.join(AppsDeploy.shell, 'dist', 'web');
  static table = path.join(AppsDeploy.served, 'chat', '.tachyon', 'app.json');
}

function writeAppsSite(chatTitle) {
  const files = {
    'home/client/manifest.json': JSON.stringify({ name: 'Home', tac: { apps: { '/chat': '../chat' } } }),
    'home/client/pages/tac.html': '<home-page />\n',
    'home/client/components/home/page/tac.html': '<main><h1 id="home-title">Home</h1><a id="to-chat" href="/chat/">Chat</a></main>\n',
    'chat/client/manifest.json': JSON.stringify({ name: 'Chat', tac: { mount: '/chat' } }),
    'chat/client/pages/tac.html': '<chat-page />\n',
    'chat/client/components/chat/page/tac.html': `<main><h1 id="chat-title">${chatTitle}</h1></main>\n`,
  };
  for (const [relative, contents] of Object.entries(files)) {
    const file = path.join(AppsDeploy.site, relative);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, contents);
  }
}

function bundle(args) {
  const built = spawnSync(WorkerGate.binary, ['bundle', ...args], { encoding: 'utf8' });
  if (built.status !== 0) {
    console.error(built.stderr || built.stdout);
    process.exit(1);
  }
}

function deployChat(chatTitle) {
  writeAppsSite(chatTitle);
  bundle([AppsDeploy.chat]);
  rmSync(path.join(AppsDeploy.served, 'chat'), { recursive: true, force: true });
  cpSync(path.join(AppsDeploy.chat, 'dist', 'web', 'chat'), path.join(AppsDeploy.served, 'chat'), { recursive: true });
  return JSON.parse(readFileSync(AppsDeploy.table, 'utf8')).version;
}

function createSiteServer(key, certificate, root) {
  return createServer({ key: readFileSync(key), cert: readFileSync(certificate) }, (request, response) => {
    let file = path.join(root, decodeURIComponent(request.url.split('?')[0]));
    if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html');
    if (!existsSync(file)) { response.writeHead(404).end(); return; }
    response.writeHead(200, { 'content-type': WorkerGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' });
    createReadStream(file).pipe(response);
  });
}

async function cacheNames(page) {
  return page.evaluate(async () => (await caches.keys()).sort());
}

async function cachedPaths(page, name) {
  return page.evaluate(async (name) => {
    const cache = await caches.open(name);
    return (await cache.keys()).map((request) => new URL(request.url).pathname).sort();
  }, name);
}

async function waitForController(page) {
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    if (navigator.serviceWorker.controller) return;
    await new Promise((resolve) => {
      navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true });
      setTimeout(resolve, 5000);
    });
  });
}

async function checkPageApps(key, certificate) {
  writeAppsSite('Chat one');
  bundle([AppsDeploy.shell, '--shell-only']);
  const first = deployChat('Chat one');
  const server = createSiteServer(key, certificate, AppsDeploy.served);
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const origin = `https://${WorkerGate.host}:${server.address().port}`;
  const context = await openContext(origin, 'apps-profile');
  try {
    const page = await context.newPage();
    await page.goto(`${origin}/`, { waitUntil: 'networkidle' });
    await waitForController(page);
    await page.goto(`${origin}/chat/`, { waitUntil: 'networkidle' });
    await page.reload({ waitUntil: 'networkidle' });
    const firstCache = `tachyon-static-/chat@${first}`;
    const names = await cacheNames(page);
    expect(names.includes(firstCache), true, `the chat app has its own cache (${names.join(' ')})`);
    const shellCache = names.find((name) => /^tachyon-static-[0-9a-f]+$/.test(name));
    expect(Boolean(shellCache), true, 'the shell keeps its own cache');
    const chatPaths = await cachedPaths(page, firstCache);
    expect(chatPaths.includes('/chat/.tachyon/tac-client.js'), true, `the chat runtime is kept in the chat cache (${chatPaths.join(' ')})`);
    await context.setOffline(true);
    await page.reload({ waitUntil: 'load' });
    await page.waitForFunction(() => document.getElementById('chat-title')?.textContent === 'Chat one', undefined, { timeout: 10_000 }).catch(() => {});
    expect(await page.textContent('#chat-title'), 'Chat one', 'the separately deployed chat page works offline');
    await context.setOffline(false);
    const worker = await page.evaluate(() => navigator.serviceWorker.controller?.scriptURL ?? '');
    const second = deployChat('Chat two');
    let loads = 0;
    const counted = () => { loads += 1; };
    page.on('load', counted);
    try {
      await page.goto(`${origin}/chat/`, { waitUntil: 'networkidle' });
      await page.waitForFunction(() => document.getElementById('chat-title')?.textContent === 'Chat two', undefined, { timeout: 10_000 }).catch(() => {});
      await page.waitForTimeout(1500);
      expect(await page.textContent('#chat-title'), 'Chat two', 'a new chat deploy shows at the next visit');
      expect(loads, 1, 'a new chat deploy does not reload the page through the worker');
    } finally { page.off('load', counted); }
    const after = await cacheNames(page);
    expect(after.includes(`tachyon-static-/chat@${second}`), true, `the new chat deploy gets a new cache (${after.join(' ')})`);
    expect(after.includes(firstCache), false, 'the older chat cache is deleted');
    expect(after.includes(shellCache), true, 'the shell cache stays when only chat changes');
    expect(await page.evaluate(() => navigator.serviceWorker.controller?.scriptURL ?? ''), worker, 'the shell worker stays the same');
  } finally {
    await context.close();
    await new Promise((resolve) => server.close(resolve));
  }
}

async function main() {
  writeFixture();
  buildFixture();
  const { key, certificate } = issueCertificate();
  const server = createWorkerServer(key, certificate);

  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address();
  const origin = `https://${WorkerGate.host}:${port}`;
  const context = await openContext(origin);
  const page = await context.newPage();
  page.on('console', (message) => {
    if (message.type() === 'error') console.log(`    note console: ${message.text()}`);
  });

  try {
    await checkRegistration(page, origin);
    await checkApiCaching(page, context, origin);
    await checkOfflineDocument(page, context);
    await checkDeployReload(page, origin);
    await checkNoWorkerRegistration(context, origin, port);
    await checkPageApps(key, certificate);
  } finally {
    await context.close();
    await new Promise((resolve) => server.close(resolve));
    rmSync(WorkerGate.project, { recursive: true, force: true });
  }

  if (!process.exitCode) console.log('\nofflinecache gate passed');
}

await main();

#!/usr/bin/env node

import { CargoArtifacts } from '../cargo-artifacts.mjs'
import assert from 'node:assert/strict';
import { access, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { withServer } from '../release/server-probe.mjs';

class StandaloneTest {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug', process.platform === 'win32' ? 'ty.exe' : 'ty');
  static workspace = null;
  static cache = null;
  static environment = null;
}

async function exists(file) {
  try { await access(file); return true; } catch { return false; }
}

async function run(args, options = {}) {
  const child = spawn(StandaloneTest.binary, args, {
    cwd: options.cwd ?? StandaloneTest.workspace,
    env: options.env ?? StandaloneTest.environment,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', (chunk) => { stdout += chunk; });
  child.stderr.on('data', (chunk) => { stderr += chunk; });
  const code = await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('close', resolve);
  });
  if (code !== 0) throw new Error(`ty ${args.join(' ')} failed (${code})\n${stdout}\n${stderr}`);
  return { stdout, stderr };
}

async function stop(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const closed = new Promise((resolve) => child.once('close', resolve));
  child.kill('SIGTERM');
  await closed;
}

async function waitForHttp(url, deadline = Date.now() + 15_000) {
  let last;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return response;
      last = new Error(`HTTP ${response.status}`);
    } catch (error) { last = error; }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw last ?? new Error(`Timed out waiting for ${url}`);
}

async function checkScaffold() {
  assert.equal((await run(['--version'])).stdout.trim().length > 0, true);
  await run(['init', 'smoke']);
  const project = path.join(StandaloneTest.workspace, 'smoke');
  const expectedScaffold = [
    '.env.example', '.gitignore', 'server/application.json', 'client/manifest.json', 'README.md', 'client/jsconfig.json', 'server/jsconfig.json', 'package.json',
    'tachyon-env.d.ts', 'tachyon-behaviors.d.ts', 'client/pages/tac.html',
    'client/components/app/shell/tac.html', 'client/services/ShellService.js', 'client/components/app/shell/tac.css',
    'client/components/app/hero/tac.html', 'client/components/app/hero/tac.css',
    'client/shared/assets/.gitkeep', 'client/shared/data/.gitkeep',
    'client/shared/styles/app.css',
    'server/controllers/HomeController.js', 'server/data/.gitkeep', 'server/deps/.gitkeep',
    '.tachyon/types/client/composed.d.ts', '.tachyon/types/server/composed.d.ts',
  ];
  for (const file of expectedScaffold) assert.equal(await exists(path.join(project, file)), true, file);
  assert.equal(await exists(path.join(project, 'db')), false, 'removed database scaffold');
  assert.equal(await exists(path.join(project, '.env.test')), false, 'only root .env is supported');
  assert.equal(
    await readFile(path.join(project, '.env.example'), 'utf8'),
    '# Copy to .env and add only server secrets. Never commit .env.\n# YON_DATASOURCE_PASSWORD=replace-me\n',
    '.env.example contains only a commented secret placeholder',
  );
  assert.deepEqual(JSON.parse(await readFile(path.join(project, 'server/application.json'), 'utf8')), {
    server: { host: '127.0.0.1', port: 8080 },
  });
  return project;
}

async function checkWebBundle(project) {
  await run(['bundle', '--target', 'web'], { cwd: project });
  assert.equal(await exists(path.join(project, 'dist', 'web', 'index.html')), true);
  const spa = await readFile(path.join(project, 'dist', 'web', 'spa-renderer.js'), 'utf8');
  assert.match(spa, /window\.__tc_rerender/);
  assert.doesNotMatch(spa, /__TACHYON_ASSET_PREFIX__|__tachyonPlaceholder|__tachyonShellPlaceholder/);
  assert.equal((await readdir(path.join(StandaloneTest.cache, 'runtime'), { withFileTypes: true })).some((entry) => entry.isDirectory()), true);
  await run(['cache', 'clean'], { cwd: project });
  assert.equal(await exists(path.join(StandaloneTest.cache, 'runtime')), false);
}

async function checkServer() {
  const response = await waitForHttp('http://127.0.0.1:18777/');
  assert.deepEqual(await response.json(), { ok: true, framework: 'Tachyon' });
  const runtime = await waitForHttp('http://127.0.0.1:18777/spa-renderer.js');
  assert.match(runtime.headers.get('content-type') ?? '', /javascript/);
}

async function writeLifecycleApp(project) {
  await writeFile(path.join(project, 'client/pages/tac.html'), '<app-lifecycle />\n');
  await mkdir(path.join(project, 'client/components/app/lifecycle'), { recursive: true });
  await writeFile(path.join(project, 'client/components/app/lifecycle/tac.html'), `
<main>
  <button on:click="appLifecycle.count += 1">Add</button><p>Count: {appLifecycle.count}</p>
  <p>Required: {appLifecycle.required.join('|')}</p><p>Events: {appLifecycle.events.join('|')}</p>
</main>
`);
  await writeFile(path.join(project, 'client/services/AppLifecycleService.js'), `
@Service()
export default class AppLifecycleService {
  count = 0
  required = ['service.state', 'mount.lifecycle']
  events = []
  @OnStart()
  initialize() { this.events = ['mounted'] }
}
`);
}

async function checkPreviewPage(page) {
  await page.goto('http://127.0.0.1:18778/');
  await page.getByText('Count: 0', { exact: true }).waitFor();
  await page.getByRole('button', { name: 'Add' }).click();
  await assert.doesNotReject(() => page.getByText('Count: 1', { exact: true }).waitFor());
  await page.getByText('Required: service.state|mount.lifecycle', { exact: true }).waitFor();
  await page.getByText('Events: mounted', { exact: true }).waitFor();
}

async function checkNativeBundle(project, page) {
  const nativeTarget = process.platform === 'darwin' ? 'macos' : process.platform === 'win32' ? 'windows' : 'linux';
  await run(['bundle', '--native', '--target', nativeTarget], { cwd: project });
  const nativeRoot = path.join(project, 'dist', nativeTarget);
  assert.equal(await exists(path.join(nativeRoot, 'tachyon.host.json')), true);
  const hostManifest = JSON.parse(await readFile(path.join(nativeRoot, 'tachyon.host.json'), 'utf8'));
  assert.equal(hostManifest.schemaVersion, 3);
  assert.equal(hostManifest.renderMode, 'bundle');
  assert.equal(hostManifest.target, nativeTarget);
  assert.equal(hostManifest.entryRoute, '/');
  assert.deepEqual(hostManifest.hostCapabilities, ['companion.invoke']);

  await withServer(StandaloneTest.binary, ['preview', project, '--static', '--out-dir', path.join(nativeRoot, 'web'), '--port', '0'], async request => {
    const response = await request('/');
    assert.equal(response.status, 200);
    await page.goto(response.url);
    await page.getByText('Count: 0', { exact: true }).waitFor();
    await page.getByRole('button', { name: 'Add' }).click();
    await page.getByText('Count: 1', { exact: true }).waitFor();
    await page.getByText('Required: service.state|mount.lifecycle', { exact: true }).waitFor();
    await page.getByText('Events: mounted', { exact: true }).waitFor();
  });
  return nativeTarget;
}

function checkNativeAbi() {
  const nativeAbi = spawnSync(process.execPath, [path.join(StandaloneTest.repo, 'scripts/native/companion-test.mjs')], {
    cwd: StandaloneTest.repo, env: { ...StandaloneTest.environment, TAC_BIN: StandaloneTest.binary }, encoding: 'utf8',
    timeout: 300_000, maxBuffer: 2 * 1024 * 1024,
  });
  assert.equal(nativeAbi.status, 0, `Native companion ABI failed: ${nativeAbi.error || nativeAbi.stderr || nativeAbi.stdout}`);
  process.stdout.write(nativeAbi.stdout);
}

async function main() {
  StandaloneTest.workspace = await mkdtemp(path.join(tmpdir(), 'tachyon-rust-standalone-'));
  StandaloneTest.cache = path.join(StandaloneTest.workspace, 'cache');
  StandaloneTest.environment = { ...process.env, TAC_CACHE_DIR: StandaloneTest.cache };
  let server;
  let preview;
  let browser;
  try {
    const project = await checkScaffold();
    await checkWebBundle(project);

    server = spawn(StandaloneTest.binary, ['serve', '--port', '18777'], {
      cwd: project,
      env: StandaloneTest.environment,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    await checkServer();

    await writeLifecycleApp(project);
    await run(['bundle', '--target', 'web'], { cwd: project });
    preview = spawn(StandaloneTest.binary, ['preview', '--static', '--port', '18778'], {
      cwd: project,
      env: StandaloneTest.environment,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    await waitForHttp('http://127.0.0.1:18778/');
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    await checkPreviewPage(page);
    await stop(preview);
    preview = undefined;

    const nativeTarget = await checkNativeBundle(project, page);
    checkNativeAbi();
    process.stdout.write(`PASS: standalone scaffold, server, web/native-bundle rendering and native ABI (${nativeTarget})\n`);
  } finally {
    if (browser) await browser.close();
    await stop(preview);
    await stop(server);
    await rm(StandaloneTest.workspace, { recursive: true, force: true });
  }
}

await main();

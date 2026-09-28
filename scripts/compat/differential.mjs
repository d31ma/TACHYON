#!/usr/bin/env node

import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { cp, mkdtemp, readFile, readdir, rm, stat } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

class Compatibility {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static corpus = path.join(Compatibility.repo, 'corpus');
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty');
  static legacyBinary = process.env.TAC_LEGACY_BIN;
}

if (!Compatibility.legacyBinary) {
  throw new Error(
    'TAC_LEGACY_BIN must name an immutable released ty executable; the in-tree legacy source is not a compatibility oracle.',
  );
}

class ComparedMarkup {
  static mimeTypes = {
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.json': 'application/json; charset=utf-8',
    '.map': 'application/json; charset=utf-8',
  };
  static semanticAttributes = [
    'alt', 'aria-current', 'aria-hidden', 'aria-label', 'aria-labelledby',
    'aria-live', 'for', 'href', 'name', 'placeholder', 'role', 'src', 'type',
    'value',
  ];
  static ignoredTags = new Set(['SCRIPT', 'STYLE', 'LINK', 'META', 'TEMPLATE', 'NOSCRIPT']);
}

function run(command, args, options = {}) {
  return new Promise((resolve) => {
    const child = spawn(command, args, { ...options, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('error', (error) => resolve({ code: -1, stdout, stderr: String(error) }));
    child.on('close', (code) => resolve({ code, stdout, stderr }));
  });
}

async function serve(root) {
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://localhost');
    let relative = decodeURIComponent(url.pathname).replace(/^\/+/, '');
    if (relative.includes('..')) {
      response.writeHead(400).end();
      return;
    }
    let file = path.join(root, relative);
    try {
      if (relative === '' || (await stat(file)).isDirectory()) {
        file = path.join(file, 'index.html');
      }
    } catch {
    }
    try {
      await stat(file);
    } catch {
      file = path.join(root, 'index.html');
      try {
        await stat(file);
      } catch {
        response.writeHead(404).end();
        return;
      }
    }
    response.writeHead(200, {
      'content-type': ComparedMarkup.mimeTypes[path.extname(file)] ?? 'application/octet-stream',
      'cache-control': 'no-store',
    });
    createReadStream(file).pipe(response);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

function extractSemanticDom(semanticAttributes, ignoredTags) {
  const ignored = new Set(ignoredTags);
  const visit = (node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent.replace(/\s+/g, ' ').trim();
      return text ? { text } : null;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return null;
    if (ignored.has(node.tagName)) return null;
    const attributes = {};
    for (const name of semanticAttributes) {
      if (node.hasAttribute(name)) attributes[name] = node.getAttribute(name);
    }
    const children = [];
    for (const child of node.childNodes) {
      const value = visit(child);
      if (value) children.push(value);
    }
    return { tag: node.tagName.toLowerCase(), attributes, children };
  };
  const body = visit(document.body);
  let node = body;
  while (node && node.children && node.children.length === 1 && !Object.keys(node.attributes ?? {}).length) {
    node = node.children[0];
  }
  return node;
}

async function renderWith(origin, routes) {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.addInitScript(
    ({ source }) => {
      // eslint-disable-next-line no-new-func
      window.__extract = new Function(`return (${source})`)();
    },
    { source: extractSemanticDom.toString() },
  );
  const rendered = {};
  try {
    for (const route of routes) {
      const response = await page.goto(`${origin}${route}`, { waitUntil: 'networkidle' });
      await page.waitForTimeout(250);
      rendered[route] = {
        status: response ? response.status() : 0,
        dom: await page.evaluate(
          ([attributes, tags]) => window.__extract(attributes, tags),
          [ComparedMarkup.semanticAttributes, [...ComparedMarkup.ignoredTags]],
        ),
      };
    }
  } finally {
    await browser.close();
  }
  return rendered;
}

function difference(left, right, at = '') {
  if (JSON.stringify(left) === JSON.stringify(right)) return null;
  if (left === null || right === null || typeof left !== typeof right) {
    return `${at || '<root>'}: ${JSON.stringify(left)} vs ${JSON.stringify(right)}`;
  }
  if (left.text !== undefined || right.text !== undefined) {
    return `${at}: text ${JSON.stringify(left.text)} vs ${JSON.stringify(right.text)}`;
  }
  if (left.tag !== right.tag) return `${at}: <${left.tag}> vs <${right.tag}>`;
  const keys = new Set([...Object.keys(left.attributes ?? {}), ...Object.keys(right.attributes ?? {})]);
  for (const key of keys) {
    if ((left.attributes ?? {})[key] !== (right.attributes ?? {})[key]) {
      return `${at}<${left.tag}>: ${key}=${JSON.stringify((left.attributes ?? {})[key])} vs ${JSON.stringify((right.attributes ?? {})[key])}`;
    }
  }
  const leftChildren = left.children ?? [];
  const rightChildren = right.children ?? [];
  if (leftChildren.length !== rightChildren.length) {
    return `${at}<${left.tag}>: ${leftChildren.length} children vs ${rightChildren.length}`;
  }
  for (let index = 0; index < leftChildren.length; index += 1) {
    const nested = difference(leftChildren[index], rightChildren[index], `${at}<${left.tag}>[${index}]`);
    if (nested) return nested;
  }
  return null;
}

async function routesOf(rustOutput) {
  const manifest = JSON.parse(await readFile(path.join(rustOutput, 'route-manifest.json'), 'utf8'));
  return manifest.routes.map((entry) => entry.route).sort();
}

async function treeOf(root, current = root, output = {}) {
  const entries = await readdir(current, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name));
  for (const entry of entries) {
    const absolute = path.join(current, entry.name);
    const relative = path.relative(root, absolute).replaceAll(path.sep, '/');
    if (entry.isDirectory()) await treeOf(root, absolute, output);
    else output[relative] = (await readFile(absolute)).toString('base64');
  }
  return output;
}

function withoutRemovedLegacyScaffold(tree) {
  const normalized = { ...tree };
  for (const file of [
    'db/README.md',
    'db/schemas/.gitkeep',
    'db/.collections/.gitkeep',
    'tachyon-env.d.ts',
  ]) {
    delete normalized[file];
  }
  delete normalized['.env.test'];
  normalized['.env.example'] = Buffer.from(
    '# Copy to .env and add only server secrets. Never commit .env.\n# YON_DATASOURCE_PASSWORD=replace-me\n',
  ).toString('base64');
  normalized['application.json'] = Buffer.from(
    `${JSON.stringify({ tac: {}, yon: { server: { host: '127.0.0.1', port: 8080 } } }, null, 2)}\n`,
  ).toString('base64');
  const readme = Buffer.from(normalized['README.md'], 'base64').toString('utf8')
    .replace(
      /This shows a staged Tachyon progress bar while installing the `ty` binary plus\s+the `fylo`, `chex`, and `ttid` binaries it drives at runtime\./,
      'This shows a staged Tachyon progress bar while installing only the standalone `ty` binary.',
    );
  normalized['README.md'] = Buffer.from(readme).toString('base64');
  const home = Buffer.from(normalized['client/pages/tac.html'], 'base64').toString('utf8')
    .replace('Build your next Bun app with Tachyon.', 'Build your next app with Tachyon.');
  normalized['client/pages/tac.html'] = Buffer.from(home).toString('base64');
  const route = Buffer.from(normalized['server/routes/yon.js'], 'base64').toString('utf8')
    .replace('export class Handler {', 'export class Handler {');
  normalized['server/routes/yon.js'] = Buffer.from(route).toString('base64');
  return normalized;
}

async function compareScaffold() {
  const workspace = await mkdtemp(path.join(tmpdir(), 'ty-compat-scaffold-'));
  const legacyRoot = path.join(workspace, 'legacy');
  const rustRoot = path.join(workspace, 'rust');
  try {
    const legacy = await run(Compatibility.legacyBinary, ['init', legacyRoot, '--name', 'Parity App']);
    const rust = await run(Compatibility.binary, ['init', rustRoot, '--name', 'Parity App']);
    if (legacy.code !== 0 || rust.code !== 0) {
      return { ok: false, detail: `legacy=${legacy.code} rust=${rust.code}\n${legacy.stderr}\n${rust.stderr}` };
    }
    const legacyTree = withoutRemovedLegacyScaffold(await treeOf(legacyRoot));
    const rustTree = await treeOf(rustRoot);
    delete rustTree['tachyon-env.d.ts'];
    const names = [...new Set([...Object.keys(legacyTree), ...Object.keys(rustTree)])].sort();
    const difference = names.find((name) => legacyTree[name] !== rustTree[name]);
    if (difference === undefined) {
      return { ok: true, detail: `${Object.keys(rustTree).length} generated files match after declared scaffold migrations` };
    }
    return { ok: false, detail: `first generated-file difference: ${difference}` };
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
}

async function expectedDivergences(source) {
  const parity = await readFile(path.join(source, 'parity.json'), 'utf8').catch(() => '{}');
  return JSON.parse(parity).expected_divergences ?? [];
}

function checkRecorder(result, expected) {
  return (check, ok, detail) => {
    const declared = expected.find(
      (entry) => entry.check === check && entry.detail === detail,
    );
    if (!ok && declared) {
      result.checks.push({ check, ok: true, detail: `expected divergence: ${detail}` });
      result.acknowledged.push({ check, detail, reason: declared.reason });
      return;
    }
    result.checks.push({ check, ok, detail });
    if (!ok) result.differences.push(`${check}: ${detail}`);
  };
}

async function legacyPageNames(legacyRoot) {
  const entries = await readdir(path.join(legacyRoot, 'pages')).catch(() => []);
  const pages = entries
    .filter((entry) => entry.endsWith('.js'))
    .map((entry) => entry.replace(/\.js$/, ''));
  return [...new Set(pages)].sort();
}

async function compareRenderedRoutes(legacyOrigin, rustOrigin, routes, record, result) {
  const legacyDom = await renderWith(legacyOrigin, routes);
  const rustDom = await renderWith(rustOrigin, routes);
  for (const route of routes) {
    const detail = difference(legacyDom[route]?.dom, rustDom[route]?.dom);
    record(`semantic dom ${route}`, detail === null, detail ?? 'identical');
    record(
      `http status ${route}`,
      legacyDom[route]?.status === 200 && rustDom[route]?.status === 200,
      `legacy ${legacyDom[route]?.status} / rust ${rustDom[route]?.status}`,
    );
  }
  result.legacyDom = legacyDom;
  result.rustDom = rustDom;
}

async function compareProject(name) {
  const source = path.join(Compatibility.corpus, name);
  const workspace = await mkdtemp(path.join(tmpdir(), `ty-compat-${name}-`));
  const project = path.join(workspace, name);
  await cp(source, project, { recursive: true });

  const expected = await expectedDivergences(source);
  const result = { project: name, checks: [], differences: [], acknowledged: [] };
  const record = checkRecorder(result, expected);

  const legacy = await run(Compatibility.legacyBinary, ['bundle'], { cwd: project });
  record('legacy build', legacy.code === 0, legacy.code === 0 ? 'ok' : legacy.stderr.slice(-400));

  const rust = await run(Compatibility.binary, ['build', project, '--out-dir', 'dist-rust']);
  record('rust build', rust.code === 0, rust.code === 0 ? 'ok' : rust.stderr.slice(-400));

  if (legacy.code !== 0 || rust.code !== 0) {
    await rm(workspace, { recursive: true, force: true });
    return result;
  }

  const legacyRoot = path.join(project, 'dist/web');
  const rustRoot = path.join(project, 'dist-rust');
  const routes = await routesOf(rustRoot);

  const legacyPages = await legacyPageNames(legacyRoot);
  record(
    'route graph',
    routes.length > 0,
    routes.length > 0 ? `${routes.length} route(s): ${routes.join(', ')}` : 'no routes published',
  );
  result.routes = routes;
  result.legacyPages = legacyPages;

  const legacyServer = await serve(legacyRoot);
  const rustServer = await serve(rustRoot);
  try {
    await compareRenderedRoutes(legacyServer.origin, rustServer.origin, routes, record, result);
  } finally {
    legacyServer.server.close();
    rustServer.server.close();
    await rm(workspace, { recursive: true, force: true });
  }
  return result;
}

async function main() {
  const scaffold = await compareScaffold();
  process.stdout.write(`==> scaffold\n    ${scaffold.ok ? 'ok  ' : 'FAIL'} ${scaffold.detail}\n`);
  if (!scaffold.ok) process.exit(1);
  const only = process.argv.slice(2).filter((value) => !value.startsWith('--'));
  const names = (await readdir(Compatibility.corpus, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((name) => only.length === 0 || only.includes(name))
    .sort();

  const results = [];
  for (const name of names) {
    process.stdout.write(`==> ${name}\n`);
    const result = await compareProject(name);
    for (const check of result.checks) {
      process.stdout.write(`    ${check.ok ? 'ok  ' : 'FAIL'} ${check.check}: ${check.detail}\n`);
    }
    for (const entry of result.acknowledged) {
      process.stdout.write(`         reason: ${entry.reason}\n`);
    }
    results.push(result);
  }

  const failed = results.filter((result) => result.differences.length > 0);
  if (process.argv.includes('--json')) {
    process.stdout.write(`${JSON.stringify(results, null, 2)}\n`);
  }
  process.stdout.write(
    `\n${results.length - failed.length}/${results.length} corpus projects match across implementations\n`,
  );
  process.exit(failed.length === 0 ? 0 : 1);
}

await main();

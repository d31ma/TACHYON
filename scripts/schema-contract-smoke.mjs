import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { withServer } from './release/server-probe.mjs';

class SchemaProject {
  static model = `/**
 * @typedef {Object} Item
 * @property {string} name
 * @property {number} [count]
 */
export {}
`;

  static controller = `@RequestRoute("/items")
@Public
class ItemsController {
  /**
   * @param {number} id
   * @param {Item} item
   * @param {string} authorization
   * @returns {Item}
   */
  @PostRoute("/{id}")
  @ResponseStatus(201)
  postItem(@FromPath id, @FromBody item, @FromHeader("authorization") authorization) {
    return { name: item.name, count: id }
  }

  /**
   * @param {number} id
   * @returns {Item}
   */
  @GetRoute("/{id}")
  getItem(@FromPath id) {
    return { count: id }
  }
}
`;
}

/** @param {string} root */
async function writeSchemaProject(root) {
  await mkdir(join(root, 'client/pages'), { recursive: true });
  await mkdir(join(root, 'client/components/schema/page'), { recursive: true });
  await mkdir(join(root, 'server/controllers'), { recursive: true });
  await mkdir(join(root, 'server/models'), { recursive: true });
  await writeFile(join(root, 'client/pages/tac.html'), '<schema-page />');
  await writeFile(join(root, 'client/components/schema/page/tac.html'), '<main>Schema acceptance</main>');
  await writeFile(join(root, 'server/models/Item.js'), SchemaProject.model);
  await writeFile(join(root, 'server/controllers/ItemsController.js'), SchemaProject.controller);
}

/**
 * @param {string} binary
 * @param {string} root
 */
async function checkSchemaValidation(binary, root) {
  await withServer(binary, ['preview', root, '--host', '127.0.0.1', '--port', '0'], async request => {
    /**
     * @param {string} path
     * @param {string} body
     * @param {string} [authorization]
     */
    const send = (path, body, authorization = 'Bearer fixture-only') => request(path, {
      method: 'POST', headers: { authorization, 'content-type': 'application/json' }, body,
    });
    assert.equal((await send('/items/7', '{"name":"Ada"}')).status, 201);
    assert.equal((await send('/items/7', '{"count":3}')).status, 400);
    assert.equal((await send('/items/7', '{"name":7}')).status, 400);
    assert.equal((await send('/items/nope', '{"name":"Ada"}')).status, 400);
    assert.equal((await send('/items/7', 'not json')).status, 400);
    assert.equal((await request('/items/7', { method: 'GET' })).status, 500);
    assert.equal((await request('/items/7', { method: 'DELETE' })).status, 405);
    const options = await request('/items/7', { method: 'OPTIONS' });
    assert.equal(options.status, 200);
    const schema = await options.json();
    assert.ok(schema.methods.POST.request.body);
    assert.ok(schema.models.Item);
    const reference = await request('/api.json');
    assert.equal(reference.status, 200);
    assert.ok((await reference.json()).models.Item);
    assert.equal((await request('/api-docs/')).status, 200);
  });
}

async function main() {
  process.env.TAC_ALLOW_UNTESTED = '1';
  const binary = process.argv[2] ?? process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty');
  const root = await mkdtemp(join(tmpdir(), 'tachyon-schema-'));
  try {
    await writeSchemaProject(root);
    await checkSchemaValidation(binary, root);
    console.log('PASS: model-based body/header/path validation, answer checks, OPTIONS schema, API reference');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

await main();

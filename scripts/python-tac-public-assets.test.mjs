import assert from 'node:assert/strict'
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { test } from 'node:test'
import { PythonTacPublicAssets } from './python-tac-public-assets.mjs'

class PublicAssetResponse {
  writeHead(status, headers) {
    this.status = status
    this.headers = headers
    return this
  }

  end(bytes) {
    this.bytes = bytes
    return this
  }
}

test('public static fixture retains exact compiler bytes and MIME without preview policy overrides', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'python-public-assets-'))
  try {
    await mkdir(path.join(root, '.tachyon'))
    await writeFile(path.join(root, 'index.html'), '<main>Exact compiler bytes</main>')
    const bytes = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0])
    await writeFile(path.join(root, '.tachyon/runtime.wasm'), bytes)
    const assets = await PythonTacPublicAssets.read(root)
    await writeFile(path.join(root, '.tachyon/runtime.wasm'), 'changed after capture')
    const response = new PublicAssetResponse()
    assets.respond({ method: 'GET', url: '/.tachyon/runtime.wasm?version=1' }, response)
    assert.equal(response.status, 200)
    assert.deepEqual(response.bytes, bytes)
    assert.equal(response.headers['content-type'], 'application/wasm')
    assert.equal(response.headers['cache-control'], 'public, max-age=60')
    assert.equal(response.headers.vary, undefined)
    assert.equal(response.headers['content-encoding'], undefined)
    const document = new PublicAssetResponse()
    assets.respond({ method: 'GET', url: '/' }, document)
    assert.equal(document.bytes.toString(), '<main>Exact compiler bytes</main>')
    assert.equal(document.headers['content-type'], 'text/html; charset=utf-8')
    const head = new PublicAssetResponse()
    assets.respond({ method: 'HEAD', url: '/' }, head)
    assert.equal(head.status, 200)
    assert.equal(head.bytes, undefined)
    assert.equal(head.headers['content-length'], Buffer.byteLength('<main>Exact compiler bytes</main>'))
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('public static fixture refuses traversal, malformed paths and non-read methods', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'python-public-assets-'))
  try {
    await writeFile(path.join(root, 'index.html'), 'bounded')
    const assets = await PythonTacPublicAssets.read(root)
    for (const url of ['/../private', '/%2e%2e/private', '/a/./b', '/%5cprivate', '/%00', '/%XX', 'https://outside.test/']) {
      const response = new PublicAssetResponse()
      assets.respond({ method: 'GET', url }, response)
      assert.equal(response.status, 400, url)
      assert.equal(response.bytes, undefined)
    }
    const missing = new PublicAssetResponse()
    assets.respond({ method: 'GET', url: '/missing' }, missing)
    assert.equal(missing.status, 404)
    const write = new PublicAssetResponse()
    assets.respond({ method: 'POST', url: '/' }, write)
    assert.equal(write.status, 405)
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test('public static capture refuses both linked files and linked roots', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'python-public-assets-'))
  try {
    const dist = path.join(root, 'dist')
    await mkdir(dist)
    await writeFile(path.join(root, 'private.txt'), 'must not enter captured assets')
    await symlink('../private.txt', path.join(dist, 'linked.txt'))
    await assert.rejects(PythonTacPublicAssets.read(dist), /refuses symlinks/)
    await symlink(dist, path.join(root, 'linked-root'))
    await assert.rejects(PythonTacPublicAssets.read(path.join(root, 'linked-root')), /non-symlinked directories/)
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

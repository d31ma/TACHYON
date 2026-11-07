import { afterEach, describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { SnippetCache } from '../scripts/snippet-cache.mjs'
import { ToolchainFingerprint } from '../scripts/toolchain-fingerprint.mjs'

class CacheFixture {
  /** @type {string[]} */
  static folders = []

  /** @returns {string} */
  static folder() {
    const folder = mkdtempSync(path.join(tmpdir(), 'snippet-cache-'))
    CacheFixture.folders.push(folder)
    return folder
  }

  /**
   * @param {Record<string, string>} files
   * @returns {string}
   */
  static stagedProject(files) {
    const root = CacheFixture.folder()
    for (const [name, code] of Object.entries(files)) {
      mkdirSync(path.dirname(path.join(root, name)), { recursive: true })
      writeFileSync(path.join(root, name), code)
    }
    return root
  }

  /**
   * @param {boolean} reads
   * @param {string} fingerprint
   * @returns {SnippetCache}
   */
  static cache(reads = true, fingerprint = 'ty abc') {
    return new SnippetCache(CacheFixture.folder(), reads, fingerprint)
  }
}

afterEach(() => {
  for (const folder of CacheFixture.folders.splice(0)) rmSync(folder, { recursive: true, force: true })
})

describe('the snippet cache remembers only passed steps with the same inputs', () => {
  test('a step that passed before is found again', () => {
    const cache = CacheFixture.cache()
    const root = CacheFixture.stagedProject({ 'client/pages/tac.html': '<main />' })
    const key = cache.key(root, ['bundle', root])
    expect(cache.hasPassed(key)).toBe(false)
    cache.rememberPass(key, 'features / mount')
    expect(cache.hasPassed(key)).toBe(true)
    expect(cache.hits).toBe(1)
  })

  test('a changed file, command or toolchain gives a new key', () => {
    const root = CacheFixture.stagedProject({ 'client/pages/tac.html': '<main />' })
    const cache = CacheFixture.cache()
    const first = cache.key(root, ['bundle', root])
    expect(cache.key(root, ['test', root])).not.toBe(first)
    expect(CacheFixture.cache(true, 'ty def').key(root, ['bundle', root])).not.toBe(first)
    writeFileSync(path.join(root, 'client/pages/tac.html'), '<main>changed</main>')
    expect(cache.key(root, ['bundle', root])).not.toBe(first)
  })

  test('the staging path and the build output do not change the key', () => {
    const cache = CacheFixture.cache()
    const first = CacheFixture.stagedProject({ 'client/pages/tac.html': '<main />' })
    const second = CacheFixture.stagedProject({ 'client/pages/tac.html': '<main />', 'dist/index.html': 'built', '.tachyon/state': 'x' })
    expect(cache.key(first, ['bundle', first])).toBe(cache.key(second, ['bundle', second]))
  })

  test('--no-cache never reads an entry but still writes one', () => {
    const folder = CacheFixture.folder()
    const root = CacheFixture.stagedProject({ 'client/pages/tac.html': '<main />' })
    const writer = new SnippetCache(folder, false, 'ty abc')
    const key = writer.key(root, ['bundle', root])
    writer.rememberPass(key, 'features / mount')
    expect(writer.hasPassed(key)).toBe(false)
    expect(new SnippetCache(folder, true, 'ty abc').hasPassed(key)).toBe(true)
  })

  test('a key is refused before the toolchain fingerprint is read', () => {
    const root = CacheFixture.stagedProject({ 'client/pages/tac.html': '<main />' })
    expect(() => CacheFixture.cache(true, '').key(root, ['bundle', root])).toThrow('toolchain fingerprint')
  })
})

describe('the toolchain fingerprint names every setting that changes a result', () => {
  test('framework settings count and run-only settings do not', () => {
    const settings = ToolchainFingerprint.settings({
      TAC_SWIFT_WASM_SDK: '/sdk',
      TAC_SNIPPETS_CONCURRENCY: '4',
      TAC_BIN: '/ty',
      RUSTUP_TOOLCHAIN: '1.90',
      HOME: '/home',
    })
    expect(settings).toEqual(['RUSTUP_TOOLCHAIN=1.90', 'TAC_SWIFT_WASM_SDK=/sdk'])
  })

  test('a missing tool is written as unavailable', async () => {
    expect(await ToolchainFingerprint.version(['tachyon-no-such-tool', ['--version']])).toBe('tachyon-no-such-tool --version: unavailable')
  })
})

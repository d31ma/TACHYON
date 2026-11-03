import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, utimesSync, writeFileSync } from 'node:fs'
import path from 'node:path'

export class SnippetCache {
  static folderName = '.step-cache'
  static rootPlaceholder = '<staging>'
  static skippedFolders = new Set(['.tachyon', 'dist', 'node_modules'])
  static entryLifetimeMilliseconds = 14 * 24 * 60 * 60 * 1000

  /**
   * @param {string} folder
   * @param {boolean} reads
   * @param {string} fingerprint
   */
  constructor(folder, reads, fingerprint) {
    /** @type {string} */
    this.folder = folder
    /** @type {boolean} */
    this.reads = reads
    /** @type {string} */
    this.fingerprint = fingerprint
    /** @type {number} */
    this.hits = 0
    mkdirSync(folder, { recursive: true })
  }

  /**
   * @param {string} root
   * @param {string[]} work
   * @returns {string}
   */
  key(root, work) {
    assert.ok(this.fingerprint, 'Read the toolchain fingerprint before the first cached step')
    const hash = createHash('sha256')
    hash.update(this.fingerprint)
    hash.update('\0')
    for (const part of work) hash.update(`${part.split(root).join(SnippetCache.rootPlaceholder)}\0`)
    for (const file of SnippetCache.filesIn(root)) {
      hash.update(`${path.relative(root, file)}\0`)
      hash.update(readFileSync(file))
      hash.update('\0')
    }
    return hash.digest('hex')
  }

  /**
   * @param {string} key
   * @returns {boolean}
   */
  hasPassed(key) {
    if (!this.reads) return false
    const entry = this.entryFile(key)
    if (!existsSync(entry)) return false
    utimesSync(entry, new Date(), new Date())
    this.hits += 1
    return true
  }

  /**
   * @param {string} key
   * @param {string} label
   */
  rememberPass(key, label) {
    const entry = this.entryFile(key)
    mkdirSync(path.dirname(entry), { recursive: true })
    const unfinished = `${entry}.${process.pid}.part`
    writeFileSync(unfinished, `${label}\n`)
    renameSync(unfinished, entry)
  }

  removeOldEntries() {
    const now = Date.now()
    for (const entry of readdirSync(this.folder, { recursive: true, withFileTypes: true })) {
      if (!entry.isFile()) continue
      const file = path.join(entry.parentPath, entry.name)
      try {
        if (now - statSync(file).mtimeMs > SnippetCache.entryLifetimeMilliseconds) rmSync(file, { force: true })
      } catch {}
    }
  }

  /**
   * @param {string} key
   * @returns {string}
   */
  entryFile(key) {
    return path.join(this.folder, key.slice(0, 2), key)
  }

  /**
   * @param {string} root
   * @returns {string[]}
   */
  static filesIn(root) {
    /** @type {string[]} */
    const found = []
    for (const entry of readdirSync(root, { withFileTypes: true })) {
      if (SnippetCache.skippedFolders.has(entry.name)) continue
      const file = path.join(root, entry.name)
      if (entry.isDirectory()) found.push(...SnippetCache.filesIn(file))
      else if (entry.isFile()) found.push(file)
    }
    return found.sort()
  }
}

import { constants } from 'node:fs'
import { lstat, open, readdir, realpath } from 'node:fs/promises'
import path from 'node:path'

export class PythonTacPublicAssets {
  static fileLimit = 16 * 1024 * 1024
  static totalLimit = 64 * 1024 * 1024
  static entryLimit = 8192
  static fileCountLimit = 4096
  static mime = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json', '.webmanifest': 'application/manifest+json', '.wasm': 'application/wasm', '.zip': 'application/zip', '.svg': 'image/svg+xml', '.png': 'image/png', '.ico': 'image/x-icon', '.woff2': 'font/woff2' }

  #files = new Map()
  #bytes = 0
  #entries = 0

  static async read(directory) {
    PythonTacPublicAssets.#requireDirectory(await lstat(directory))
    const assets = new PythonTacPublicAssets(await realpath(directory))
    await assets.#directory(assets.root, 0)
    return assets
  }

  constructor(root) {
    this.root = root
  }

  respond(request, response) {
    try {
      this.#response(request, response)
    } catch {
      response.writeHead(400, { 'cache-control': 'no-store' }).end()
    }
  }

  static #requireDirectory(metadata) {
    if (!metadata.isDirectory() || metadata.isSymbolicLink()) throw new Error('The public fixture requires regular non-symlinked directories')
  }

  async #directory(directory, depth) {
    this.#requireDepth(depth)
    await this.#entriesIn(directory, depth)
  }

  #requireDepth(depth) {
    if (depth > 32) throw new Error('Public fixture directory depth exceeds 32')
  }

  async #entriesIn(directory, depth) {
    for (const entry of await readdir(directory, { withFileTypes: true })) await this.#entry(directory, depth, entry)
  }

  async #entry(directory, depth, entry) {
    this.#countEntry()
    const file = path.join(directory, entry.name)
    const metadata = await lstat(file)
    if (metadata.isDirectory()) return this.#childDirectory(file, depth, metadata)
    return this.#file(file, metadata)
  }

  async #childDirectory(directory, depth, metadata) {
    PythonTacPublicAssets.#requireDirectory(metadata)
    await this.#directory(directory, depth + 1)
  }

  #countEntry() {
    this.#entries += 1
    if (this.#entries > PythonTacPublicAssets.entryLimit) throw new Error('Public fixture entry count exceeds bounds')
  }

  async #file(file, metadata) {
    this.#requireRegular(metadata)
    await this.#capture(file)
  }

  #requireRegular(metadata) {
    if (!metadata.isFile() || metadata.isSymbolicLink()) throw new Error('The public fixture refuses symlinks and special files')
  }

  async #capture(file) {
    const handle = await open(file, constants.O_RDONLY | constants.O_NOFOLLOW)
    try {
      const bytes = await this.#readFile(handle)
      this.#retain(file, bytes)
    } finally {
      await handle.close()
    }
  }

  async #readFile(handle) {
    const metadata = await handle.stat()
    this.#requireFile(metadata)
    const bytes = Buffer.alloc(metadata.size + 1)
    const size = await this.#readBounded(handle, bytes)
    this.#requireStableLength(metadata.size, size)
    return Buffer.from(bytes.subarray(0, size))
  }

  async #readBounded(handle, bytes) {
    let size = 0
    for await (const chunk of handle.createReadStream({ start: 0, end: bytes.length - 1, autoClose: false })) {
      chunk.copy(bytes, size)
      size += chunk.length
    }
    return size
  }

  #requireStableLength(expected, actual) {
    if (expected !== actual) throw new Error('Public fixture asset changed during capture')
  }

  #requireFile(metadata) {
    if (!metadata.isFile()) throw new Error('Public fixture asset must be a regular file')
    this.#requireLength(metadata.size)
  }

  #requireLength(size) {
    if (size > PythonTacPublicAssets.fileLimit || this.#bytes + size > PythonTacPublicAssets.totalLimit) throw new Error('Public fixture bytes exceed bounds')
  }

  #retain(file, bytes) {
    if (this.#files.size >= PythonTacPublicAssets.fileCountLimit) throw new Error('Public fixture file count exceeds bounds')
    this.#bytes += bytes.length
    this.#files.set(`/${path.relative(this.root, file).split(path.sep).join('/')}`, bytes)
  }

  #response(request, response) {
    if (!['GET', 'HEAD'].includes(request.method)) return response.writeHead(405, { 'cache-control': 'no-store', allow: 'GET, HEAD' }).end()
    return this.#asset(request, response)
  }

  #asset(request, response) {
    const name = PythonTacPublicAssets.#requestPath(request.url)
    const selected = this.#select(name)
    if (!selected) return response.writeHead(404, { 'cache-control': 'no-store' }).end()
    return this.#send(request, response, selected)
  }

  static #requestPath(raw) {
    const name = decodeURIComponent(raw.split('?')[0])
    if (!name.startsWith('/') || name.includes('\\') || name.includes('\0') || name.split('/').some(part => part === '.' || part === '..') || name.length > 1024) throw new Error('Unsafe public fixture asset path')
    return name
  }

  #select(name) {
    const key = this.#files.has(name) ? name : `${name.replace(/\/$/, '')}/index.html`
    const bytes = this.#files.get(key)
    return bytes && { key, bytes }
  }

  #send(request, response, asset) {
    response.writeHead(200, { 'content-type': PythonTacPublicAssets.mime[path.extname(asset.key)] ?? 'application/octet-stream', 'content-length': asset.bytes.length, 'cache-control': 'public, max-age=60', 'x-content-type-options': 'nosniff' })
    response.end(request.method === 'HEAD' ? undefined : asset.bytes)
  }
}

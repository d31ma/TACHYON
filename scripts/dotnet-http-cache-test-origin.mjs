import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { request } from 'node:http'
import { createServer } from 'node:https'
import path from 'node:path'

/** @typedef {{method: string, path: string, cookie: boolean, authorization: boolean}} ForwardedCacheRequest */

export class DotnetHttpCacheTestOrigin {
  /** @type {ForwardedCacheRequest[]} */
  #requests = []
  /** @type {string[]} */
  #failures = []
  /** @type {Set<import('node:http').ClientRequest>} */
  #active = new Set()

  /** @returns {ForwardedCacheRequest[]} */
  requests() {
    return this.#requests.slice()
  }

  /** @returns {string[]} */
  failures() {
    return this.#failures.slice()
  }

  /** @param {string} upstream @param {string} fixture @param {(origin: string) => Promise<unknown>} probe @returns {Promise<unknown>} */
  async run(upstream, fixture, probe) {
    return this.#serve(upstream, fixture, probe)
  }

  /** @param {string} upstream @param {string} fixture @param {(origin: string) => Promise<unknown>} probe @returns {Promise<unknown>} */
  async #serve(upstream, fixture, probe) {
    const [key, cert] = await Promise.all(['dotnet-http-cache-test-key.pem', 'dotnet-http-cache-test-cert.pem'].map(name => readFile(path.join(fixture, name))))
    const server = createServer({ key, cert }, (incoming, outgoing) => this.#forward(upstream, incoming, outgoing))
    server.setTimeout(10_000)
    server.requestTimeout = 10_000
    server.headersTimeout = 10_000
    await this.#listen(server)
    try {
      const address = server.address()
      assert.ok(address && typeof address === 'object', 'TLS fixture must have an ephemeral local port')
      return await probe(`https://tachyon.test:${address.port}`)
    } finally {
      for (const forwarded of this.#active) forwarded.destroy()
      server.closeAllConnections()
      await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve(undefined)))
    }
  }

  /** @param {import('node:https').Server} server @returns {Promise<void>} */
  #listen(server) {
    return new Promise((resolve, reject) => {
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => { server.off('error', reject); resolve() })
    })
  }

  /** @param {string} upstream @param {import('node:http').IncomingMessage} incoming @param {import('node:http').ServerResponse} outgoing @returns {void} */
  #forward(upstream, incoming, outgoing) {
    if (!this.#record(incoming)) { this.#unavailable(outgoing); return }
    const url = new URL(upstream)
    const forwarded = request({ hostname: '127.0.0.1', port: url.port, path: incoming.url, method: incoming.method, headers: { ...incoming.headers, host: url.host } }, response => {
      outgoing.writeHead(response.statusCode ?? 502, response.headers)
      response.on('error', () => outgoing.destroy())
      response.pipe(outgoing)
    })
    this.#active.add(forwarded)
    forwarded.once('close', () => this.#active.delete(forwarded))
    forwarded.setTimeout(10_000, () => forwarded.destroy(new Error('Local Yon upstream deadline exceeded')))
    forwarded.on('error', () => this.#unavailable(outgoing))
    incoming.once('aborted', () => forwarded.destroy())
    incoming.pipe(forwarded)
  }

  /** @param {import('node:http').IncomingMessage} incoming @returns {boolean} */
  #record(incoming) {
    const selected = new URL(incoming.url ?? '/', 'https://tachyon.test').pathname
    if (selected !== '/api/cached-probe') return true
    return this.#retain(incoming, selected)
  }

  /** @param {import('node:http').IncomingMessage} incoming @param {string} selected @returns {boolean} */
  #retain(incoming, selected) {
    if (this.#requests.length >= 32) return false
    this.#requests.push({ method: incoming.method ?? '', path: selected, cookie: incoming.headers.cookie !== undefined, authorization: incoming.headers.authorization !== undefined })
    return true
  }

  /** @param {import('node:http').ServerResponse} outgoing @returns {void} */
  #unavailable(outgoing) {
    if (this.#failures.length < 8) this.#failures.push('Local Yon upstream unavailable')
    if (!outgoing.headersSent) outgoing.writeHead(502)
    outgoing.end('Local Yon upstream unavailable')
  }
}

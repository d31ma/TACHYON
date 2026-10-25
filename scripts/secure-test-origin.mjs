import { SupervisedCommand } from './supervised-command.mjs'
import assert from 'node:assert/strict'
import { X509Certificate, createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { createServer, request } from 'node:http'
import { createServer as createSecureServer } from 'node:https'
import { connect } from 'node:net'
import path from 'node:path'
import process from 'node:process'

/** @typedef {import('playwright').Page} Page */
/** @typedef {{proxy: {server: string}, args: string[], ignoreHTTPSErrors: boolean, firefoxUserPrefs?: Record<string, boolean>}} LaunchOptions */

export class SecureTestOrigin {
  static host = 'tachyon.test'
  static origin = `https://${SecureTestOrigin.host}`
  static readyLine = /Tachyon preview ready at http:\/\/127\.0\.0\.1:(\d+)\//
  static readyMilliseconds = 60_000
  static buildMilliseconds = 900_000

  networkDown = false
  /** @type {Map<string, {type: string, body: string}>} */
  replaced = new Map()
  /** @type {string[]} */
  offlineAttempts = []
  tunnelPort = 0
  trusted = ''
  /** @type {Array<import('node:net').Server>} */
  #servers = []
  /** @type {SupervisedCommand|undefined} */
  #preview

  /** @param {string} binary @param {string} project @returns {Promise<string>} */
  static async bundle(binary, project) {
    const built = await SupervisedCommand.run(binary, ['bundle', project], {
      timeoutMilliseconds: SecureTestOrigin.buildMilliseconds,
      env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
    })
    const output = `${built.stdout}${built.stderr}`
    assert.equal(built.status, 0, `ty bundle failed: ${output}`)
    return output
  }

  /** Serves a bundled project with ty preview --static at https://tachyon.test through a test proxy.
   * @param {string} binary @param {string} project @returns {Promise<SecureTestOrigin>} */
  static async open(binary, project) {
    const origin = new SecureTestOrigin()
    try {
      origin.#preview = await SupervisedCommand.start(binary, ['preview', project, '--static', '--host', '127.0.0.1', '--port', '0'], {
        timeoutMilliseconds: 30 * 60 * 1000,
        env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
      })
      const previewPort = await SecureTestOrigin.readyPort(origin.#preview)
      const files = await SecureTestOrigin.certificate(project)
      origin.trusted = SecureTestOrigin.publicKeyPin(files.certificate)
      const secure = origin.#secureForwarder(files, previewPort)
      origin.#servers.push(secure)
      const tunnel = SecureTestOrigin.tunnel(await SecureTestOrigin.listen(secure))
      origin.#servers.push(tunnel)
      origin.tunnelPort = await SecureTestOrigin.listen(tunnel)
      return origin
    } catch (error) {
      await origin.close()
      throw error
    }
  }

  /** @returns {Promise<void>} */
  async close() {
    for (const server of this.#servers) server.close()
    await this.#preview?.stop()
  }

  /** Chromium trusts the test certificate by its public key, so the origin counts as secure; the other engines only skip the certificate check.
   * @param {string} engine @returns {LaunchOptions} */
  launchOptions(engine) {
    return {
      proxy: { server: `http://127.0.0.1:${this.tunnelPort}` },
      args: engine === 'chromium' ? [`--ignore-certificate-errors-spki-list=${this.trusted}`] : [],
      ignoreHTTPSErrors: engine !== 'chromium',
      firefoxUserPrefs: engine === 'firefox' ? { 'dom.serviceWorkers.testing.enabled': true } : undefined,
    }
  }

  /** @param {Page} page @param {number} milliseconds @returns {Promise<string>} */
  static controlled(page, milliseconds) {
    return page.evaluate(async (wait) => {
      if (!('serviceWorker' in navigator)) return 'no service worker support'
      const ready = navigator.serviceWorker.ready.then(() => 'ready')
      const late = new Promise(resolve => setTimeout(() => resolve('the worker never became ready'), wait))
      const state = await Promise.race([ready, late])
      if (state !== 'ready' || navigator.serviceWorker.controller) return state
      await new Promise(resolve => {
        navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true })
        setTimeout(resolve, wait)
      })
      return navigator.serviceWorker.controller ? 'ready' : 'the worker never took control'
    }, milliseconds)
  }

  /** @param {SupervisedCommand} preview @returns {Promise<number>} */
  static async readyPort(preview) {
    const deadline = Date.now() + SecureTestOrigin.readyMilliseconds
    while (Date.now() < deadline) {
      const match = preview.stdout.toString('utf8').match(SecureTestOrigin.readyLine)
      if (match) return Number(match[1])
      if (preview.exited) throw new Error(`ty preview stopped before it was ready: ${preview.stderr.toString('utf8')}`)
      await new Promise(resolve => setTimeout(resolve, 100))
    }
    throw new Error('ty preview did not print its address in time.')
  }

  /** @param {string} project @returns {Promise<{key: string, certificate: string}>} */
  static async certificate(project) {
    const key = path.join(project, 'key.pem')
    const certificate = path.join(project, 'cert.pem')
    const issued = await SupervisedCommand.run('openssl', [
      'req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', key, '-out', certificate, '-days', '1',
      '-subj', `/CN=${SecureTestOrigin.host}`, '-addext', `subjectAltName=DNS:${SecureTestOrigin.host}`,
    ], { timeoutMilliseconds: 30_000 })
    assert.equal(issued.status, 0, `cannot issue a test certificate: ${issued.stderr}`)
    return { key, certificate }
  }

  /** @param {string} certificate @returns {string} */
  static publicKeyPin(certificate) {
    const publicKey = new X509Certificate(readFileSync(certificate)).publicKey.export({ type: 'spki', format: 'der' })
    return createHash('sha256').update(publicKey).digest('base64')
  }

  /** @param {{key: string, certificate: string}} files @param {number} previewPort @returns {import('node:https').Server} */
  #secureForwarder({ key, certificate }, previewPort) {
    return createSecureServer({ key: readFileSync(key), cert: readFileSync(certificate) }, (incoming, outgoing) => {
      if (this.networkDown) {
        this.offlineAttempts.push(incoming.url ?? '')
        incoming.socket.destroy()
        return
      }
      const replacement = this.replaced.get(incoming.url ?? '')
      if (replacement) {
        outgoing.writeHead(200, { 'content-type': replacement.type, 'cache-control': 'no-store' }).end(replacement.body)
        return
      }
      const headers = { ...incoming.headers, host: `127.0.0.1:${previewPort}` }
      const forward = request({ host: '127.0.0.1', port: previewPort, method: incoming.method, path: incoming.url, headers }, answer => {
        answer.on('error', () => outgoing.destroy())
        outgoing.writeHead(answer.statusCode ?? 502, answer.headers)
        answer.pipe(outgoing)
      })
      forward.on('error', () => outgoing.headersSent ? outgoing.destroy() : outgoing.writeHead(502).end())
      incoming.on('error', () => forward.destroy())
      outgoing.on('error', () => forward.destroy())
      incoming.pipe(forward)
    })
  }

  /** Browsers also open connections to their own services through the proxy; only tachyon.test is tunneled.
   * @param {number} securePort @returns {import('node:http').Server} */
  static tunnel(securePort) {
    const server = createServer((_incoming, outgoing) => outgoing.writeHead(405).end())
    server.on('connect', (incoming, socket, head) => {
      socket.on('error', () => socket.destroy())
      if (!incoming.url?.startsWith(`${SecureTestOrigin.host}:`)) {
        socket.end('HTTP/1.1 403 Forbidden\r\n\r\n')
        return
      }
      const upstream = connect(securePort, '127.0.0.1', () => {
        socket.write('HTTP/1.1 200 Connection Established\r\n\r\n')
        upstream.write(head)
        upstream.pipe(socket)
        socket.pipe(upstream)
      })
      upstream.on('error', () => socket.destroy())
      socket.on('close', () => upstream.destroy())
    })
    return server
  }

  /** @param {import('node:net').Server} server @returns {Promise<number>} */
  static listen(server) {
    return new Promise((resolve, reject) => {
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const address = server.address()
        resolve(typeof address === 'object' && address ? address.port : 0)
      })
    })
  }
}

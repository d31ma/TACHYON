import { readFile } from 'node:fs/promises'
import { createServer as createPlainServer, request } from 'node:http'
import { createServer } from 'node:https'
import { connect } from 'node:net'
import path from 'node:path'
import { PythonTacPublicAssets } from './python-tac-public-assets.mjs'

/** @typedef {{down: boolean, attempts: string[]}} PythonTacTestNetwork */
/** @typedef {{origin: string, proxy: string, network: PythonTacTestNetwork}} PythonTacTestSite */

export class PythonTacTestOrigin {
  static host = 'python-tac.test'

  static async run(upstream, fixture, probe) {
    return PythonTacTestOrigin.serve(fixture, (incoming, outgoing) => PythonTacTestOrigin.forward(upstream, incoming, outgoing), probe)
  }

  static async publicStatic(fixture, probe) {
    const assets = await PythonTacPublicAssets.read(path.join(fixture, 'dist'))
    return PythonTacTestOrigin.serve(fixture, (incoming, outgoing) => assets.respond(incoming, outgoing), probe)
  }

  static async serve(fixture, handler, probe) {
    const [key, cert] = await Promise.all(['python-tac-test-key.pem', 'python-tac-test-cert.pem'].map(name => readFile(path.join(fixture, name))))
    /** @type {PythonTacTestNetwork} */
    const network = { down: false, attempts: [] }
    const server = createServer({ key, cert }, (incoming, outgoing) => PythonTacTestOrigin.answer(network, handler, incoming, outgoing))
    server.setTimeout(10_000)
    server.requestTimeout = 10_000
    server.headersTimeout = 10_000
    await PythonTacTestOrigin.listen(server)
    const tunnel = PythonTacTestOrigin.tunnel(server.address().port)
    try {
      await PythonTacTestOrigin.listen(tunnel)
      return await probe({ origin: `https://${PythonTacTestOrigin.host}:${server.address().port}`, proxy: `http://127.0.0.1:${tunnel.address().port}`, network })
    } finally {
      await Promise.all([server, tunnel].map(PythonTacTestOrigin.close))
    }
  }

  static answer(network, handler, incoming, outgoing) {
    if (!network.down) return handler(incoming, outgoing)
    network.attempts.push(new URL(incoming.url ?? '/', `https://${PythonTacTestOrigin.host}`).pathname)
    incoming.socket.destroy()
  }

  static tunnel(securePort) {
    const server = createPlainServer((_incoming, outgoing) => outgoing.writeHead(405).end())
    const sockets = new Set()
    server.on('connect', (incoming, socket, head) => PythonTacTestOrigin.connect(securePort, sockets, incoming, socket, head))
    return Object.assign(server, { sockets })
  }

  static connect(securePort, sockets, incoming, socket, head) {
    if (incoming.url !== `${PythonTacTestOrigin.host}:${securePort}`) {
      socket.end('HTTP/1.1 403 Forbidden\r\n\r\n')
      return
    }
    const upstream = connect(securePort, '127.0.0.1', () => {
      socket.write('HTTP/1.1 200 Connection Established\r\n\r\n')
      upstream.write(head)
      upstream.pipe(socket)
      socket.pipe(upstream)
    })
    for (const end of [socket, upstream]) {
      sockets.add(end)
      end.on('close', () => sockets.delete(end))
    }
    upstream.on('error', () => socket.destroy())
    socket.on('error', () => upstream.destroy())
  }

  static close(server) {
    server.closeAllConnections?.()
    server.sockets?.forEach(socket => socket.destroy())
    return new Promise((resolve, reject) => server.close(error => error && error.code !== 'ERR_SERVER_NOT_RUNNING' ? reject(error) : resolve()))
  }

  static listen(server) {
    return new Promise((resolve, reject) => {
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => { server.off('error', reject); resolve() })
    })
  }

  static forward(upstream, incoming, outgoing) {
    const url = new URL(upstream)
    const forwarded = request({ hostname: '127.0.0.1', port: url.port, path: incoming.url, method: incoming.method, headers: { ...incoming.headers, host: url.host } }, response => {
      outgoing.writeHead(response.statusCode, response.headers)
      response.on('error', () => outgoing.destroy())
      response.pipe(outgoing)
    })
    forwarded.setTimeout(10_000, () => forwarded.destroy(new Error('preview upstream deadline exceeded')))
    forwarded.on('error', () => PythonTacTestOrigin.unavailable(outgoing))
    incoming.pipe(forwarded)
  }

  static unavailable(outgoing) {
    if (!outgoing.headersSent) outgoing.writeHead(502)
    outgoing.end('Preview upstream unavailable')
  }
}

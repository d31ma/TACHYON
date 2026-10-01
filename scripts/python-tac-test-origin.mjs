import { readFile } from 'node:fs/promises'
import { request } from 'node:http'
import { createServer } from 'node:https'
import path from 'node:path'

export class PythonTacTestOrigin {
  static async run(upstream, fixture, probe) {
    const [key, cert] = await Promise.all(['python-tac-test-key.pem', 'python-tac-test-cert.pem'].map(name => readFile(path.join(fixture, name))))
    const server = createServer({ key, cert }, (incoming, outgoing) => PythonTacTestOrigin.forward(upstream, incoming, outgoing))
    server.setTimeout(10_000)
    await PythonTacTestOrigin.listen(server)
    try {
      return await probe(`https://python-tac.test:${server.address().port}`)
    } finally {
      server.closeAllConnections()
      await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()))
    }
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

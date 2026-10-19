import http from 'node:http'
import https from 'node:https'
import net from 'node:net'
import { readFileSync } from 'node:fs'

export class YonTlsFront {
  static host = '127.0.0.1'

  /** @param {{cert:string,key:string}} files @param {number} yonPort */
  constructor(files, yonPort) {
    this.yonPort = yonPort
    this.server = https.createServer({ cert: readFileSync(files.cert), key: readFileSync(files.key) })
    this.server.on('request', (request, response) => this.#forward(request, response))
    this.server.on('upgrade', (request, socket, head) => this.#tunnel(request, socket, head))
  }

  /** @returns {Promise<number>} */
  listen() {
    return new Promise((resolve, reject) => {
      this.server.once('error', reject)
      this.server.listen(0, YonTlsFront.host, () => resolve(this.server.address().port))
    })
  }

  close() {
    this.server.closeAllConnections()
    this.server.close()
  }

  #forward(request, response) {
    const headers = { ...request.headers, host: this.#upstreamHost() }
    const upstream = http.request({ host: YonTlsFront.host, port: this.yonPort, method: request.method, path: request.url, headers }, (answer) => {
      response.writeHead(answer.statusCode ?? 502, answer.headers)
      answer.pipe(response)
    })
    upstream.once('error', () => response.destroy())
    request.pipe(upstream)
  }

  #upstreamHost() {
    return `${YonTlsFront.host}:${this.yonPort}`
  }

  #headerLine(name, value) {
    return `${name}: ${name.toLowerCase() === 'host' ? this.#upstreamHost() : value}`
  }

  #tunnel(request, socket, head) {
    const upstream = net.connect(this.yonPort, YonTlsFront.host, () => {
      const lines = [`${request.method} ${request.url} HTTP/1.1`]
      for (let index = 0; index < request.rawHeaders.length; index += 2) lines.push(this.#headerLine(request.rawHeaders[index], request.rawHeaders[index + 1]))
      upstream.write(`${lines.join('\r\n')}\r\n\r\n`)
      upstream.write(head)
      upstream.pipe(socket)
      socket.pipe(upstream)
    })
    upstream.once('error', () => socket.destroy())
    socket.once('error', () => upstream.destroy())
  }
}

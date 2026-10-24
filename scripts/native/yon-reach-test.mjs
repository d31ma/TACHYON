#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs'
import { createServer as createHttpsServer } from 'node:https'
import { connect, createServer } from 'node:net'
import { request as httpRequest } from 'node:http'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { YonReachFixture } from './yon-reach-fixture.mjs'

class YonReachTest {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static targets = YonReachTest.chosenTargets()
  static services = process.env.TAC_YON_SERVICES ?? 'swift'
  static serviceNames = { swift: 'Swift', rust: 'Rust' }
  static appOrigin = 'tachyon-app://bundle'
  static proofTimeout = 90000
  static pause = 250
  static deviceSet = ''
  static device = ''

  static chosenTargets() {
    const named = process.argv.find((argument) => argument.startsWith('--target='))?.slice('--target='.length)
    const fallback = process.platform === 'darwin' ? 'macos,ios,ios-tls' : 'web'
    return (named ?? fallback).split(',')
  }

  /** @param {string} program @param {string[]} args @param {number} timeout */
  static run(program, args, timeout = 240000) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout, maxBuffer: 8 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address()
        server.close(() => resolve(port))
      })
    })
  }

  /** @param {(origin: string) => Promise<void>} body @param {{appOrigins: string[], native: boolean, target: string}} options */
  static async withYon(body, { appOrigins, native, target }, yonOrigin) {
    const fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'yon-reach-'))
    try {
      const yonPort = await this.freePort()
      const origin = yonOrigin ?? `http://127.0.0.1:${yonPort}`
      YonReachFixture.prepare(fixture, { yonOrigin: typeof origin === 'function' ? origin(yonPort) : origin, appOrigins, native, services: this.services })
      const build = target === 'web' ? ['bundle', fixture] : ['build', fixture, '--target', target]
      this.run(this.binary, build)
      await withServer(this.binary, ['serve', fixture, '--no-watch', '--port', String(yonPort)], async () => {
        await body({ fixture, yonPort })
      })
    } finally {
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1') rmSync(fixture, { recursive: true, force: true })
    }
  }

  static async web() {
    const { chromium } = await import('playwright')
    const pagePort = await this.freePort()
    const pageOrigin = `http://127.0.0.1:${pagePort}`
    await this.withYon(async ({ fixture }) => {
      await withServer(this.binary, ['preview', fixture, '--static', '--port', String(pagePort)], async (pages) => {
        const browser = await chromium.launch({ headless: true })
        try {
          const page = await browser.newPage()
          await page.goto((await pages('/')).url)
          await page.waitForFunction(() => ['done', 'error'].includes(document.querySelector('#yon-phase')?.textContent), null, { timeout: this.proofTimeout })
          YonReachFixture.assertProof(JSON.parse(await page.textContent('#yon-proof')), { pageOrigin, native: false })
        } finally {
          await browser.close()
        }
      })
    }, { appOrigins: [pageOrigin], native: false, target: 'web' })
    console.log('Yon reach passed in Chromium: HTTP, a CORS preflight, a stream, a socket, a topic, a 422 problem and a signed-in route on another origin.')
  }

  /** @param {string} fixture @param {string} target */
  static application(fixture, target) {
    const output = path.join(fixture, 'dist', target)
    const bundle = readdirSync(output).find((name) => name.endsWith('.app'))
    assert.ok(bundle, `no .app in ${output}`)
    return path.join(output, bundle)
  }

  /** @param {string} file */
  static async proof(file) {
    const deadline = Date.now() + this.proofTimeout
    while (Date.now() < deadline && !existsSync(file)) await new Promise((resolve) => setTimeout(resolve, this.pause))
    assert.ok(existsSync(file), `the app wrote no proof at ${file}`)
    return JSON.parse(JSON.parse(readFileSync(file, 'utf8')).proof)
  }

  static async macos() {
    await this.withYon(async ({ fixture }) => {
      const application = this.application(fixture, 'macos')
      const file = path.join(this.run('/usr/bin/getconf', ['DARWIN_USER_TEMP_DIR']), YonReachFixture.proofFile)
      rmSync(file, { force: true })
      this.run('/usr/bin/open', ['-n', application])
      try {
        YonReachFixture.assertProof(await this.proof(file), { pageOrigin: this.appOrigin, native: true })
      } finally {
        spawnSync('/usr/bin/pkill', ['-f', path.join(application, 'Contents/MacOS')])
      }
    }, { appOrigins: [this.appOrigin], native: true, target: 'macos' })
    console.log(`Yon reach passed on macOS: the WKWebView page and the linked ${this.serviceNames[this.services]} services reached Yon from tachyon-app://bundle.`)
  }

  /** @param {string[]} args */
  static simctl(...args) {
    return this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args])
  }

  static bootSimulator() {
    this.deviceSet = mkdtempSync(path.join(tmpdir(), 'yon-reach-devices-'))
    const runtimes = JSON.parse(this.simctl('list', 'runtimes', '--json')).runtimes
    const runtime = runtimes.filter((item) => item.isAvailable && item.identifier.includes('.iOS-'))
      .sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0]
    assert.ok(runtime, 'an installed iOS simulator runtime is required')
    const phone = (runtime.supportedDeviceTypes ?? []).find((item) => item.productFamily === 'iPhone')
    assert.ok(phone, 'the iOS runtime needs an iPhone device type')
    this.device = this.simctl('create', `YonReach-${randomUUID()}`, phone.identifier, runtime.identifier)
    this.simctl('boot', this.device)
    this.run('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, 'bootstatus', this.device, '-b'])
  }

  static shutdownSimulator() {
    for (const args of [['shutdown', this.device], ['delete', this.device]]) {
      spawnSync('/usr/bin/xcrun', ['simctl', '--set', this.deviceSet, ...args], { timeout: 60000, stdio: 'ignore' })
    }
    rmSync(this.deviceSet, { recursive: true, force: true })
  }

  /** @param {boolean} secure */
  static async ios(secure) {
    this.bootSimulator()
    try {
      const run = async ({ fixture }) => {
        const application = this.application(fixture, 'ios')
        this.simctl('install', this.device, application)
        this.simctl('launch', this.device, YonReachFixture.applicationId)
        const container = this.simctl('get_app_container', this.device, YonReachFixture.applicationId, 'data')
        YonReachFixture.assertProof(await this.proof(path.join(container, 'tmp', YonReachFixture.proofFile)), { pageOrigin: this.appOrigin, native: true })
        this.simctl('terminate', this.device, YonReachFixture.applicationId)
      }
      const options = { appOrigins: [this.appOrigin], native: true, target: 'ios' }
      if (secure) await this.withTls((origin, body) => this.withYon(body, options, origin), run)
      else await this.withYon(run, options)
    } finally {
      this.shutdownSimulator()
    }
    console.log(`Yon reach passed on the iOS simulator over ${secure ? 'https' : 'http'}: the WKWebView page and the linked ${this.serviceNames[this.services]} services reached Yon from tachyon-app://bundle.`)
  }

  /** @param {(origin: (yonPort: number) => string, body: Function) => Promise<void>} withYon @param {Function} body */
  static async withTls(withYon, body) {
    const directory = mkdtempSync(path.join(tmpdir(), 'yon-reach-tls-'))
    const key = path.join(directory, 'key.pem')
    const certificate = path.join(directory, 'certificate.pem')
    this.run('/usr/bin/openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-sha256', '-nodes', '-days', '2', '-subj', '/CN=localhost',
      '-addext', 'subjectAltName=DNS:localhost', '-addext', 'extendedKeyUsage=serverAuth', '-keyout', key, '-out', certificate])
    this.simctl('keychain', this.device, 'add-root-cert', certificate)
    const proxyPort = await this.freePort()
    let yonPort = 0
    const proxy = this.tlsProxy(readFileSync(key), readFileSync(certificate), () => yonPort)
    await new Promise((resolve) => proxy.listen(proxyPort, '127.0.0.1', resolve))
    try {
      await withYon((port) => { yonPort = port; return `https://localhost:${proxyPort}` }, body)
    } finally {
      proxy.close()
      rmSync(directory, { recursive: true, force: true })
    }
  }

  /** @param {Buffer} key @param {Buffer} cert @param {() => number} yonPort */
  static tlsProxy(key, cert, yonPort) {
    const proxy = createHttpsServer({ key, cert }, (incoming, outgoing) => {
      const forwarded = httpRequest({ host: '127.0.0.1', port: yonPort(), method: incoming.method, path: incoming.url,
        headers: { ...incoming.headers, host: `127.0.0.1:${yonPort()}` } }, (answer) => {
        outgoing.writeHead(answer.statusCode ?? 502, answer.headers)
        answer.pipe(outgoing)
      })
      forwarded.on('error', () => outgoing.destroy())
      incoming.pipe(forwarded)
    })
    proxy.on('upgrade', (incoming, socket, head) => {
      const upstream = connect(yonPort(), '127.0.0.1', () => {
        const headers = Object.entries({ ...incoming.headers, host: `127.0.0.1:${yonPort()}` }).map(([name, value]) => `${name}: ${value}`)
        upstream.write([`${incoming.method} ${incoming.url} HTTP/1.1`, ...headers, '', ''].join('\r\n'))
        upstream.write(head)
        upstream.pipe(socket)
        socket.pipe(upstream)
      })
      upstream.on('error', () => socket.destroy())
      socket.on('error', () => upstream.destroy())
    })
    return proxy
  }

  static async main() {
    assert.ok(this.serviceNames[this.services], `unknown TAC_YON_SERVICES ${this.services}; use swift or rust`)
    const checks = { web: () => this.web(), macos: () => this.macos(), ios: () => this.ios(false), 'ios-tls': () => this.ios(true) }
    for (const target of this.targets) {
      assert.ok(checks[target], `unknown target ${target}; use web, macos, ios or ios-tls`)
      if (target !== 'web') assert.equal(process.platform, 'darwin', `${target} needs macOS and Xcode`)
      await checks[target]()
    }
  }
}

await YonReachTest.main()

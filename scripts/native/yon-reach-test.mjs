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
  static serviceNames = { swift: 'Swift', rust: 'Rust', kotlin: 'Kotlin' }
  static androidOrigin = 'https://appassets.tachyon.local'
  static androidPackage = YonReachFixture.applicationId.replaceAll('-', '_')
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
      if (native) YonReachFixture.assertLinked(path.join(fixture, 'dist', target), this.serviceNames[this.services])
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

  static adb() {
    const sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
    assert.ok(sdk, 'set ANDROID_HOME for the Android run')
    return path.join(sdk, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  }

  /** @param {string} fixture */
  static androidApk(fixture) {
    const output = path.join(fixture, 'dist/android')
    YonReachFixture.assertLinked(output, this.serviceNames[this.services])
    return path.join(output, YonReachFixture.applicationName, `${YonReachFixture.applicationName}.apk`)
  }

  /** @param {string} apk */
  static async launchAndroid(apk) {
    const adb = this.adb()
    this.run(adb, ['install', '-r', apk])
    this.run(adb, ['shell', 'pm', 'clear', this.androidPackage])
    this.run(adb, ['shell', 'am', 'start', '-W', '-n', `${this.androidPackage}/.MainActivity`])
    let pid = ''
    for (let attempt = 0; attempt < 120 && !(pid && this.run(adb, ['shell', 'cat', '/proc/net/unix']).includes(`webview_devtools_remote_${pid}`)); attempt++) {
      await new Promise((resolve) => setTimeout(resolve, this.pause * 2))
      pid = spawnSync(adb, ['shell', 'pidof', this.androidPackage], { encoding: 'utf8', timeout: 30000 }).stdout.trim()
    }
    assert.match(pid, /^\d+$/, 'the Android app must start')
    return pid
  }

  static async android() {
    assert.notEqual(this.services, 'swift', 'the Android app links Kotlin or Rust services; set TAC_YON_SERVICES=kotlin or rust')
    const { chromium } = await import('playwright')
    const adb = this.adb()
    this.run(adb, ['wait-for-device'])
    await this.withYon(async ({ fixture, yonPort }) => {
      const apk = this.androidApk(fixture)
      this.run(adb, ['reverse', `tcp:${yonPort}`, `tcp:${yonPort}`])
      let browser
      let forwarded
      try {
        const pid = await this.launchAndroid(apk)
        forwarded = this.run(adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`])
        browser = await chromium.connectOverCDP(`http://127.0.0.1:${forwarded}`, { noDefaults: true })
        const page = browser.contexts()[0].pages()[0]
        assert.ok(page, 'the Android WebView exposes a live document')
        await page.waitForFunction(() => ['done', 'error'].includes(document.querySelector('#yon-phase')?.textContent), null, { timeout: this.proofTimeout })
        YonReachFixture.assertProof(JSON.parse(await page.textContent('#yon-proof')), { pageOrigin: this.androidOrigin, native: true })
        const linkedPid = await page.evaluate(() => __tachyonTac.service('yonReach').record())
        assert.equal(Number(linkedPid), Number(pid), 'the linked service ran inside the Android app process')
      } finally {
        if (browser) await browser.close()
        if (forwarded) this.run(adb, ['forward', '--remove', `tcp:${forwarded}`])
        this.run(adb, ['reverse', '--remove', `tcp:${yonPort}`])
        spawnSync(adb, ['shell', 'am', 'force-stop', this.androidPackage], { timeout: 30000 })
      }
    }, { appOrigins: [this.androidOrigin], native: true, target: 'android' }, (port) => `http://localhost:${port}`)
    console.log(`Yon reach passed on the Android emulator: the page held the linked ${this.serviceNames[this.services]} services' stream and socket clients and delivered their messages, and the page and a linked ${this.serviceNames[this.services]} HTTP client reached Yon from ${this.androidOrigin}.`)
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
    assert.ok(this.serviceNames[this.services], `unknown TAC_YON_SERVICES ${this.services}; use swift, rust or kotlin`)
    const checks = { web: () => this.web(), macos: () => this.macos(), ios: () => this.ios(false), 'ios-tls': () => this.ios(true), android: () => this.android() }
    for (const target of this.targets) {
      assert.ok(checks[target], `unknown target ${target}; use web, macos, ios, ios-tls or android`)
      if (!['web', 'android'].includes(target)) assert.equal(process.platform, 'darwin', `${target} needs macOS and Xcode`)
      await checks[target]()
    }
  }
}

await YonReachTest.main()

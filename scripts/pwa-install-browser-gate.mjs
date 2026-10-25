#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { SupervisedCommand } from './supervised-command.mjs'
import assert from 'node:assert/strict'
import { X509Certificate, createHash } from 'node:crypto'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer, request } from 'node:http'
import { createServer as createSecureServer } from 'node:https'
import { connect } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { crc32, deflateSync } from 'node:zlib'
import { chromium } from 'playwright'

/** @typedef {import('playwright').BrowserContext} BrowserContext */
/** @typedef {import('playwright').Page} Page */
/** @typedef {{url: string, errors: Array<{message: string}>, data?: string}} AppManifest */
/** @typedef {{errorId: string}} InstallabilityError */
/** @typedef {{tunnelPort: number, trusted: string}} Network */
/** @typedef {{installable: string[], appId: string|null, manifestType: string, iconTypes: string[], offlineTitle: string, refused: string[]}} GateResult */

class InstallGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static host = 'tachyon.test'
  static origin = `https://${InstallGate.host}`
  static readyLine = /Tachyon preview ready at http:\/\/127\.0\.0\.1:(\d+)\//
  static readyMilliseconds = 60_000
  static stepMilliseconds = 20_000
  static buildMilliseconds = 180_000
  static published = '/manifest.webmanifest'
  static icons = [{ src: '/shared/icons/install-192.png', edge: 192 }, { src: '/shared/icons/install-512.png', edge: 512 }]
  static manifest = {
    name: 'Install Gate',
    short_name: 'Install',
    id: '/',
    start_url: '/',
    scope: '/',
    display: 'standalone',
    background_color: '#ffffff',
    theme_color: '#008000',
    icons: InstallGate.icons.map(icon => ({ src: icon.src, sizes: `${icon.edge}x${icon.edge}`, type: 'image/png', purpose: 'any' })),
  }
  static refusedManifest = { name: 'Install Gate', start_url: '/', display: 'browser', icons: [] }
  static refusals = ['manifest-display-not-supported', 'manifest-missing-suitable-icon']
  /** @type {string|null} */
  static replacedManifest = null
  static networkDown = false

  /** @returns {Promise<void>} */
  static async run() {
    const project = mkdtempSync(path.join(tmpdir(), 'ty-pwa-install-gate-'))
    /** @type {SupervisedCommand|undefined} */
    let preview
    /** @type {Array<import('node:net').Server>} */
    const servers = []
    try {
      InstallGate.writeFixture(project)
      await InstallGate.bundle(project)
      preview = await SupervisedCommand.start(InstallGate.binary, ['preview', project, '--static', '--host', '127.0.0.1', '--port', '0'], {
        timeoutMilliseconds: 30 * 60 * 1000,
        env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
      })
      const previewPort = await InstallGate.readyPort(preview)
      const files = await InstallGate.certificate(project)
      const secure = InstallGate.secureForwarder(files, previewPort)
      servers.push(secure)
      const tunnel = InstallGate.tunnel(await InstallGate.listen(secure))
      servers.push(tunnel)
      const network = { tunnelPort: await InstallGate.listen(tunnel), trusted: InstallGate.publicKeyPin(files.certificate) }
      const installable = await InstallGate.probe(project, network)
      InstallGate.replacedManifest = JSON.stringify(InstallGate.refusedManifest)
      const refused = await InstallGate.refusedProbe(project, network)
      InstallGate.replacedManifest = null
      /** @type {GateResult} */
      const result = { ...installable, refused }
      console.log(JSON.stringify(result, null, 2))
      console.log('PASS: PWA install browser gate')
    } finally {
      for (const server of servers) server.close()
      await preview?.stop()
      rmSync(project, { recursive: true, force: true })
    }
  }

  /** @param {string} project @param {string} relative @param {string|Buffer} contents @returns {void} */
  static write(project, relative, contents) {
    const file = path.join(project, relative)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }

  /** @param {string} project @returns {void} */
  static writeFixture(project) {
    InstallGate.write(project, 'client/manifest.json', `${JSON.stringify(InstallGate.manifest, null, 2)}\n`)
    for (const icon of InstallGate.icons) InstallGate.write(project, path.join('client', icon.src), InstallGate.png(icon.edge))
    InstallGate.write(project, 'client/pages/tac.html', '<install-page />\n')
    InstallGate.write(project, 'client/components/install/page/tac.html', '<main aria-label="Installed"><h1 id="title">Installed offline</h1></main>\n')
  }

  /** @param {number} edge @returns {Buffer} */
  static png(edge) {
    const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(edge * 3, Buffer.from([0, 128, 0]))])
    const header = Buffer.alloc(13)
    header.writeUInt32BE(edge, 0)
    header.writeUInt32BE(edge, 4)
    header.set([8, 2, 0, 0, 0], 8)
    return Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      InstallGate.chunk('IHDR', header),
      InstallGate.chunk('IDAT', deflateSync(Buffer.concat(Array.from({ length: edge }, () => row)))),
      InstallGate.chunk('IEND', Buffer.alloc(0)),
    ])
  }

  /** @param {string} kind @param {Buffer} data @returns {Buffer} */
  static chunk(kind, data) {
    const length = Buffer.alloc(4)
    length.writeUInt32BE(data.length)
    const body = Buffer.concat([Buffer.from(kind, 'ascii'), data])
    const check = Buffer.alloc(4)
    check.writeUInt32BE(crc32(body))
    return Buffer.concat([length, body, check])
  }

  /** @param {string} project @returns {Promise<void>} */
  static async bundle(project) {
    const built = await SupervisedCommand.run(InstallGate.binary, ['bundle', project], {
      timeoutMilliseconds: InstallGate.buildMilliseconds,
      env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
    })
    assert.equal(built.status, 0, `ty bundle failed: ${built.stderr || built.stdout}`)
  }

  /** @param {SupervisedCommand} preview @returns {Promise<number>} */
  static async readyPort(preview) {
    const deadline = Date.now() + InstallGate.readyMilliseconds
    while (Date.now() < deadline) {
      const match = preview.stdout.toString('utf8').match(InstallGate.readyLine)
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
      '-subj', `/CN=${InstallGate.host}`, '-addext', `subjectAltName=DNS:${InstallGate.host}`,
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
  static secureForwarder({ key, certificate }, previewPort) {
    return createSecureServer({ key: readFileSync(key), cert: readFileSync(certificate) }, (incoming, outgoing) => {
      if (InstallGate.networkDown) {
        incoming.socket.destroy()
        return
      }
      if (InstallGate.replacedManifest !== null && incoming.url === InstallGate.published) {
        outgoing.writeHead(200, { 'content-type': 'application/manifest+json', 'cache-control': 'no-store' }).end(InstallGate.replacedManifest)
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

  /** @param {number} securePort @returns {import('node:http').Server} */
  static tunnel(securePort) {
    const server = createServer((_incoming, outgoing) => outgoing.writeHead(405).end())
    server.on('connect', (incoming, socket, head) => {
      socket.on('error', () => socket.destroy())
      if (!incoming.url?.startsWith(`${InstallGate.host}:`)) {
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

  /** @param {string} project @param {string} name @param {Network} network @returns {Promise<BrowserContext>} */
  static async launch(project, name, { tunnelPort, trusted }) {
    const context = await chromium.launchPersistentContext(mkdtempSync(path.join(project, `${name}-profile-`)), {
      channel: 'chromium',
      headless: true,
      proxy: { server: `http://127.0.0.1:${tunnelPort}` },
      args: [`--ignore-certificate-errors-spki-list=${trusted}`],
    })
    context.setDefaultTimeout(InstallGate.stepMilliseconds)
    context.setDefaultNavigationTimeout(InstallGate.stepMilliseconds)
    return context
  }

  /** @param {string} project @param {Network} network @returns {Promise<Omit<GateResult, 'refused'>>} */
  static async probe(project, network) {
    const context = await InstallGate.launch(project, 'installable', network)
    try {
      const page = context.pages()[0] ?? await context.newPage()
      await page.goto(`${InstallGate.origin}/`, { waitUntil: 'load' })
      await InstallGate.requireHead(page)
      const manifestType = await InstallGate.requirePublishedManifest(page)
      const iconTypes = await InstallGate.requireIcons(page)
      await InstallGate.requireController(page)
      const session = await context.newCDPSession(page)
      await InstallGate.requireParsedManifest(session)
      const installable = await InstallGate.installability(session, errors => errors.length === 0)
      assert.deepEqual(installable, [], `Chromium finds the app installable (${installable.join(', ')})`)
      const { appId } = /** @type {{appId?: string}} */ (await session.send('Page.getAppId'))
      assert.equal(appId, `${InstallGate.origin}/`, 'Chromium takes the app id from the manifest id')
      const offlineTitle = await InstallGate.requireOfflineStart(context, page)
      return { installable, appId: appId ?? null, manifestType, iconTypes, offlineTitle }
    } finally {
      await context.close()
    }
  }

  /** @param {string} project @param {Network} network @returns {Promise<string[]>} */
  static async refusedProbe(project, network) {
    const context = await InstallGate.launch(project, 'refused', network)
    try {
      const page = context.pages()[0] ?? await context.newPage()
      await page.goto(`${InstallGate.origin}/`, { waitUntil: 'load' })
      const session = await context.newCDPSession(page)
      const refused = await InstallGate.installability(session, errors => InstallGate.refusals.every(error => errors.includes(error)))
      for (const error of InstallGate.refusals) assert.ok(refused.includes(error), `Chromium refuses a manifest without icons or an app display (${refused.join(', ')})`)
      return refused
    } finally {
      await context.close()
    }
  }

  /** @param {Page} page @returns {Promise<void>} */
  static async requireHead(page) {
    const head = await page.evaluate(() => ({
      manifest: document.querySelector('link[rel="manifest"]')?.getAttribute('href') ?? null,
      touchIcon: document.querySelector('link[rel="apple-touch-icon"]')?.getAttribute('href') ?? null,
      themeColor: document.querySelector('meta[name="theme-color"]')?.getAttribute('content') ?? null,
    }))
    assert.equal(head.manifest, InstallGate.published, 'the page links the published manifest')
    assert.equal(head.touchIcon, InstallGate.icons.at(-1)?.src, 'the page names the largest icon for iOS home screens')
    assert.equal(head.themeColor, InstallGate.manifest.theme_color, 'the page carries the manifest theme color')
  }

  /** @param {Page} page @returns {Promise<string>} */
  static async requirePublishedManifest(page) {
    const answer = await InstallGate.fetched(page, InstallGate.published)
    assert.equal(answer.status, 200, 'the manifest is served')
    assert.match(answer.type, /^application\/manifest\+json\b/, 'the manifest has the web app manifest media type')
    assert.deepEqual(JSON.parse(answer.text), InstallGate.manifest, 'the published manifest is the client manifest unchanged')
    return answer.type
  }

  /** @param {Page} page @returns {Promise<string[]>} */
  static async requireIcons(page) {
    const types = []
    for (const icon of InstallGate.icons) {
      const answer = await InstallGate.fetched(page, icon.src)
      assert.equal(answer.status, 200, `${icon.src} is served`)
      assert.match(answer.type, /^image\/png\b/, `${icon.src} is served as PNG`)
      types.push(answer.type)
    }
    return types
  }

  /** @param {Page} page @param {string} file @returns {Promise<{status: number, type: string, text: string}>} */
  static fetched(page, file) {
    return page.evaluate(async (url) => {
      const answer = await fetch(url)
      return { status: answer.status, type: answer.headers.get('content-type') ?? '', text: await answer.text() }
    }, file)
  }

  /** @param {Page} page @returns {Promise<void>} */
  static async requireController(page) {
    const controlled = await page.evaluate(async (milliseconds) => {
      if (!('serviceWorker' in navigator)) return 'no service worker support'
      const ready = navigator.serviceWorker.ready.then(() => 'ready')
      const late = new Promise(resolve => setTimeout(() => resolve('the worker never became ready'), milliseconds))
      const state = await Promise.race([ready, late])
      if (state !== 'ready' || navigator.serviceWorker.controller) return state
      await new Promise(resolve => {
        navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true })
        setTimeout(resolve, milliseconds)
      })
      return navigator.serviceWorker.controller ? 'ready' : 'the worker never took control'
    }, InstallGate.stepMilliseconds / 2)
    assert.equal(controlled, 'ready', 'the Tachyon service worker controls the page')
  }

  /** @param {import('playwright').CDPSession} session @returns {Promise<void>} */
  static async requireParsedManifest(session) {
    const manifest = /** @type {AppManifest} */ (await session.send('Page.getAppManifest'))
    assert.equal(manifest.url, `${InstallGate.origin}${InstallGate.published}`, 'Chromium reads the published manifest')
    assert.deepEqual(manifest.errors, [], 'Chromium parses the manifest without errors')
  }

  /** @param {import('playwright').CDPSession} session @param {(errors: string[]) => boolean} settled @returns {Promise<string[]>} */
  static async installability(session, settled) {
    /** @type {string[]} */
    let errors = []
    const deadline = Date.now() + InstallGate.stepMilliseconds
    while (Date.now() < deadline) {
      const answer = /** @type {{installabilityErrors: InstallabilityError[]}} */ (await session.send('Page.getInstallabilityErrors'))
      errors = answer.installabilityErrors.map(error => error.errorId)
      if (settled(errors)) break
      await new Promise(resolve => setTimeout(resolve, 250))
    }
    return errors
  }

  /** @param {BrowserContext} context @param {Page} page @returns {Promise<string>} */
  static async requireOfflineStart(context, page) {
    await page.reload({ waitUntil: 'load' })
    InstallGate.networkDown = true
    await context.setOffline(true)
    try {
      await page.goto(`${InstallGate.origin}${InstallGate.manifest.start_url}`, { waitUntil: 'load' })
      const title = await page.locator('#title').innerText()
      assert.equal(title, 'Installed offline', 'the start URL opens offline, as an installed app does')
      return title
    } finally {
      await context.setOffline(false)
      InstallGate.networkDown = false
    }
  }
}

await InstallGate.run()

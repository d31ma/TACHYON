#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { SupervisedCommand } from './supervised-command.mjs'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer, request } from 'node:http'
import { createServer as createSecureServer } from 'node:https'
import { connect } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { chromium, firefox, webkit } from 'playwright'

/** @typedef {import('playwright').BrowserType} BrowserType */
/** @typedef {import('playwright').BrowserContext} BrowserContext */
/** @typedef {import('playwright').Page} Page */
/** @typedef {{cached: string[], policies: Record<string, string|null>}} CachedFiles */
/** @typedef {{browser: string, cached: string[], offlineTitle: string, offlineColor: string, offlineAttempts: string[]}} BrowserResult */

class PreviewOfflineGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static host = 'tachyon.test'
  static origin = `https://${PreviewOfflineGate.host}`
  static readyLine = /Tachyon preview ready at http:\/\/127\.0\.0\.1:(\d+)\//
  static readyMilliseconds = 60_000
  static stepMilliseconds = 20_000
  static buildMilliseconds = 180_000
  static document = '/'
  static listedFiles = ['/', '/.tachyon/tac-client.js', '/.tachyon/components.css']
  static emulatedOffline = ['chromium', 'firefox']
  static green = 'rgb(0, 128, 0)'
  /** @type {Array<[string, BrowserType]>} */
  static browsers = [['chromium', chromium], ['firefox', firefox], ['webkit', webkit]]
  static selected = (process.env.TAC_OFFLINE_BROWSERS ?? 'chromium,firefox,webkit').split(',')
  static networkDown = false
  /** @type {string[]} */
  static offlineAttempts = []

  /** @returns {Promise<void>} */
  static async run() {
    const project = mkdtempSync(path.join(tmpdir(), 'ty-preview-offline-gate-'))
    /** @type {SupervisedCommand|undefined} */
    let preview
    /** @type {Array<import('node:net').Server>} */
    const servers = []
    try {
      PreviewOfflineGate.writeFixture(project)
      await PreviewOfflineGate.bundle(project)
      preview = await SupervisedCommand.start(PreviewOfflineGate.binary, ['preview', project, '--static', '--host', '127.0.0.1', '--port', '0'], {
        timeoutMilliseconds: 30 * 60 * 1000,
        env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
      })
      const previewPort = await PreviewOfflineGate.readyPort(preview)
      const secure = PreviewOfflineGate.secureForwarder(await PreviewOfflineGate.certificate(project), previewPort)
      servers.push(secure)
      const securePort = await PreviewOfflineGate.listen(secure)
      const tunnel = PreviewOfflineGate.tunnel(securePort)
      servers.push(tunnel)
      const tunnelPort = await PreviewOfflineGate.listen(tunnel)
      /** @type {BrowserResult[]} */
      const results = []
      for (const [name, type] of PreviewOfflineGate.browsers) {
        if (!PreviewOfflineGate.selected.includes(name)) continue
        results.push(await PreviewOfflineGate.probe(name, type, project, tunnelPort))
      }
      console.log(JSON.stringify(results, null, 2))
      console.log('PASS: preview offline browser gate')
    } finally {
      for (const server of servers) server.close()
      await preview?.stop()
      rmSync(project, { recursive: true, force: true })
    }
  }

  /** @param {string} project @param {string} relative @param {string} contents @returns {void} */
  static write(project, relative, contents) {
    const file = path.join(project, relative)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }

  /** @param {string} project @returns {void} */
  static writeFixture(project) {
    PreviewOfflineGate.write(project, 'client/pages/tac.html', '<offline-page />\n')
    PreviewOfflineGate.write(project, 'client/components/offline/page/tac.html', '<main aria-label="Offline"><h1 id="title">Kept offline</h1></main>\n')
    PreviewOfflineGate.write(project, 'client/components/offline/page/tac.css', '#title { color: rgb(0, 128, 0) }\n')
  }

  /** @param {string} project @returns {Promise<void>} */
  static async bundle(project) {
    const built = await SupervisedCommand.run(PreviewOfflineGate.binary, ['bundle', project], {
      timeoutMilliseconds: PreviewOfflineGate.buildMilliseconds,
      env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
    })
    assert.equal(built.status, 0, `ty bundle failed: ${built.stderr || built.stdout}`)
  }

  /** @param {SupervisedCommand} preview @returns {Promise<number>} */
  static async readyPort(preview) {
    const deadline = Date.now() + PreviewOfflineGate.readyMilliseconds
    while (Date.now() < deadline) {
      const match = preview.stdout.toString('utf8').match(PreviewOfflineGate.readyLine)
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
      '-subj', `/CN=${PreviewOfflineGate.host}`, '-addext', `subjectAltName=DNS:${PreviewOfflineGate.host}`,
    ], { timeoutMilliseconds: 30_000 })
    assert.equal(issued.status, 0, `cannot issue a test certificate: ${issued.stderr}`)
    return { key, certificate }
  }

  /** @param {{key: string, certificate: string}} files @param {number} previewPort @returns {import('node:https').Server} */
  static secureForwarder({ key, certificate }, previewPort) {
    return createSecureServer({ key: readFileSync(key), cert: readFileSync(certificate) }, (incoming, outgoing) => {
      if (PreviewOfflineGate.networkDown) {
        PreviewOfflineGate.offlineAttempts.push(incoming.url ?? '')
        incoming.socket.destroy()
        return
      }
      const headers = { ...incoming.headers, host: `127.0.0.1:${previewPort}` }
      const forward = request({ host: '127.0.0.1', port: previewPort, method: incoming.method, path: incoming.url, headers }, answer => {
        outgoing.writeHead(answer.statusCode ?? 502, answer.headers)
        answer.pipe(outgoing)
      })
      forward.on('error', () => outgoing.writeHead(502).end())
      incoming.pipe(forward)
    })
  }

  /** @param {number} securePort @returns {import('node:http').Server} */
  static tunnel(securePort) {
    const server = createServer((_incoming, outgoing) => outgoing.writeHead(405).end())
    server.on('connect', (incoming, socket, head) => {
      if (!incoming.url?.startsWith(`${PreviewOfflineGate.host}:`)) {
        socket.on('error', () => socket.destroy())
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
      socket.on('error', () => upstream.destroy())
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

  /** @param {string} name @param {BrowserType} type @param {string} project @param {number} tunnelPort @returns {Promise<BrowserResult>} */
  static async probe(name, type, project, tunnelPort) {
    const profile = mkdtempSync(path.join(project, `${name}-profile-`))
    const context = await type.launchPersistentContext(profile, {
      headless: true,
      ignoreHTTPSErrors: true,
      proxy: { server: `http://127.0.0.1:${tunnelPort}` },
      args: name === 'chromium' ? ['--ignore-certificate-errors'] : [],
      firefoxUserPrefs: name === 'firefox' ? { 'dom.serviceWorkers.testing.enabled': true } : undefined,
    })
    context.setDefaultTimeout(PreviewOfflineGate.stepMilliseconds)
    context.setDefaultNavigationTimeout(PreviewOfflineGate.stepMilliseconds)
    try {
      const page = context.pages()[0] ?? await context.newPage()
      await PreviewOfflineGate.controlledVisit(page, name)
      await page.reload({ waitUntil: 'load' })
      const kept = await PreviewOfflineGate.waitForCachedFiles(page, name)
      PreviewOfflineGate.offlineAttempts = []
      PreviewOfflineGate.networkDown = true
      const emulated = PreviewOfflineGate.emulatedOffline.includes(name)
      if (emulated) await context.setOffline(true)
      await page.reload({ waitUntil: 'load' })
      const offlineTitle = await page.locator('#title').innerText()
      const offlineColor = await page.evaluate(() => getComputedStyle(document.getElementById('title') ?? document.body).color)
      const offlineAttempts = [...new Set(PreviewOfflineGate.offlineAttempts)]
      if (emulated) await context.setOffline(false)
      PreviewOfflineGate.networkDown = false
      assert.equal(offlineTitle, 'Kept offline', `${name} renders the page offline`)
      assert.equal(offlineColor, PreviewOfflineGate.green, `${name} applies the stylesheet offline`)
      for (const file of PreviewOfflineGate.listedFiles.filter(file => file !== PreviewOfflineGate.document)) {
        assert.ok(!offlineAttempts.includes(file), `${name} answers ${file} from the worker cache, not the network`)
      }
      return { browser: name, cached: kept.cached, offlineTitle, offlineColor, offlineAttempts }
    } finally {
      await context.close()
    }
  }

  /** @param {Page} page @param {string} name @returns {Promise<void>} */
  static async controlledVisit(page, name) {
    await page.goto(`${PreviewOfflineGate.origin}/`, { waitUntil: 'load' })
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
    }, PreviewOfflineGate.stepMilliseconds / 2)
    assert.equal(controlled, 'ready', `${name} runs the Tachyon service worker under ty preview`)
  }

  /** @param {Page} page @param {string} name @returns {Promise<CachedFiles>} */
  static async waitForCachedFiles(page, name) {
    /** @type {CachedFiles} */
    let kept = { cached: [], policies: {} }
    const deadline = Date.now() + PreviewOfflineGate.stepMilliseconds
    while (Date.now() < deadline) {
      kept = await page.evaluate(PreviewOfflineGate.readCache, PreviewOfflineGate.listedFiles)
      if (PreviewOfflineGate.listedFiles.every(file => kept.cached.includes(file))) break
      await page.waitForTimeout(200)
    }
    for (const file of PreviewOfflineGate.listedFiles) {
      assert.ok(kept.cached.includes(file), `${name} keeps ${file} in the worker cache (${kept.cached.join(' ')})`)
      assert.equal(kept.policies[file], 'no-cache', `${name} stored ${file} with the preview header`)
    }
    return kept
  }

  /** @param {string[]} files @returns {Promise<CachedFiles>} */
  static async readCache(files) {
    const name = (await caches.keys()).find(value => value.startsWith('tachyon-static-'))
    if (!name) return { cached: [], policies: {} }
    const cache = await caches.open(name)
    const cached = (await cache.keys()).map(entry => new URL(entry.url).pathname).sort()
    /** @type {Record<string, string|null>} */
    const policies = {}
    for (const file of files) {
      const stored = await cache.match(new URL(file, location.href).href)
      policies[file] = stored?.headers.get('cache-control') ?? null
    }
    return { cached, policies }
  }
}

await PreviewOfflineGate.run()

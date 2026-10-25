#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { SecureTestOrigin } from './secure-test-origin.mjs'
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { crc32, deflateSync } from 'node:zlib'
import { chromium } from 'playwright'

/** @typedef {import('playwright').BrowserContext} BrowserContext */
/** @typedef {import('playwright').Page} Page */
/** @typedef {{url: string, errors: Array<{message: string}>, data?: string}} AppManifest */
/** @typedef {{errorId: string}} InstallabilityError */
/** @typedef {{installable: string[], appId: string|null, manifestType: string, iconTypes: string[], offlineTitle: string, refused: string[]}} GateResult */

class InstallGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static origin = SecureTestOrigin.origin
  static stepMilliseconds = 20_000
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

  /** @returns {Promise<void>} */
  static async run() {
    const project = mkdtempSync(path.join(tmpdir(), 'ty-pwa-install-gate-'))
    /** @type {SecureTestOrigin|undefined} */
    let origin
    try {
      InstallGate.writeFixture(project)
      await SecureTestOrigin.bundle(InstallGate.binary, project)
      origin = await SecureTestOrigin.open(InstallGate.binary, project)
      const installable = await InstallGate.probe(project, origin)
      origin.replaced.set(InstallGate.published, { type: 'application/manifest+json', body: JSON.stringify(InstallGate.refusedManifest) })
      const refused = await InstallGate.refusedProbe(project, origin)
      origin.replaced.clear()
      /** @type {GateResult} */
      const result = { ...installable, refused }
      console.log(JSON.stringify(result, null, 2))
      console.log('PASS: PWA install browser gate')
    } finally {
      await origin?.close()
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

  /** @param {string} project @param {string} name @param {SecureTestOrigin} origin @returns {Promise<BrowserContext>} */
  static async launch(project, name, origin) {
    const context = await chromium.launchPersistentContext(mkdtempSync(path.join(project, `${name}-profile-`)), {
      ...origin.launchOptions('chromium'),
      channel: 'chromium',
      headless: true,
    })
    context.setDefaultTimeout(InstallGate.stepMilliseconds)
    context.setDefaultNavigationTimeout(InstallGate.stepMilliseconds)
    return context
  }

  /** @param {string} project @param {SecureTestOrigin} origin @returns {Promise<Omit<GateResult, 'refused'>>} */
  static async probe(project, origin) {
    const context = await InstallGate.launch(project, 'installable', origin)
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
      const offlineTitle = await InstallGate.requireOfflineStart(context, page, origin)
      return { installable, appId: appId ?? null, manifestType, iconTypes, offlineTitle }
    } finally {
      await context.close()
    }
  }

  /** @param {string} project @param {SecureTestOrigin} origin @returns {Promise<string[]>} */
  static async refusedProbe(project, origin) {
    const context = await InstallGate.launch(project, 'refused', origin)
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
    const controlled = await SecureTestOrigin.controlled(page, InstallGate.stepMilliseconds / 2)
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

  /** @param {BrowserContext} context @param {Page} page @param {SecureTestOrigin} origin @returns {Promise<string>} */
  static async requireOfflineStart(context, page, origin) {
    await page.reload({ waitUntil: 'load' })
    origin.networkDown = true
    await context.setOffline(true)
    try {
      await page.goto(`${InstallGate.origin}${InstallGate.manifest.start_url}`, { waitUntil: 'load' })
      const title = await page.locator('#title').innerText()
      assert.equal(title, 'Installed offline', 'the start URL opens offline, as an installed app does')
      return title
    } finally {
      await context.setOffline(false)
      origin.networkDown = false
    }
  }
}

await InstallGate.run()

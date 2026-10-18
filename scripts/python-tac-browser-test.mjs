#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium, firefox, webkit } from 'playwright'
import { PythonTacTestOrigin } from './python-tac-test-origin.mjs'

class PythonTacGate {
  static timeoutMilliseconds = 300_000
  static readyMilliseconds = 90_000
  static engines = ['chromium', 'firefox', 'webkit']
  static emulatedOffline = ['chromium', 'firefox']
  static assets = ['pyodide.mjs', 'pyodide.asm.mjs', 'pyodide.asm.wasm', 'python_stdlib.zip', 'pyodide-lock.json']
  static offlineFiles = [...PythonTacGate.assets.map(name => `/.tachyon/services/python/314.0.7/${name}`), '/.tachyon/services/CounterService.js', '/.tachyon/services/AuditService.js', '/.tachyon/services/LibraryService.js']

  static async run() {
    const fixture = path.resolve(PythonTacGate.required('TAC_PYTHON_FIXTURE_OUT'))
    const binary = path.resolve(PythonTacGate.required('TAC_BIN'))
    const sourceCommit = PythonTacGate.required('TAC_PYTHON_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const engines = PythonTacGate.selectedEngines()
    const run = { fixture, sourceCommit, compilerReferenceBinary: binary, compilerReferenceBinarySha256: compilerSha256 }
    await PythonTacTestOrigin.publicStatic(fixture, async site => {
      for (const engine of engines) await PythonTacGate.runEngine(engine, site, run)
    })
  }

  static selectedEngines() {
    const requested = process.env.TAC_PYTHON_BROWSERS
    if (!requested) return PythonTacGate.engines
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(PythonTacGate.engines.includes(engine), `unknown browser ${engine}`)
    return engines
  }

  static browser(engine) {
    switch (engine) {
      case 'chromium': return chromium
      case 'firefox': return firefox
      case 'webkit': return webkit
    }
  }

  static async runEngine(engine, site, run) {
    const started = performance.now()
    const diagnostics = { phase: 'starting-preview', engine, errors: [], consoleErrors: [], failedRequests: [], httpErrors: [], externalRequests: [] }
    const facts = { sourceCommit: run.sourceCommit, compilerReferenceBinary: run.compilerReferenceBinary, compilerReferenceBinarySha256: run.compilerReferenceBinarySha256, cliServedResponses: false }
    let timer
    try {
      const expired = new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`Python Tac browser fixture exceeded 300 seconds in ${engine}`)), PythonTacGate.timeoutMilliseconds)
      })
      const result = await Promise.race([PythonTacGate.serve(site, diagnostics), expired])
      await PythonTacGate.receipt(run.fixture, engine, { ...result, ...facts, elapsedMilliseconds: performance.now() - started })
      PythonTacGate.assertResult(result)
      console.log(JSON.stringify(result))
    } catch (error) {
      await PythonTacGate.receipt(run.fixture, engine, { ...diagnostics, status: 'fail', error: PythonTacGate.summary(error), ...facts, elapsedMilliseconds: performance.now() - started })
      throw error
    } finally {
      clearTimeout(timer)
    }
  }

  static required(name) {
    assert.ok(process.env[name], `${name} must explicitly identify the retained local fixture`)
    return process.env[name]
  }

  static async receipt(fixture, engine, result) {
    await writeFile(path.join(fixture, `python-tac-browser-${engine}.json`), JSON.stringify({ ...result, scope: `Python Tac ${engine} fixture: synchronous and async def services, a LocalState field and a SessionState field`, asyncMethods: true, fullPythonTacQualified: false }, null, 2) + '\n')
  }

  static async serve(site, diagnostics) {
    diagnostics.testOrigin = { origin: site.origin, transport: 'Explicit public static HTTPS test hosting of captured ordinary compiler dist bytes', cachePolicy: 'public, max-age=60; no Vary or content encoding; no source or worker rewrite', previewOfflineQualified: false, resolution: 'Test HTTP CONNECT proxy, set through Playwright, maps the test hostname to 127.0.0.1 in every engine', offline: 'The test origin refuses every request while offline; Chromium and Firefox also emulate offline', secureContext: 'Local self-signed test certificate accepted only by isolated browser; no published TLS identity qualification' }
    return PythonTacGate.probe(site, diagnostics)
  }

  static async launch(engine, proxy) {
    return PythonTacGate.browser(engine).launch({
      timeout: 30_000,
      proxy: { server: proxy },
      args: engine === 'chromium' ? ['--ignore-certificate-errors'] : [],
      firefoxUserPrefs: engine === 'firefox' ? { 'dom.serviceWorkers.testing.enabled': true } : undefined,
    })
  }

  static async probe({ origin, proxy, network }, diagnostics) {
    diagnostics.phase = 'launching-browser'
    const browser = await PythonTacGate.launch(diagnostics.engine, proxy)
    diagnostics.browserVersion = browser.version()
    try {
      const context = await browser.newContext({ serviceWorkers: 'allow', ignoreHTTPSErrors: true })
      const page = await context.newPage()
      PythonTacGate.observe(page, origin, diagnostics)
      diagnostics.phase = 'opening-python-view'
      await PythonTacGate.open(page, origin, diagnostics)
      diagnostics.phase = 'online-service-assertions'
      const online = await PythonTacGate.exercise(page)
      diagnostics.phase = 'waiting-for-pinned-cache'
      const cached = await PythonTacGate.waitForPinnedCache(page)
      diagnostics.documentCache = await page.evaluate(async () => ({ paths: (await Promise.all((await caches.keys()).map(async name => (await (await caches.open(name)).keys()).map(request => new URL(request.url).pathname)))).flat() }))
      diagnostics.phase = 'offline-reload'
      const offline = await PythonTacGate.offline(context, page, network, diagnostics)
      diagnostics.phase = 'new-tab-saved-fields'
      const newTab = await PythonTacGate.newTab(context, origin, diagnostics)
      diagnostics.phase = 'complete'
      return { ...diagnostics, status: 'pass', online, offline, cached, newTab }
    } finally {
      await browser.close()
    }
  }

  static async offline(context, page, network, diagnostics) {
    const emulated = PythonTacGate.emulatedOffline.includes(diagnostics.engine)
    network.attempts = []
    network.down = true
    try {
      if (emulated) await context.setOffline(true)
      await page.reload({ waitUntil: 'load', timeout: 20_000 })
      await page.locator('#python-ready').waitFor({ timeout: PythonTacGate.readyMilliseconds })
      assert.equal(await page.locator('#python-ready').textContent(), 'Python ready')
      diagnostics.phase = 'offline-service-assertions'
      const probe = await PythonTacGate.exercise(page)
      return { ...probe, emulated, networkAttempts: [...new Set(network.attempts)] }
    } finally {
      if (emulated) await context.setOffline(false)
      network.down = false
    }
  }

  static async newTab(context, origin, diagnostics) {
    const page = await context.newPage()
    try {
      PythonTacGate.observe(page, origin, diagnostics)
      await page.goto(origin, { waitUntil: 'load', timeout: 20_000 })
      await page.locator('#python-ready').waitFor({ timeout: PythonTacGate.readyMilliseconds })
      return { loaded: await page.locator('#view-loaded').textContent(), shelved: await page.locator('#view-shelved').textContent() }
    } finally {
      await page.close()
    }
  }

  static observe(page, origin, diagnostics) {
    page.on('pageerror', error => PythonTacGate.record(diagnostics.errors, PythonTacGate.summary(error)))
    page.on('console', message => {
      if (['error', 'warning'].includes(message.type())) PythonTacGate.record(diagnostics.consoleErrors, { type: message.type(), message: PythonTacGate.summary(message.text()) })
    })
    page.on('requestfailed', request => PythonTacGate.record(diagnostics.failedRequests, { path: PythonTacGate.safePath(request.url()), resource: request.resourceType(), error: PythonTacGate.summary(request.failure()?.errorText ?? 'unknown failure') }))
    page.on('response', response => {
      if (response.status() >= 400) PythonTacGate.record(diagnostics.httpErrors, { path: PythonTacGate.safePath(response.url()), status: response.status() })
      if (response.request().resourceType() === 'document') diagnostics.documentHeaders = { vary: response.headers()['vary'] ?? '', cacheControl: response.headers()['cache-control'] ?? '', contentEncoding: response.headers()['content-encoding'] ?? '' }
    })
    page.on('request', request => {
      const url = new URL(request.url())
      if (url.protocol.startsWith('http') && url.origin !== origin) PythonTacGate.record(diagnostics.externalRequests, PythonTacGate.safePath(url.href))
    })
  }

  static record(entries, value) {
    if (entries.length < 20) entries.push(value)
  }

  static summary(value) {
    const lines = String(value).replace(/\u001b\[[0-9;]*m/g, '').split('\n').map(line => line.trim()).filter(Boolean)
    const excerpt = lines.length <= 8 ? lines : [...lines.slice(0, 4), '[traceback middle omitted]', ...lines.slice(-4)]
    return excerpt.map(line => line.slice(0, 240)).join('\n').slice(0, 2_000) || 'unknown error'
  }

  static safePath(value) {
    const url = new URL(value)
    return url.protocol.startsWith('http') ? url.pathname.slice(0, 240) : url.protocol
  }

  static async open(page, origin, diagnostics) {
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 20_000 })
    diagnostics.pageContext = await page.evaluate(() => ({ origin: location.origin, isSecureContext, serviceWorkerAvailable: 'serviceWorker' in navigator }))
    await page.locator('#python-ready').waitFor({ timeout: PythonTacGate.readyMilliseconds })
    assert.equal(await page.locator('#python-ready').textContent(), 'Python ready')
    diagnostics.phase = 'waiting-for-worker-control'
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null, undefined, { timeout: 10_000 })
    diagnostics.phase = 'reopening-python-view'
    await page.reload({ waitUntil: 'networkidle', timeout: 20_000 })
    await page.locator('#python-ready').waitFor({ timeout: PythonTacGate.readyMilliseconds })
    assert.equal(await page.locator('#python-ready').textContent(), 'Python ready')
  }

  static async waitForPinnedCache(page) {
    const urls = PythonTacGate.assets.map(name => `/.tachyon/services/python/314.0.7/${name}`)
    return page.evaluate(async paths => {
      const deadline = performance.now() + 15_000
      while (performance.now() < deadline) {
        const names = await caches.keys()
        const matches = await Promise.all(paths.map(async asset => {
          const responses = await Promise.all(names.map(async name => (await caches.open(name)).match(asset)))
          const response = responses.find(value => value?.ok)
          return response ? { path: asset, bytes: (await response.arrayBuffer()).byteLength } : null
        }))
        if (matches.every(Boolean)) return matches
        await new Promise(resolve => setTimeout(resolve, 100))
      }
      throw new Error('The service worker did not cache every pinned Python runtime file within 15 seconds')
    }, urls)
  }

  static async exercise(page) {
    await page.locator('#view-increment').click()
    await page.waitForFunction(() => document.querySelector('#view-count')?.textContent === '4', undefined, { timeout: 5_000 })
    await page.locator('#view-echo').click()
    await page.waitForFunction(() => document.querySelector('#view-echo-count')?.textContent === '1', undefined, { timeout: 5_000 })
    assert.equal(await page.locator('#view-model').textContent(), 'initial')
    const restored = await page.locator('#view-loaded').textContent()
    const restoredSession = await page.locator('#view-shelved').textContent()
    await page.locator('#view-load').click()
    await page.waitForFunction(() => document.querySelector('#view-total')?.textContent === '5', undefined, { timeout: 5_000 })
    assert.equal(await page.locator('#view-loaded').textContent(), 'atlas')
    assert.equal(await page.locator('#view-shelved').textContent(), 'atlas')
    const storage = await PythonTacGate.savedFields(page)
    const asynchronous = await PythonTacGate.exerciseAsync(page)
    const lifetime = await page.evaluateHandle(async () => {
      const { default: CounterService } = await import('/.tachyon/services/CounterService.js')
      const { default: AuditService } = await import('/.tachyon/services/AuditService.js')
      const audit = new AuditService({})
      const first = new CounterService({ title: 'first', count: 0, fixed: 'fixed', currentGuide: null })
      const second = new CounterService({ title: 'second', count: 0, fixed: 'fixed', currentGuide: null })
      const attempt = action => {
        try { action(); return '' } catch (error) { return String(error) }
      }
      const before = { started: audit.snapshot(), first: first.count, second: second.count }
      const incremented = first.increment()
      first.count = 7
      first.currentGuide = { title: 'stored', pages: 2 }
      const model = first.echo({ title: 'typed', pages: 3 })
      const state = { first: first.count, second: second.count, stored: first.currentGuide, model, helperHidden: first.counter === undefined, fixed: first.fixed }
      const refused = {
        missingModelField: attempt(() => first.echo({ title: 'missing' })),
        booleanInteger: attempt(() => first.echo({ title: 'wrong', pages: true })),
        extraModelField: attempt(() => first.echo({ title: 'wrong', pages: 1, secret: 'must not enter' })),
        readonly: attempt(() => { first.fixed = 'changed' }),
        missingProps: attempt(() => new CounterService({ title: 'incomplete' })),
      }
      first.__tachyonDispose()
      const retained = { lifecycle: audit.snapshot(), secondIncremented: second.increment() }
      refused.disposed = attempt(() => first.increment())
      first.__tachyonDispose()
      second.__tachyonDispose()
      return { audit, result: { before, incremented, state, refused, retained } }
    })
    try {
      await page.locator('#view-hide').click()
      await page.locator('[data-tachyon-component="python-probe"]').waitFor({ state: 'detached', timeout: 5_000 })
      const managed = await lifetime.evaluate(async ({ audit, result }) => {
        try {
          const deadline = performance.now() + 5_000
          while (audit.snapshot()[1] !== 1) {
            if (performance.now() >= deadline) throw new Error('Managed conditional unmount did not release the last shared helper')
            await new Promise(resolve => setTimeout(resolve, 10))
          }
          return { ...result, stopped: audit.snapshot(), teardown: 'managed conditional unmount via Python service event' }
        } finally {
          audit.__tachyonDispose()
        }
      })
      return { ...managed, asynchronous, restored, restoredSession, storage }
    } finally {
      await lifetime.dispose()
    }
  }

  static async savedFields(page) {
    return page.evaluate(() => {
      const fields = store => Object.fromEntries(Object.keys(store).filter(key => /(loaded|shelved)$/.test(key)).map(key => [key.endsWith('loaded') ? 'loaded' : 'shelved', JSON.parse(store.getItem(key) ?? 'null')]))
      return { local: fields(localStorage), session: fields(sessionStorage) }
    })
  }

  static async exerciseAsync(page) {
    return page.evaluate(async () => {
      const { default: LibraryService } = await import('/.tachyon/services/LibraryService.js')
      const library = new LibraryService({})
      const before = library.total
      const pending = library.load('globe')
      const isPromise = typeof pending?.then === 'function'
      const meanwhile = library.total - before
      const loaded = await pending
      const after = { added: library.total - before, loaded: library.loaded }
      const failed = await library.fail().then(() => '', error => String(error.message))
      const holding = library.hold()
      library.__tachyonDispose()
      const cancelled = await holding.then(() => '', error => error.code)
      return { isPromise, meanwhile, loaded, after, failed, cancelled }
    })
  }

  static assertResult(result) {
    assert.equal(result.status, 'pass')
    assert.deepEqual(result.errors, [])
    assert.deepEqual(result.externalRequests, [])
    assert.deepEqual(result.cached.map(asset => asset.bytes), [17_931, 1_250_344, 9_598_218, 2_545_637, 119_077])
    assert.equal(result.online.restored, 'waiting')
    assert.equal(result.offline.restored, 'atlas')
    assert.equal(result.online.restoredSession, 'none')
    assert.equal(result.offline.restoredSession, 'atlas')
    assert.deepEqual(result.offline.networkAttempts.filter(file => PythonTacGate.offlineFiles.includes(file)), [])
    assert.deepEqual(result.newTab, { loaded: 'atlas', shelved: 'none' })
    for (const probe of [result.online, result.offline]) {
      assert.deepEqual(probe.before, { started: [1, 0], first: 1, second: 1 })
      assert.equal(probe.incremented, 4)
      assert.deepEqual(probe.state, { first: 7, second: 1, stored: { title: 'stored', pages: 2 }, model: { title: 'typed', pages: 3 }, helperHidden: true, fixed: 'fixed' })
      assert.ok(Object.values(probe.refused).every(Boolean), JSON.stringify(probe.refused))
      assert.deepEqual(probe.retained, { lifecycle: [1, 0], secondIncremented: 4 })
      assert.deepEqual(probe.stopped, [1, 1])
      assert.equal(probe.teardown, 'managed conditional unmount via Python service event')
      assert.deepEqual(probe.storage, { local: { loaded: 'atlas' }, session: { shelved: 'atlas' } })
      assert.deepEqual(probe.asynchronous, { isPromise: true, meanwhile: 0, loaded: { title: 'globe', pages: 5 }, after: { added: 5, loaded: 'globe' }, failed: 'Web service library: Python Tac operation failed.', cancelled: 'CANCELLED' })
    }
  }
}

await PythonTacGate.run()

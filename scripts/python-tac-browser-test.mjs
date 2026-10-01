#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'
import { PythonTacTestOrigin } from './python-tac-test-origin.mjs'

class PythonTacGate {
  static timeoutMilliseconds = 45_000
  static host = 'python-tac.test'
  static assets = ['pyodide.mjs', 'pyodide.asm.mjs', 'pyodide.asm.wasm', 'python_stdlib.zip', 'pyodide-lock.json']

  static async run() {
    const fixture = path.resolve(PythonTacGate.required('TAC_PYTHON_FIXTURE_OUT'))
    const binary = path.resolve(PythonTacGate.required('TAC_BIN'))
    const sourceCommit = PythonTacGate.required('TAC_PYTHON_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const started = performance.now()
    const diagnostics = { phase: 'starting-preview', errors: [], consoleErrors: [], failedRequests: [], httpErrors: [], externalRequests: [] }
    let timer
    try {
      const expired = new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error('Python Tac browser fixture exceeded 45 seconds')), PythonTacGate.timeoutMilliseconds)
      })
      const result = await Promise.race([PythonTacGate.serve(binary, fixture, diagnostics), expired])
      await PythonTacGate.receipt(fixture, { ...result, sourceCommit, serverBinary: binary, serverBinarySha256: compilerSha256, elapsedMilliseconds: performance.now() - started })
      PythonTacGate.assertResult(result)
      console.log(JSON.stringify(result))
    } catch (error) {
      await PythonTacGate.receipt(fixture, { ...diagnostics, status: 'fail', error: PythonTacGate.summary(error), sourceCommit, serverBinary: binary, serverBinarySha256: compilerSha256, elapsedMilliseconds: performance.now() - started })
      throw error
    } finally {
      clearTimeout(timer)
    }
  }

  static required(name) {
    assert.ok(process.env[name], `${name} must explicitly identify the retained local fixture`)
    return process.env[name]
  }

  static async receipt(fixture, result) {
    await writeFile(path.join(fixture, 'python-tac-browser.json'), JSON.stringify({ ...result, scope: 'initial synchronous Python Tac Chromium fixture', asynchronousQualified: false, fullPythonTacQualified: false }, null, 2) + '\n')
  }

  static async serve(binary, fixture, diagnostics) {
    let result
    await withServer(binary, ['preview', fixture, '--static', '--out-dir', 'dist', '--host', '127.0.0.1', '--port', '0', '--no-self-restart'], async request => {
      const upstream = new URL((await request('/')).url).origin
      result = await PythonTacTestOrigin.run(upstream, fixture, async origin => {
        diagnostics.testOrigin = { origin, transport: 'HTTPS loopback proxy to ordinary ty preview; unmodified response bytes and headers', resolution: 'Chromium-only hostname mapping to 127.0.0.1', secureContext: 'Local self-signed test certificate accepted only by isolated browser; no published TLS identity qualification' }
        return PythonTacGate.probe(origin, diagnostics)
      })
    })
    return result
  }

  static async probe(origin, diagnostics) {
    diagnostics.phase = 'launching-browser'
    const browser = await chromium.launch({ timeout: 10_000, args: [`--host-resolver-rules=MAP ${PythonTacGate.host} 127.0.0.1`, '--ignore-certificate-errors', '--no-proxy-server'] })
    diagnostics.chromiumVersion = browser.version()
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
      diagnostics.phase = 'offline-reload'
      await context.setOffline(true)
      await page.reload({ waitUntil: 'networkidle', timeout: 20_000 })
      await page.locator('#python-ready').waitFor({ timeout: 15_000 })
      assert.equal(await page.locator('#python-ready').textContent(), 'Python ready')
      diagnostics.phase = 'offline-service-assertions'
      const offline = await PythonTacGate.exercise(page)
      await context.setOffline(false)
      diagnostics.phase = 'complete'
      return { ...diagnostics, status: 'pass', online, offline, cached }
    } finally {
      await browser.close()
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
    await page.locator('#python-ready').waitFor({ timeout: 15_000 })
    assert.equal(await page.locator('#python-ready').textContent(), 'Python ready')
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null, undefined, { timeout: 10_000 })
  }

  static async waitForPinnedCache(page) {
    const urls = PythonTacGate.assets.map(name => `/.tachyon/services/python/314.0.7/${name}`)
    await page.waitForFunction(async paths => {
      const names = await caches.keys()
      const matches = await Promise.all(paths.map(async asset => {
        const responses = await Promise.all(names.map(async name => (await caches.open(name)).match(asset)))
        return responses.some(response => response?.ok)
      }))
      return matches.every(Boolean)
    }, urls, { timeout: 15_000 })
    return page.evaluate(async paths => {
      const result = []
      const names = await caches.keys()
      for (const asset of paths) {
        const responses = await Promise.all(names.map(async name => (await caches.open(name)).match(asset)))
        const response = responses.find(value => value?.ok)
        result.push({ path: asset, bytes: (await response.arrayBuffer()).byteLength })
      }
      return result
    }, urls)
  }

  static async exercise(page) {
    await page.locator('#view-increment').click()
    await page.waitForFunction(() => document.querySelector('#view-count')?.textContent === '4', undefined, { timeout: 5_000 })
    await page.locator('#view-echo').click()
    await page.waitForFunction(() => document.querySelector('#view-echo-count')?.textContent === '1', undefined, { timeout: 5_000 })
    assert.equal(await page.locator('#view-model').textContent(), 'initial')
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
      return await lifetime.evaluate(async ({ audit, result }) => {
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
    } finally {
      await lifetime.dispose()
    }
  }

  static assertResult(result) {
    assert.equal(result.status, 'pass')
    assert.deepEqual(result.errors, [])
    assert.deepEqual(result.externalRequests, [])
    assert.deepEqual(result.cached.map(asset => asset.bytes), [17_931, 1_250_344, 9_598_218, 2_545_637, 119_077])
    for (const probe of [result.online, result.offline]) {
      assert.deepEqual(probe.before, { started: [1, 0], first: 1, second: 1 })
      assert.equal(probe.incremented, 4)
      assert.deepEqual(probe.state, { first: 7, second: 1, stored: { title: 'stored', pages: 2 }, model: { title: 'typed', pages: 3 }, helperHidden: true, fixed: 'fixed' })
      assert.ok(Object.values(probe.refused).every(Boolean), JSON.stringify(probe.refused))
      assert.deepEqual(probe.retained, { lifecycle: [1, 0], secondIncremented: 4 })
      assert.deepEqual(probe.stopped, [1, 1])
      assert.equal(probe.teardown, 'managed conditional unmount via Python service event')
    }
  }
}

await PythonTacGate.run()

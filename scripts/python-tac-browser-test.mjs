#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class PythonTacGate {
  static timeoutMilliseconds = 45_000
  static assets = ['pyodide.mjs', 'pyodide.asm.mjs', 'pyodide.asm.wasm', 'python_stdlib.zip', 'pyodide-lock.json']

  static async run() {
    const fixture = path.resolve(PythonTacGate.required('TAC_PYTHON_FIXTURE_OUT'))
    const binary = path.resolve(PythonTacGate.required('TAC_BIN'))
    const sourceCommit = PythonTacGate.required('TAC_PYTHON_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const started = performance.now()
    let timer
    try {
      const expired = new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error('Python Tac browser fixture exceeded 45 seconds')), PythonTacGate.timeoutMilliseconds)
      })
      const result = await Promise.race([PythonTacGate.serve(binary, fixture), expired])
      await PythonTacGate.receipt(fixture, { ...result, sourceCommit, serverBinary: binary, serverBinarySha256: compilerSha256, elapsedMilliseconds: performance.now() - started })
      PythonTacGate.assertResult(result)
      console.log(JSON.stringify(result))
    } catch (error) {
      await PythonTacGate.receipt(fixture, { status: 'fail', error: String(error), sourceCommit, serverBinary: binary, serverBinarySha256: compilerSha256, elapsedMilliseconds: performance.now() - started })
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

  static async serve(binary, fixture) {
    let result
    await withServer(binary, ['preview', fixture, '--static', '--out-dir', 'dist', '--host', '127.0.0.1', '--port', '0', '--no-self-restart'], async request => {
      const origin = new URL((await request('/')).url).origin
      result = await PythonTacGate.probe(origin)
    })
    return result
  }

  static async probe(origin) {
    const browser = await chromium.launch({ timeout: 10_000 })
    try {
      const context = await browser.newContext({ serviceWorkers: 'allow' })
      const page = await context.newPage()
      const errors = []
      const externalRequests = []
      page.on('pageerror', error => errors.push(String(error)))
      page.on('request', request => {
        const url = new URL(request.url())
        if (url.protocol.startsWith('http') && url.origin !== origin) externalRequests.push(url.href)
      })
      await PythonTacGate.open(page, origin)
      const online = await PythonTacGate.exercise(page)
      const cached = await PythonTacGate.waitForPinnedCache(page)
      await context.setOffline(true)
      await page.reload({ waitUntil: 'networkidle', timeout: 20_000 })
      await page.locator('#python-ready').waitFor({ timeout: 15_000 })
      assert.equal(await page.locator('#python-ready').textContent(), 'Python ready')
      const offline = await PythonTacGate.exercise(page)
      await context.setOffline(false)
      return { status: 'pass', chromiumVersion: browser.version(), online, offline, cached, errors, externalRequests }
    } finally {
      await browser.close()
    }
  }

  static async open(page, origin) {
    await page.goto(origin, { waitUntil: 'networkidle', timeout: 20_000 })
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
    return page.evaluate(async () => {
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
      const stopped = audit.snapshot()
      audit.__tachyonDispose()
      return { before, incremented, state, refused, retained, stopped }
    })
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
    }
  }
}

await PythonTacGate.run()

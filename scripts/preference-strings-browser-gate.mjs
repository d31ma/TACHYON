#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class PreferenceStringsBrowserGate {
  static async run() {
    const root = path.resolve(PreferenceStringsBrowserGate.required('TAC_PREFERENCE_FIXTURE_OUT'))
    const binary = path.resolve(PreferenceStringsBrowserGate.required('TAC_BIN'))
    const sourceCommit = PreferenceStringsBrowserGate.required('TAC_PREFERENCE_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const diagnostics = { phase: 'starting-preview', sourceCommit, compilerSha256, errors: [], failedRequests: [], externalRequests: [] }
    const started = performance.now()
    try {
      const result = await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return PreferenceStringsBrowserGate.probe(origin, diagnostics)
      })
      await PreferenceStringsBrowserGate.receipt(root, { ...result, status: 'pass', elapsedMilliseconds: performance.now() - started })
      console.log(JSON.stringify(result))
    } catch (error) {
      await PreferenceStringsBrowserGate.receipt(root, { ...diagnostics, status: 'fail', error: String(error).slice(0, 2_000), elapsedMilliseconds: performance.now() - started })
      throw error
    }
  }

  static required(name) {
    assert.ok(process.env[name], `${name} must identify the explicit retained fixture`)
    return process.env[name]
  }

  static async receipt(root, result) {
    await writeFile(path.join(root, 'preference-strings-browser.json'), JSON.stringify({ ...result, scope: 'ordinary production Java TeaVM adapter/table through Chromium', denial: 'SecurityError fault injected at browser Storage.prototype; host transport unchanged', internalRetainedJavaProvider: 'separately qualified by opt-in JDK ownership test', allLanguageParityQualified: false }, null, 2) + '\n')
  }

  static async probe(origin, diagnostics) {
    const browser = await chromium.launch({ timeout: 10_000 })
    diagnostics.chromiumVersion = browser.version()
    try {
      const context = await browser.newContext()
      await context.addInitScript(() => {
        if (localStorage.getItem('tachyon.platform') === null) localStorage.setItem('tachyon.platform', 'macOS')
      })
      const page = await context.newPage()
      PreferenceStringsBrowserGate.observe(page, origin, diagnostics)
      diagnostics.phase = 'ordinary-view-binding'
      await page.goto(origin, { waitUntil: 'networkidle', timeout: 15_000 })
      await page.waitForFunction(() => document.querySelector('#preference-value')?.textContent === 'macOS', undefined, { timeout: 10_000 })
      await page.locator('#preference-write').click()
      await page.waitForFunction(() => document.querySelector('#preference-value')?.textContent === 'Linux', undefined, { timeout: 5_000 })
      diagnostics.phase = 'compiled-java-activation'
      const activation = await PreferenceStringsBrowserGate.activation(page)
      PreferenceStringsBrowserGate.checkActivation(activation)
      diagnostics.phase = 'raw-reload'
      await page.reload({ waitUntil: 'networkidle', timeout: 15_000 })
      await page.waitForFunction(() => document.querySelector('#preference-value')?.textContent === 'Windows', undefined, { timeout: 5_000 })
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      return { ...diagnostics, activation, reloadRawValue: await page.locator('#preference-value').textContent() }
    } finally {
      await browser.close()
    }
  }

  static observe(page, origin, diagnostics) {
    page.on('pageerror', error => PreferenceStringsBrowserGate.record(diagnostics.errors, error.message.slice(0, 2_000)))
    page.on('requestfailed', request => PreferenceStringsBrowserGate.record(diagnostics.failedRequests, { path: new URL(request.url()).pathname, error: request.failure()?.errorText }))
    page.on('request', request => {
      const url = new URL(request.url())
      if (url.protocol.startsWith('http') && url.origin !== origin) PreferenceStringsBrowserGate.record(diagnostics.externalRequests, url.pathname.slice(0, 240))
    })
    page.on('response', response => {
      if (response.status() >= 400) PreferenceStringsBrowserGate.record(diagnostics.errors, `HTTP ${response.status()} ${new URL(response.url()).pathname}`)
    })
  }

  static record(entries, value) {
    if (entries.length < 20) entries.push(value)
  }

  static async activation(page) {
    return page.evaluate(async () => {
      const { default: ChoiceService } = await import('/.tachyon/services/ChoiceService.js')
      const first = new ChoiceService({})
      const originals = { get: Storage.prototype.getItem, set: Storage.prototype.setItem }
      const counters = { reads: 0, writes: 0 }
      let denied = false
      Storage.prototype.getItem = function (key) {
        counters.reads++
        if (denied) throw new DOMException('Fixture storage denied', 'SecurityError')
        return originals.get.call(this, key)
      }
      Storage.prototype.setItem = function (key, value) {
        counters.writes++
        if (denied) throw new DOMException('Fixture storage denied', 'SecurityError')
        return originals.set.call(this, key, value)
      }
      const grant = document.querySelector('meta[name="tachyon-browser-apis"]')
      const granted = grant?.content ?? ''
      let second
      let firstDisposed = false
      try {
        assertGrant(grant)
        const raw = { written: first.write('tachyon.platform', 'macOS'), read: first.read('tachyon.platform'), stored: originals.get.call(localStorage, 'tachyon.platform') }
        first.write('tachyon.preference.empty', '')
        first.write('tachyon.preference.unicode', '雪🌍')
        const values = { empty: first.read('tachyon.preference.empty'), unicode: first.read('tachyon.preference.unicode') }
        grant.content = ''
        const beforeMissing = { ...counters }
        const missing = { read: first.read('tachyon.platform'), written: first.write('tachyon.platform', 'forbidden'), before: beforeMissing, after: { ...counters } }
        grant.content = granted
        denied = true
        const refusal = { read: first.read('tachyon.platform'), written: first.write('tachyon.platform', 'denied') }
        denied = false
        const beforeBounds = { ...counters }
        const bounds = { emptyKey: first.write('', 'bad'), oversizedValue: first.write('tachyon.platform', 'x'.repeat(4097)), oversizedKey: first.read('x'.repeat(129)), before: beforeBounds, after: { ...counters } }
        const retainedRead = () => first.read('tachyon.platform')
        const retainedWrite = () => first.write('tachyon.platform', 'stale')
        first.__tachyonDispose()
        firstDisposed = true
        second = new ChoiceService({})
        const beforeDisposed = { ...counters }
        const disposed = { read: failure(retainedRead), write: failure(retainedWrite), before: beforeDisposed, after: { ...counters }, freshWrite: second.write('tachyon.platform', 'Windows'), freshRead: second.read('tachyon.platform'), oldReadAfterNew: failure(retainedRead) }
        return { raw, values, missing, refusal, bounds, disposed, hiddenProvider: first.preferences === undefined }
      } finally {
        if (grant) grant.content = granted
        Storage.prototype.getItem = originals.get
        Storage.prototype.setItem = originals.set
        if (!firstDisposed) first.__tachyonDispose()
        second?.__tachyonDispose()
      }
      function failure(action) {
        try { action(); return '' } catch (error) { return String(error) }
      }
      function assertGrant(value) {
        if (!value?.content.split(' ').includes('preferenceStrings')) throw new Error('Ordinary manifest grant is missing')
      }
    })
  }

  static checkActivation(value) {
    assert.deepEqual(value.raw, { written: true, read: 'macOS', stored: 'macOS' })
    assert.deepEqual(value.values, { empty: '', unicode: '雪🌍' })
    assert.equal(value.missing.read, null)
    assert.equal(value.missing.written, false)
    assert.deepEqual(value.missing.before, value.missing.after)
    assert.deepEqual(value.refusal, { read: null, written: false })
    assert.equal(value.bounds.emptyKey, false)
    assert.equal(value.bounds.oversizedValue, false)
    assert.equal(value.bounds.oversizedKey, null)
    assert.deepEqual(value.bounds.before, value.bounds.after)
    assert.match(value.disposed.read, /disposed|stale/i)
    assert.match(value.disposed.write, /disposed|stale/i)
    assert.match(value.disposed.oldReadAfterNew, /disposed|stale/i)
    assert.deepEqual(value.disposed.before, value.disposed.after)
    assert.equal(value.disposed.freshWrite, true)
    assert.equal(value.disposed.freshRead, 'Windows')
    assert.equal(value.hiddenProvider, true)
  }
}

await PreferenceStringsBrowserGate.run()

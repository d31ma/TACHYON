#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class DocumentTitleBrowserGate {
  static async run() {
    const root = path.resolve(DocumentTitleBrowserGate.required('TAC_DOCUMENT_TITLE_FIXTURE_OUT'))
    const binary = path.resolve(DocumentTitleBrowserGate.required('TAC_BIN'))
    const sourceCommit = DocumentTitleBrowserGate.required('TAC_DOCUMENT_TITLE_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const diagnostics = { phase: 'starting-preview', sourceCommit, fixtureSourceCommit: process.env.TAC_DOCUMENT_TITLE_FIXTURE_SOURCE_COMMIT ?? 'not supplied', compilerSha256, errors: [], failedRequests: [], externalRequests: [] }
    const started = performance.now()
    try {
      const result = await withServer(binary, ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return DocumentTitleBrowserGate.probe(origin, diagnostics)
      })
      await DocumentTitleBrowserGate.receipt(root, { ...result, status: 'pass', elapsedMilliseconds: performance.now() - started })
      console.log(JSON.stringify(result))
    } catch (error) {
      await DocumentTitleBrowserGate.receipt(root, { ...diagnostics, status: 'fail', error: String(error).slice(0, 2000), elapsedMilliseconds: performance.now() - started })
      throw error
    }
  }

  static required(name) {
    assert.ok(process.env[name], `${name} must identify the explicit retained fixture`)
    return process.env[name]
  }

  static async receipt(root, result) {
    await writeFile(path.join(root, 'document-title-browser.json'), JSON.stringify({ ...result, scope: 'ordinary compiled Kotlin/Wasm facade and actual managed TypeScript Service through Chromium', fakeDocumentUsed: false, adapterReplaced: false, privateProviderLease: 'separate Kotlin/JVM scope and native Test gates; browser stale facade refusal is a different boundary', sameIdReuse: 'actual generated Kotlin entry protocol; previous private provider is not retrieved or exposed', scriptPrivacy: 'authored TypeScript private declaration; no claim that JavaScript reflection cannot inspect erased TS fields', websiteFeatureMigrated: false }, null, 2) + '\n')
  }

  static async probe(origin, diagnostics) {
    const browser = await chromium.launch({ timeout: 10000 })
    diagnostics.chromiumVersion = browser.version()
    try {
      const page = await browser.newPage()
      DocumentTitleBrowserGate.observe(page, origin, diagnostics)
      diagnostics.phase = 'ordinary-managed-view'
      await page.goto(origin, { waitUntil: 'networkidle', timeout: 15000 })
      await DocumentTitleBrowserGate.title(page, 'Kotlin first — 🚀')
      const managed = await DocumentTitleBrowserGate.managed(page, diagnostics)
      diagnostics.phase = 'compiled-kotlin-facade'
      const facade = await DocumentTitleBrowserGate.facade(page)
      DocumentTitleBrowserGate.checkFacade(facade)
      diagnostics.phase = 'compiled-owner-reuse-and-startup-failure'
      const reuse = await DocumentTitleBrowserGate.reuse(page)
      DocumentTitleBrowserGate.checkReuse(reuse)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      assert.deepEqual(diagnostics.externalRequests, [])
      diagnostics.phase = 'complete'
      return { ...diagnostics, managed, facade, reuse }
    } finally {
      await browser.close()
    }
  }

  static async title(page, expected) {
    try {
      await page.waitForFunction(value => document.title === value, expected, { timeout: 5000 })
      assert.equal(await page.title(), expected)
    } catch (error) {
      const snapshot = await page.evaluate(() => ({
        title: document.title,
        kotlinLabels: [...document.querySelectorAll('.kotlin-label')].slice(0, 4).map(node => node.textContent?.slice(0, 256)),
        mounted: ['kotlin-first-container', 'kotlin-second-container', 'script-container'].filter(id => document.getElementById(id)),
        scriptWrites: document.querySelector('#script-ready')?.textContent?.slice(0, 64),
        components: [...document.querySelectorAll('[data-tachyon-component]')].slice(0, 8).map(node => ({ name: node.dataset.tachyonComponent, active: node.dataset.tachyonActive, mountError: node.dataset.tachyonMountError })),
      }))
      throw new Error(`Expected title ${JSON.stringify(expected)}; actual fixture ${JSON.stringify(snapshot)}; ${String(error)}`)
    }
  }

  static async managed(page, diagnostics) {
    await page.locator('#kotlin-first-container .kotlin-write').click()
    await DocumentTitleBrowserGate.title(page, 'Kotlin click — 🌍')
    await page.locator('#kotlin-first-container .kotlin-attempt').click()
    await DocumentTitleBrowserGate.title(page, 'Kotlin attempt — 雪')
    await page.locator('#kotlin-first-container .kotlin-empty').click()
    await DocumentTitleBrowserGate.title(page, '')
    await page.locator('#show-second').click()
    await page.locator('#kotlin-second-container').waitFor({ timeout: 5000 })
    await DocumentTitleBrowserGate.title(page, 'Kotlin second — 雪')
    await page.locator('#hide-first').click()
    await page.locator('#kotlin-first-container').waitFor({ state: 'detached', timeout: 5000 })
    await DocumentTitleBrowserGate.title(page, 'Kotlin second — 雪')
    const trace = await DocumentTitleBrowserGate.titleTrace(page)
    try {
      await page.locator('#show-first').click()
      await page.locator('#kotlin-first-container').waitFor({ timeout: 5000 })
      await DocumentTitleBrowserGate.title(page, 'Kotlin first — 🚀')
    } finally {
      diagnostics.remountTitleWrites = await trace.evaluate(value => value.finish())
      await trace.dispose()
    }
    await page.locator('#show-script').click()
    await page.locator('#script-ready').waitFor({ timeout: 5000 })
    await DocumentTitleBrowserGate.title(page, 'Script start — 🦊')
    await page.locator('#script-write').click()
    await page.waitForFunction(() => document.querySelector('#script-ready')?.textContent === '1', undefined, { timeout: 5000 })
    await DocumentTitleBrowserGate.title(page, 'Script click — 🦊')
    await page.locator('#script-attempt').click()
    await DocumentTitleBrowserGate.title(page, 'Script attempt — 雪')
    await page.locator('#script-invalid').click()
    await DocumentTitleBrowserGate.title(page, 'Script attempt — 雪')
    await page.locator('#script-empty').click()
    await DocumentTitleBrowserGate.title(page, '')
    const retiredButton = await page.locator('#script-write').elementHandle()
    assert.ok(retiredButton, 'actual mounted script button')
    try {
      await page.locator('#hide-script').click()
      await page.locator('#script-container').waitFor({ state: 'detached', timeout: 5000 })
      await DocumentTitleBrowserGate.title(page, '')
      await retiredButton.evaluate(button => button.click())
      await DocumentTitleBrowserGate.title(page, '')
    } finally {
      await retiredButton.dispose()
    }
    await page.locator('#show-script').click()
    await page.waitForFunction(() => document.querySelector('#script-ready')?.textContent === '0', undefined, { timeout: 5000 })
    await DocumentTitleBrowserGate.title(page, 'Script start — 🦊')
    return { kotlinUnicode: true, kotlinAttemptUiWrite: true, scriptAttemptUiWrite: true, empty: true, independentMountedOwners: true, firstDisposalDoesNotRestoreTitle: true, kotlinRemount: true, scriptUnicode: true, scriptOversizedRefused: true, scriptRetiredDomListenerInactive: true, scriptRemount: true }
  }

  static async titleTrace(page) {
    return page.evaluateHandle(() => {
      const entries = []
      const observer = new MutationObserver(records => {
        for (const record of records) {
          const element = record.target.nodeType === Node.TEXT_NODE ? record.target.parentElement : record.target
          if (element?.closest?.('title') && entries.length < 12) {
            entries.push({ old: record.oldValue ?? [...record.removedNodes].map(node => node.textContent).join(''), added: [...record.addedNodes].map(node => node.textContent).join(''), current: document.title })
          }
        }
      })
      observer.observe(document.head, { subtree: true, childList: true, characterData: true, characterDataOldValue: true })
      return { finish() { observer.disconnect(); return entries } }
    })
  }

  static async facade(page) {
    return page.evaluate(async () => {
      const { default: KotlinTitleService } = await import('/.tachyon/services/KotlinTitleService.js')
      const first = new KotlinTitleService({ label: 'Facade first — 🌍', fails: false })
      const firstStart = document.title
      const second = new KotlinTitleService({ label: 'Facade second — 雪', fails: false })
      const secondStart = document.title
      let firstDisposed = false
      try {
        const privateProvider = !('documentTitle' in first) && !Object.keys(first).includes('documentTitle')
        const bounds = []
        for (const text of ['é'.repeat(513), '\u0000', '\ud800', '\udc00']) {
          const before = document.title
          bounds.push({ accepted: second.attempt(text), unchanged: document.title === before })
        }
        const exact = second.attempt('é'.repeat(512))
        const exactTitle = document.title
        const empty = second.attempt('')
        const emptyTitle = document.title
        second.write('current-owner')
        const retainedWrite = first.write
        first.__tachyonDispose()
        firstDisposed = true
        const afterDisposal = document.title
        let stale = ''
        try { retainedWrite('stale') } catch (error) { stale = String(error) }
        const afterStale = document.title
        second.write('newer-owner — 🦊')
        return { firstStart, secondStart, privateProvider, bounds, exact, exactTitle, empty, emptyTitle, afterDisposal, stale, afterStale, secondStillWorks: document.title }
      } finally {
        if (!firstDisposed) first.__tachyonDispose()
        second.__tachyonDispose()
      }
    })
  }

  static checkFacade(value) {
    assert.equal(value.firstStart, 'Facade first — 🌍')
    assert.equal(value.secondStart, 'Facade second — 雪')
    assert.equal(value.privateProvider, true)
    assert.deepEqual(value.bounds, Array.from({ length: 4 }, () => ({ accepted: false, unchanged: true })))
    assert.equal(value.exact, true)
    assert.equal(value.exactTitle, 'é'.repeat(512))
    assert.equal(value.empty, true)
    assert.equal(value.emptyTitle, '')
    assert.equal(value.afterDisposal, 'current-owner')
    assert.match(value.stale, /stale|disposed/i)
    assert.equal(value.afterStale, 'current-owner')
    assert.equal(value.secondStillWorks, 'newer-owner — 🦊')
  }

  static async reuse(page) {
    return page.evaluate(async () => {
      const kotlin = await import('/.tachyon/services/kotlin/services.js')
      const owner = '86cf44d0-cc07-4132-aa86-a8d870eddf4e'
      const request = payload => JSON.parse(kotlin.tacInvoke(JSON.stringify({ route: 'web', service: 'kotlinTitle', owner, ...payload })))
      const initial = request({ op: 'init', props: { label: 'Fixed owner first', fails: false } })
      const initialTitle = document.title
      request({ op: 'dispose' })
      const failed = request({ op: 'init', props: { label: 'Failed startup write', fails: true } })
      const failureTitle = document.title
      const replacement = request({ op: 'init', props: { label: 'Fixed owner replacement — 🌍', fails: false } })
      const replacementTitle = document.title
      try {
        const write = request({ op: 'call', name: 'write', args: ['Fixed owner final'] })
        return { initialError: initial.error ?? '', initialTitle, failedError: failed.error ?? '', failureTitle, replacementError: replacement.error ?? '', replacementTitle, writeError: write.error ?? '', finalTitle: document.title, hiddenProvider: !replacement.value?.fields?.includes('documentTitle') && !replacement.value?.methods?.includes('documentTitle') }
      } finally {
        request({ op: 'dispose' })
      }
    })
  }

  static checkReuse(value) {
    assert.equal(value.initialError, '')
    assert.equal(value.initialTitle, 'Fixed owner first')
    assert.notEqual(value.failedError, '')
    assert.equal(value.failureTitle, 'Failed startup write', 'startup failure retires scope without restoring title')
    assert.equal(value.replacementError, '')
    assert.equal(value.replacementTitle, 'Fixed owner replacement — 🌍')
    assert.equal(value.writeError, '')
    assert.equal(value.finalTitle, 'Fixed owner final')
    assert.equal(value.hiddenProvider, true)
  }

  static observe(page, origin, diagnostics) {
    page.on('pageerror', error => DocumentTitleBrowserGate.record(diagnostics.errors, error.message.slice(0, 2000)))
    page.on('requestfailed', request => DocumentTitleBrowserGate.record(diagnostics.failedRequests, { path: new URL(request.url()).pathname, error: request.failure()?.errorText }))
    page.on('request', request => {
      const url = new URL(request.url())
      if (url.protocol.startsWith('http') && url.origin !== origin) DocumentTitleBrowserGate.record(diagnostics.externalRequests, url.pathname.slice(0, 240))
    })
    page.on('response', response => {
      if (response.status() >= 400) DocumentTitleBrowserGate.record(diagnostics.errors, `HTTP ${response.status()} ${new URL(response.url()).pathname}`)
    })
  }

  static record(entries, value) {
    if (entries.length < 20) entries.push(value)
  }
}

await DocumentTitleBrowserGate.run()

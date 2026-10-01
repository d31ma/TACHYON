#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cpSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'

class BrowserHttpGate {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = fileURLToPath(new URL('../website/content/docs/calling-the-server/browser-http-dart', import.meta.url))
  static launchers = { chromium, firefox, webkit }

  static build(project) {
    const result = spawnSync(BrowserHttpGate.binary, ['bundle', project, '--no-incremental'], {
      encoding: 'utf8', timeout: 240000, maxBuffer: 4 * 1024 * 1024,
    })
    assert.equal(result.status, 0, result.error?.message ?? result.stderr ?? result.stdout)
  }

  static async check(launcher, origin) {
    const browser = await launcher.launch()
    try {
      const context = await browser.newContext()
      const page = await context.newPage()
      const errors = []
      const requests = []
      page.on('pageerror', error => errors.push(error.message))
      page.on('request', request => { if (request.url().includes('/api/')) requests.push(request) })
      await context.addCookies([{ name: 'private_session', value: 'must-not-travel', url: origin }])
      await page.addInitScript(() => {
        globalThis.httpCalls = []
        globalThis.httpTicks = 0
        const original = globalThis.fetch
        globalThis.fetch = (input, options) => {
          if (String(input).includes('/api/')) globalThis.httpCalls.push({ url: String(input), cache: options?.cache, credentials: options?.credentials })
          return original(input, options)
        }
        setInterval(() => { globalThis.httpTicks += 1 }, 10)
      })
      await page.goto(origin)
      await page.locator('#ask').waitFor()
      const before = await page.evaluate(() => globalThis.httpTicks)
      await page.locator('#ask').click()
      await page.waitForFunction(() => document.querySelector('#message')?.textContent === 'Hello from Yon')
      assert.equal(await page.locator('#loads').textContent(), '1', 'service side effects run once')
      assert.ok(await page.evaluate(() => globalThis.httpTicks) - before >= 5, 'the page continues running during Yon delay')
      assert.equal(requests.filter(request => request.url().endsWith('/api/greeting')).length, 1)
      assert.equal((await requests[0].allHeaders()).cookie, undefined, 'same-origin session cookies omitted')
      await page.locator('#ask').click()
      await page.waitForFunction(() => document.querySelector('#loads')?.textContent === '2')
      assert.equal(requests.filter(request => request.url().endsWith('/api/greeting')).length, 1, 'cached typed answer avoids HTTP')
      await page.locator('#clear').click()
      await page.waitForFunction(() => document.querySelector('#clears')?.textContent === '1')
      await page.locator('#ask').click()
      await page.waitForFunction(() => document.querySelector('#loads')?.textContent === '3')
      const calls = await page.evaluate(() => globalThis.httpCalls)
      assert.equal(calls.filter(call => call.url.endsWith('/api/greeting')).at(-1).cache, 'no-cache', 'post-eviction read revalidates')
      assert.ok(calls.every(call => call.credentials === 'omit'))
      await page.locator('#override').click()
      await page.waitForFunction(() => document.querySelector('#custom')?.textContent === 'Custom adapter wins')
      assert.ok(!requests.some(request => request.url().endsWith('/api/never')), 'matching custom adapter prevents fetch')
      const disposal = await page.evaluate(async () => {
        const entry = performance.getEntriesByType('resource').find(entry => new URL(entry.name).pathname.endsWith('/GreetingService.js'))
        if (!entry) throw new Error('compiled service module missing')
        const module = await import(entry.name)
        const firstOwner = new AbortController()
        const secondOwner = new AbortController()
        const first = await module.__tachyon_create_owner__({}, firstOwner.signal)
        const second = await module.__tachyon_create_owner__({}, secondOwner.signal)
        await first.clear()
        const cancelled = first.ask().then(() => 'completed', error => String(error))
        const completed = second.ask().then(() => second.message)
        setTimeout(() => firstOwner.abort(), 15)
        const result = { cancelled: await cancelled, completed: await completed }
        secondOwner.abort()
        return result
      })
      assert.match(disposal.cancelled, /CANCELLED/, 'disposing one owner aborts its pending request')
      assert.equal(disposal.completed, 'Hello from Yon', 'another owner remains live')
      assert.deepEqual(errors, [])
      await context.close()
    } finally { await browser.close() }
  }

  static async run() {
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-browser-http-'))
    try {
      cpSync(BrowserHttpGate.fixture, project, { recursive: true })
      BrowserHttpGate.build(project)
      await withServer(BrowserHttpGate.binary, ['serve', project, '--host', '127.0.0.1', '--port', '0', '--no-watch', '--no-self-restart', '--no-bundle'], async request => {
        const response = await request('/')
        const origin = new URL(response.url).origin
        for (const [name, launcher] of Object.entries(BrowserHttpGate.launchers)) {
          await BrowserHttpGate.check(launcher, origin)
          console.log(`${name}: injected compiled Dart client, typed real Yon answer, async responsiveness, once-only service, omitted cookies, cache/revalidation, custom override and isolated disposal pass`)
        }
      })
    } finally { rmSync(project, { recursive: true, force: true }) }
  }
}

await BrowserHttpGate.run()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { chromium } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'
import { YonReachFixture } from './yon-reach-fixture.mjs'

class YonOriginTest {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'yon-origin-'))
  static timeout = 15000

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

  static prepare(pagePort, yonPort) {
    new YonReachFixture({ directory: this.fixture, yonOrigin: `http://127.0.0.1:${yonPort}`, appOrigins: [`http://127.0.0.1:${pagePort}`] }).write()
    const built = spawnSync(this.binary, ['bundle', this.fixture], { encoding: 'utf8' })
    assert.equal(built.status, 0, `${built.stdout}${built.stderr}`)
  }

  static async checkCrossOriginAnswers(yon, pageOrigin) {
    const listed = await yon('/api/notes', { headers: { origin: pageOrigin } })
    assert.equal(listed.headers.get('access-control-allow-origin'), pageOrigin)
    const unlisted = await yon('/api/notes', { headers: { origin: 'https://evil.example' } })
    assert.equal(unlisted.headers.get('access-control-allow-origin'), null)
    const preflight = await yon('/api/notes', { method: 'OPTIONS', headers: { origin: pageOrigin, 'access-control-request-method': 'POST', 'access-control-request-headers': 'content-type' } })
    assert.equal(preflight.status, 204)
    assert.equal(preflight.headers.get('access-control-allow-headers'), 'content-type')
    const tokenPreflight = await yon('/api/account', { method: 'OPTIONS', headers: { origin: pageOrigin, 'access-control-request-method': 'GET', 'access-control-request-headers': 'authorization' } })
    assert.equal(tokenPreflight.status, 204)
    assert.equal(tokenPreflight.headers.get('access-control-allow-headers'), 'authorization')
    const refusedEntry = await yon('/api/entries', { method: 'POST', headers: { origin: pageOrigin, 'content-type': 'application/json' }, body: JSON.stringify({ name: '' }) })
    assert.equal(refusedEntry.status, 422)
    assert.equal(refusedEntry.headers.get('access-control-allow-origin'), pageOrigin)
    assert.match(refusedEntry.headers.get('content-type') ?? '', /^application\/problem\+json/)
    const problem = await refusedEntry.json()
    assert.equal(problem.status, 422)
    assert.deepEqual(problem.errors, [{ field: 'name', rule: 'NotEmpty' }])
    const anonymous = await yon('/api/account', { headers: { origin: pageOrigin } })
    assert.equal(anonymous.status, 401)
    assert.equal(anonymous.headers.get('access-control-allow-origin'), pageOrigin)
  }

  static async answer(page, method, route, click) {
    const answered = page.waitForResponse((response) => response.request().method() === method && new URL(response.url()).pathname === route, { timeout: this.timeout })
    const [response] = await Promise.all([answered, click()])
    return response
  }

  static async checkPage(url) {
    const browser = await chromium.launch({ headless: true })
    try {
      const page = await browser.newPage()
      const errors = []
      page.on('pageerror', (error) => errors.push(error.message))
      await page.goto(url)
      const shows = (text) => page.waitForFunction((expected) => document.body.textContent.includes(expected), text, { timeout: this.timeout })
      await shows('Welcome to the room')
      await page.getByRole('button', { name: 'Say hello' }).click()
      await shows('Hello from a tab')
      await shows('ANVIL-1 · 3')
      await page.getByRole('button', { name: 'Load note' }).click()
      await shows('Note from Yon')
      await page.getByRole('button', { name: 'Save note' }).click()
      await shows('Saved from another origin')
      await page.getByRole('button', { name: 'Send topic' }).click()
      await shows('Topic from Yon')
      const refusedEntry = await this.answer(page, 'POST', '/api/entries', () => page.getByRole('button', { name: 'Send invalid entry' }).click())
      assert.equal(refusedEntry.status(), 422)
      await shows('Entry refused by Yon')
      const anonymous = await this.answer(page, 'GET', '/api/account', () => page.getByRole('button', { name: 'Call without token' }).click())
      assert.equal(anonymous.status(), 401)
      await shows('Refused without a token')
      const signedIn = await this.answer(page, 'GET', '/api/account', () => page.getByRole('button', { name: 'Call with token' }).click())
      assert.equal(signedIn.status(), 200)
      await shows('Signed in as ada')
      assert.deepEqual(errors, [])
    } finally {
      await browser.close()
    }
  }

  static async run() {
    const pagePort = await this.freePort()
    const yonPort = await this.freePort()
    this.prepare(pagePort, yonPort)
    const pageOrigin = `http://127.0.0.1:${pagePort}`
    await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
      await this.checkCrossOriginAnswers(yon, pageOrigin)
      await withServer(this.binary, ['preview', this.fixture, '--static', '--port', String(pagePort)], async (pages) => {
        const response = await pages('/')
        assert.equal(response.status, 200)
        assert.match(response.headers.get('content-security-policy') ?? '', new RegExp(`connect-src [^;]*http://127\\.0\\.0\\.1:${yonPort} ws://127\\.0\\.0\\.1:${yonPort}`))
        await this.checkPage(response.url)
      })
    })
    console.log('Yon origin passed: a page on one port reached Yon on another over HTTP, a CORS preflight, a stream, a socket, a topic, a 422 check, and a signed-in route refused without a token and answered with one in Chromium.')
  }
}

await YonOriginTest.run()

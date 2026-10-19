#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'

class YonReachFixtureTest {
  static repository = fileURLToPath(new URL('..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static python = process.env.TAC_PYTHON ?? 'python3'
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'yon-reach-'))
  static timeout = 15000
  static invalidOrder = JSON.stringify({ name: '', level: 0 })
  static token = 'Bearer reach'
  static steps = [
    { button: 'Check HTTP', labels: ['HTTP ok hello'] },
    { button: 'Check socket', labels: ['Socket ok echo'] },
    { button: 'Check topic', labels: ['Topic sent accepted', 'Topic ok'] },
    { button: 'Check validation', labels: ['Check valid accepted', 'Check invalid refused'] },
    { button: 'Check signed in', labels: ['SignedIn anonymous refused', 'SignedIn ok reach'] },
  ]
  static linkedButtons = ['Check Rust HTTP', 'Check CSharp HTTP']
  static expectedAnswers = [
    'GET /api/hello 200',
    'GET /api/ticks 200',
    'GET /.tachyon/topics/reach-news 200',
    'POST /api/news 200',
    'POST /api/orders 200',
    'POST /api/orders 422',
    'GET /api/account 401',
    'GET /api/account 200',
  ]

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

  static run(command, args) {
    const finished = spawnSync(command, args, { encoding: 'utf8', timeout: 300000 })
    assert.equal(finished.status, 0, `${command} ${args.join(' ')}\n${finished.stdout}${finished.stderr}${finished.error ?? ''}`)
  }

  static prepare(pageOrigin, yonOrigin) {
    const script = path.join(this.repository, 'scripts/native/yon-reach-fixture.py')
    this.run(this.python, [script, this.fixture, 'web', yonOrigin, '--page-origin', pageOrigin])
    this.run(this.binary, ['bundle', this.fixture])
  }

  static async checkYonAnswers(yon, pageOrigin) {
    const hello = await yon('/api/hello', { headers: { origin: pageOrigin } })
    assert.equal(hello.status, 200)
    assert.equal(hello.headers.get('access-control-allow-origin'), pageOrigin)
    assert.deepEqual(await hello.json(), { message: 'hello' })
    const invalid = await yon('/api/orders', { method: 'POST', headers: { origin: pageOrigin, 'content-type': 'application/json' }, body: this.invalidOrder })
    assert.equal(invalid.status, 422)
    assert.equal(invalid.headers.get('access-control-allow-origin'), pageOrigin)
    assert.equal(invalid.headers.get('content-type'), 'application/problem+json')
    const problem = await invalid.json()
    assert.deepEqual(problem.errors, [{ field: 'level', rule: 'Min' }, { field: 'name', rule: 'NotEmpty' }, { field: 'name', rule: 'Length' }])
    const anonymous = await yon('/api/account', { headers: { origin: pageOrigin } })
    assert.equal(anonymous.status, 401)
    assert.equal(anonymous.headers.get('access-control-allow-origin'), pageOrigin)
    const signedIn = await yon('/api/account', { headers: { origin: pageOrigin, authorization: this.token } })
    assert.equal(signedIn.status, 200)
    const preflight = await yon('/api/account', { method: 'OPTIONS', headers: { origin: pageOrigin, 'access-control-request-method': 'GET', 'access-control-request-headers': 'authorization' } })
    assert.equal(preflight.status, 204)
    assert.equal(preflight.headers.get('access-control-allow-headers'), 'authorization')
  }

  static async expectLabels(page, labels) {
    for (const label of labels) {
      const shown = await page.getByText(label, { exact: true }).waitFor({ timeout: this.timeout }).then(() => true, () => false)
      const labelsNow = await page.locator('main p').allTextContents()
      assert.ok(shown, `the page never showed '${label}'. It shows: ${labelsNow.join(' | ')}`)
      console.log(`    ok   ${label}`)
    }
  }

  static async checkPage(url, yonOrigin) {
    const browser = await chromium.launch({ headless: true })
    try {
      const page = await browser.newPage()
      const errors = []
      const answers = new Set()
      const sockets = []
      page.on('pageerror', (error) => errors.push(error.message))
      page.on('websocket', (socket) => sockets.push(socket.url()))
      page.on('response', (response) => {
        const target = new URL(response.url())
        const method = response.request().method()
        if (target.origin === yonOrigin && method !== 'OPTIONS') answers.add(`${method} ${target.pathname} ${response.status()}`)
      })
      await page.goto(url)
      await this.expectLabels(page, ['Stream ok 3', 'Socket open'])
      for (const step of this.steps) {
        console.log(`  ${step.button}`)
        await page.getByRole('button', { name: step.button, exact: true }).click()
        await this.expectLabels(page, step.labels)
      }
      for (const name of this.linkedButtons) assert.equal(await page.getByRole('button', { name }).count(), 0, `${name} is only for native targets`)
      for (const answer of this.expectedAnswers) assert.ok(answers.has(answer), `Yon did not answer ${answer}: ${[...answers].join(', ')}`)
      assert.deepEqual(sockets, [`${yonOrigin.replace('http:', 'ws:')}/api/echo`])
      assert.deepEqual(errors, [])
    } finally {
      await browser.close()
    }
  }

  static async main() {
    const pagePort = await this.freePort()
    const yonPort = await this.freePort()
    const pageOrigin = `http://127.0.0.1:${pagePort}`
    const yonOrigin = `http://127.0.0.1:${yonPort}`
    this.prepare(pageOrigin, yonOrigin)
    await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
      await this.checkYonAnswers(yon, pageOrigin)
      await withServer(this.binary, ['preview', this.fixture, '--static', '--port', String(pagePort)], async (pages) => {
        const response = await pages('/')
        assert.equal(response.status, 200)
        await this.checkPage(response.url, yonOrigin)
      })
    })
    console.log('Yon reach fixture passed: a page on one origin reached Yon on another over HTTP, a stream, a socket, a topic, a 422 Check refusal and a SignedIn route in Chromium.')
  }
}

await YonReachFixtureTest.main()

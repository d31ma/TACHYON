#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class FirstAppDartBrowserGate {
  static repository = fileURLToPath(new URL('../', import.meta.url))
  static launchers = { chromium, firefox, webkit }

  static async run() {
    await FirstAppDartBrowserGate.#dispatch()
  }

  static #required(name) {
    assert.ok(process.env[name], `${name} must identify the actual staged Step 4 project/compiler`)
    return process.env[name]
  }

  static async #dispatch() {
    const project = path.resolve(FirstAppDartBrowserGate.#required('TAC_FIRST_APP_DART_PROJECT'))
    if (process.argv.includes('--stage-only')) return FirstAppDartBrowserGate.stage(project)
    return FirstAppDartBrowserGate.probe(project)
  }

  static async stage(project) {
    const docs = JSON.parse(await readFile(path.join(FirstAppDartBrowserGate.repository, 'website/client/shared/data/docs.json'), 'utf8'))
    const sections = docs.topics['first-app'].sections
    const step4 = sections.find(section => section.heading.startsWith('Step 4:'))
    const step3 = sections.find(section => section.heading.startsWith('Step 3:'))
    const clients = step4.files.filter(file => file.name.endsWith('.dart') || file.name.endsWith('.html'))
    const server = step3.files.filter(file => file.name === 'server/controllers/GreetingController.js' || file.name === 'server/models/Greeting.ts')
    const tests = FirstAppDartBrowserGate.#tests(sections)
    assert.equal(clients.length, 6, 'the published Dart slice has exactly its four native role/model files and two views')
    assert.equal(server.length, 2, 'the exact preceding tutorial Yon endpoint/model is retained')
    await FirstAppDartBrowserGate.#write(project, [...clients, ...server, ...tests])
  }

  static #tests(sections) {
    if (!process.argv.includes('--with-tests')) return []
    const step5 = sections.find(section => section.heading.startsWith('Step 5:'))
    const names = ['client/tests/HelloTestSetup.dart', 'client/tests/CounterServiceTest.dart', 'server/tests/GreetingTestSetup.js', 'server/tests/GreetingControllerTest.js']
    const tests = step5.files.filter(file => names.includes(file.name))
    assert.equal(tests.length, 4, 'the cumulative project contains the actual published Dart and matching Yon tests')
    return tests
  }

  static async #write(project, files) {
    for (const file of files) {
      const target = path.join(project, file.name)
      await mkdir(path.dirname(target), { recursive: true })
      await writeFile(target, file.code)
    }
  }

  static async probe(project) {
    const binary = path.resolve(FirstAppDartBrowserGate.#required('TAC_BIN'))
    const sourceCommit = FirstAppDartBrowserGate.#required('TAC_FIRST_APP_DART_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const receipt = { status: 'starting', sourceCommit, compilerSha256, browsers: [] }
    const started = performance.now()
    try {
      await withServer(binary, ['serve', project, '--host', '127.0.0.1', '--port', '0', '--no-watch', '--no-self-restart', '--no-bundle'], async request => {
        const origin = new URL((await request('/hello/')).url).origin
        await FirstAppDartBrowserGate.#browsers(origin, receipt)
      })
      receipt.status = 'pass'
    } catch (error) {
      receipt.status = 'fail'
      receipt.error = String(error).slice(0, 2000)
      throw error
    } finally {
      receipt.elapsedMilliseconds = performance.now() - started
      await writeFile(path.join(project, 'first-app-dart-browser.json'), JSON.stringify(receipt, null, 2) + '\n')
    }
  }

  static async #browsers(origin, receipt) {
    for (const [name, launcher] of Object.entries(FirstAppDartBrowserGate.launchers)) {
      const result = { name, errors: [], requests: [], requestCount: 0 }
      receipt.browsers.push(result)
      await FirstAppDartBrowserGate.check(launcher, origin, result)
    }
  }

  static async check(launcher, origin, result) {
    const browser = await launcher.launch({ timeout: 10_000 })
    const deadline = setTimeout(() => browser.close(), 30_000)
    try {
      const context = await browser.newContext()
      await context.addCookies([{ name: 'private_session', value: 'must-not-travel', url: origin }])
      const page = await context.newPage()
      page.setDefaultTimeout(10_000)
      FirstAppDartBrowserGate.#observe(page, result)
      const response = await page.goto(`${origin}/hello/`)
      assert.equal(response.status(), 200)
      await page.getByRole('button', { name: 'Click me', exact: true }).click()
      await page.waitForFunction(() => document.querySelector('section')?.textContent.includes('You clicked 1 times.'))
      const answer = page.waitForResponse(response => new URL(response.url()).pathname === '/api/greeting')
      await page.getByRole('button', { name: 'Ask the server', exact: true }).click()
      const remote = await answer
      const headers = await remote.request().allHeaders()
      result.requests.push({ path: '/api/greeting', method: remote.request().method(), cookie: headers.cookie ?? null })
      assert.equal(remote.status(), 200)
      await page.waitForFunction(() => [...document.querySelectorAll('section > p')].some(paragraph => paragraph.textContent === 'Hello from Yon'))
      await page.getByRole('button', { name: 'Click me', exact: true }).click()
      await page.waitForFunction(() => document.querySelector('section')?.textContent.includes('You clicked 2 times.'))
      assert.deepEqual(result.requests, [{ path: '/api/greeting', method: 'GET', cookie: null }], 'real Yon GET, no same-origin cookie')
      assert.equal(result.requestCount, 1, 'one click causes exactly one actual endpoint request')
      assert.deepEqual(result.errors, [])
      result.rendered = await page.locator('section > p').allTextContents()
      result.status = 'pass'
      await context.close()
    } finally {
      clearTimeout(deadline)
      await browser.close()
    }
  }

  static #observe(page, result) {
    page.on('request', request => {
      if (new URL(request.url()).pathname === '/api/greeting') result.requestCount += 1
    })
    page.on('pageerror', error => FirstAppDartBrowserGate.#record(result.errors, error.message))
    page.on('requestfailed', request => FirstAppDartBrowserGate.#record(result.errors, `${new URL(request.url()).pathname}: ${request.failure()?.errorText}`))
  }

  static #record(errors, message) {
    if (errors.length < 12) errors.push(message.slice(0, 2000))
  }
}

await FirstAppDartBrowserGate.run()

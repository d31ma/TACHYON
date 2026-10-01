#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class WebsiteLuaGate {
  static cases = [[0, 'About 1 minute to read'], [200, 'About 1 minute to read'], [201, 'About 2 minutes to read'], [1150, 'About 6 minutes to read']]
  static routes = ['/docs/annotations/', '/docs/types-in-every-language/']
  static quantityCases = [[1, 'feature', '1 feature'], [12, 'feature', '12 features'], [0, 'guide', '0 guides']]

  static async run() {
    await WebsiteLuaGate.#execute()
  }

  static #required(name) {
    assert.ok(process.env[name], `${name} must identify the retained ordinary website build`)
    return process.env[name]
  }

  static async #execute() {
    const fixture = path.resolve(WebsiteLuaGate.#required('TAC_WEBSITE_LUA_OUT'))
    const binary = path.resolve(WebsiteLuaGate.#required('TAC_BIN'))
    const sourceCommit = WebsiteLuaGate.#required('TAC_WEBSITE_SOURCE_COMMIT')
    const compilerSha256 = createHash('sha256').update(await readFile(binary)).digest('hex')
    const diagnostics = { status: 'starting', sourceCommit, compilerSha256, errors: [], routes: [] }
    try {
      await WebsiteLuaGate.#checkEmitted(fixture)
      await WebsiteLuaGate.#checkJavaEmitted(fixture)
      await WebsiteLuaGate.#serve(binary, fixture, diagnostics)
      diagnostics.status = 'pass'
    } catch (error) {
      diagnostics.status = 'fail'
      diagnostics.error = String(error).slice(0, 2000)
      throw error
    } finally {
      await writeFile(path.join(fixture, 'website-lua-browser.json'), JSON.stringify(diagnostics, null, 2) + '\n')
    }
  }

  static async #checkEmitted(fixture) {
    const services = path.join(fixture, '.tachyon/services')
    const wrapper = await readFile(path.join(services, 'ReadingTimeService.js'), 'utf8')
    assert.match(wrapper, /from '\.\/tachyon-lua-services\.js'/)
    const engine = await readFile(path.join(services, 'tachyon-lua-services.js'), 'utf8')
    assert.ok(engine.includes('./lua/tachyon-lua.wasm'), 'ordinary wrapper loads the real Lua engine')
    const wasm = await readFile(path.join(services, 'lua/tachyon-lua.wasm'))
    assert.deepEqual([...wasm.subarray(0, 4)], [0, 97, 115, 109], 'actual Wasm artifact exists')
  }

  static async #serve(binary, fixture, diagnostics) {
    await withServer(binary, ['preview', fixture, '--static', '--out-dir', '.', '--host', '127.0.0.1', '--port', '0', '--no-self-restart'], async request => {
      const origin = new URL((await request('/')).url).origin
      await WebsiteLuaGate.#probe(origin, diagnostics)
    })
  }

  static async #checkJavaEmitted(fixture) {
    const services = path.join(fixture, '.tachyon/services')
    const wrapper = await readFile(path.join(services, 'QuantityService.js'), 'utf8')
    assert.match(wrapper, /from '\.\/tachyon-java-services\.js'/)
    const wasm = await readFile(path.join(services, 'java/services.wasm'))
    assert.deepEqual([...wasm.subarray(0, 4)], [0, 97, 115, 109], 'actual Java Wasm artifact exists')
  }

  static async #probe(origin, diagnostics) {
    const browser = await chromium.launch({ timeout: 10_000 })
    const deadline = setTimeout(() => browser.close(), 45_000)
    try {
      const page = await browser.newPage()
      page.setDefaultTimeout(15_000)
      page.on('pageerror', error => WebsiteLuaGate.#record(diagnostics.errors, error.message))
      page.on('requestfailed', request => WebsiteLuaGate.#record(diagnostics.errors, `${new URL(request.url()).pathname}: ${request.failure()?.errorText}`))
      await WebsiteLuaGate.#routes(page, origin, diagnostics)
      diagnostics.javaQuantity = await WebsiteLuaGate.#quantity(page, origin)
      assert.deepEqual(diagnostics.errors, [], 'real website has no page or request errors')
    } finally {
      clearTimeout(deadline)
      await browser.close()
    }
  }

  static async #routes(page, origin, diagnostics) {
    for (const route of WebsiteLuaGate.routes) {
      diagnostics.routes.push(await WebsiteLuaGate.#route(page, origin, route))
    }
  }

  static #record(errors, message) {
    if (errors.length < 12) errors.push(message.slice(0, 2000))
  }

  static async #quantity(page, origin) {
    const response = await page.goto(`${origin}/docs/`)
    assert.equal(response.status(), 200)
    await page.waitForFunction(() => globalThis.__tachyonTac?.service('quantity') && globalThis.__tachyonTac?.service('documentation'))
    const actual = await page.evaluate(cases => {
      const service = globalThis.__tachyonTac.service('quantity')
      const documentation = globalThis.__tachyonTac.service('documentation')
      return { count: documentation.count(), cases: cases.map(([count, noun]) => service.describe(count, noun)), hidden: typeof service.nounFor }
    }, WebsiteLuaGate.quantityCases)
    assert.deepEqual(actual.cases, WebsiteLuaGate.quantityCases.map(([, , label]) => label))
    assert.equal(actual.hidden, 'undefined', 'private Java method stays private')
    const label = `${actual.count} feature${actual.count === 1 ? '' : 's'}.`
    await page.waitForFunction(expected => document.querySelector('section:has(#features) > p')?.textContent.trim().startsWith(expected), label)
    const rendered = await page.locator('section:has(#features) > p').innerText()
    assert.ok(rendered.trim().startsWith(label), 'real documentation feature count uses Java quantity service')
    return { ...actual, rendered }
  }

  static async #route(page, origin, route) {
    const response = await page.goto(`${origin}${route}`)
    assert.equal(response.status(), 200)
    await page.waitForFunction(() => globalThis.__tachyonTac?.service('readingTime') && globalThis.__tachyonTac?.service('topic'))
    const actual = await page.evaluate(cases => {
      const service = globalThis.__tachyonTac.service('readingTime')
      const topic = globalThis.__tachyonTac.service('topic')
      return { words: topic.words(), cases: cases.map(([words]) => service.label(words)), hidden: [typeof service._minutes, typeof service._phrase] }
    }, WebsiteLuaGate.cases)
    assert.deepEqual(actual.cases, WebsiteLuaGate.cases.map(([, label]) => label))
    assert.deepEqual(actual.hidden, ['undefined', 'undefined'], 'private Lua methods stay private')
    const minutes = Math.max(1, Math.ceil(actual.words / 200))
    const label = `About ${minutes} minute${minutes === 1 ? '' : 's'} to read`
    await page.waitForFunction(expected => document.querySelector('#guide-reading-time')?.textContent.startsWith(expected), label)
    return { route, ...actual, rendered: await page.locator('#guide-reading-time').textContent() }
  }
}

await WebsiteLuaGate.run()

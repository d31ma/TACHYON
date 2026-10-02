#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {{name: string, code: string}} FeatureSnippet */
/** @typedef {{name: string | undefined, code: string | null | undefined}} RenderedFeatureSnippet */
/** @typedef {{id: string, path: string, source: string, title: string, summary: string, group: string, verify: string | null, groupAnchor: string, files: FeatureSnippet[], found: boolean, catalogueTitle: string | undefined, catalogueFiles: FeatureSnippet[], hidden: string[]}} FeaturePageProbe */
/** @typedef {{id: string, path: string, source: string, title: string, groupAnchor: string, files: number, renderedNames: (string | undefined)[], hidden: string[]}} FeatureBrowserResult */

class WebsiteLuaGate {
  static cases = [[0, 'About 1 minute to read'], [200, 'About 1 minute to read'], [201, 'About 2 minutes to read'], [1150, 'About 6 minutes to read']]
  static routes = ['/docs/annotations/', '/docs/types-in-every-language/']
  /** @type {string[]} */
  static featureRoutes = ['routing', 'no-such-feature', 'companion-publish', 'routing']
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
      await WebsiteLuaGate.#checkFsharpEmitted(fixture)
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
    const wrapper = await readFile(path.join(services, 'InstallService.js'), 'utf8')
    assert.match(wrapper, /from '\.\/tachyon-java\.js'/)
    const wasm = await readFile(path.join(services, 'java/services.wasm'))
    assert.deepEqual([...wasm.subarray(0, 4)], [0, 97, 115, 109], 'actual Java Wasm artifact exists')
  }

  static async #checkFsharpEmitted(fixture) {
    const wrapper = await readFile(path.join(fixture, '.tachyon/services/QuantityService.js'), 'utf8')
    assert.match(wrapper, /from '\.\/tachyon-csharp\.js'/)
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
      diagnostics.kotlinFeatures = await WebsiteLuaGate.#features(page, origin)
      diagnostics.fsharpQuantity = await WebsiteLuaGate.#quantity(page, origin)
      diagnostics.javaInstall = await WebsiteLuaGate.#install(page, origin)
      diagnostics.pythonLanguages = await WebsiteLuaGate.#languages(page)
      assert.deepEqual(diagnostics.errors, [], 'real website has no page or request errors')
    } finally {
      clearTimeout(deadline)
      await browser.close()
    }
  }

  static async #languages(page) {
    const section = page.locator('#languages')
    await section.getByRole('button', { name: 'Yon · server controllers', exact: true }).click()
    await page.waitForFunction(() => [...(globalThis.__tachyonTac?.instances.values() ?? [])].some(owner => owner.languages))
    const actual = await page.evaluate(() => {
      const service = [...globalThis.__tachyonTac.instances.values()].find(owner => owner.languages).languages
      const expected = service.entries.flatMap(entry => Array.isArray(entry?.files) ? entry.files : [])
      return { expected: expected.map(file => file.name), names: service.handlers().map(file => file.name),
        hidden: [typeof service._files_of, typeof service._entry_files] }
    })
    assert.ok(actual.expected.length > 0, 'the real Yon catalogue has handler files')
    assert.deepEqual(actual.names, actual.expected, 'required Python entries retain ordered structured files')
    assert.deepEqual(actual.hidden, ['undefined', 'undefined'])
    await page.locator('#yon-language-examples').waitFor({ state: 'visible' })
    await page.waitForFunction(() => {
      const files = [...globalThis.__tachyonTac.instances.values()].find(owner => owner.languages).languages.handlers()
      const rendered = [...document.querySelectorAll('#yon-language-examples .demo-code-col')]
      return rendered.length > 0 && rendered.every(column => {
        const name = column.querySelector('.demo-code-name')?.textContent.trim()
        const file = files.find(candidate => candidate.name === name)
        return file && column.querySelector('pre code')?.textContent === file.code
      })
    })
    const renderedNames = await page.locator('#yon-language-examples .demo-code-name').allTextContents()
    assert.ok(renderedNames.length > 0, 'actual Yon-layer code retains the files returned by Python')
    await section.getByRole('button', { name: 'Tac · browser and native', exact: true }).click()
    await page.locator('#yon-language-examples').waitFor({ state: 'hidden' })
    return { ...actual, renderedNames }
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} origin
   * @returns {Promise<FeatureBrowserResult[]>}
   */
  static async #features(page, origin) {
    const routes = []
    for (const slug of WebsiteLuaGate.featureRoutes) routes.push(await WebsiteLuaGate.#feature(page, origin, slug))
    return routes
  }

  /**
   * @param {import('playwright').Page} page
   * @param {string} origin
   * @param {string} slug
   * @returns {Promise<FeatureBrowserResult>}
   */
  static async #feature(page, origin, slug) {
    const response = await page.goto(`${origin}/docs/features/${slug}/`)
    assert.equal(response.status(), 200, 'the genuine dynamic feature route is served')
    await page.waitForFunction(() => [...(globalThis.__tachyonTac?.instances.values() ?? [])].some(owner => owner.feature))
    const actual = await page.evaluate(() => {
      const service = [...globalThis.__tachyonTac.instances.values()].find(owner => owner.feature).feature
      const found = service.catalogue.features.find(entry => entry.id === service.id)
      const entry = service.entry()
      return { id: service.id, path: service.path(), source: service.source,
        title: entry.title, summary: entry.summary, group: entry.group, verify: entry.verify,
        groupAnchor: service.groupAnchor(), files: service.files(),
        found: Boolean(found), catalogueTitle: found?.title, catalogueFiles: found?.files ?? [],
        hidden: [typeof service.documentTitle, typeof service.initialize, typeof service.foundEntry] }
    })
    WebsiteLuaGate.#featureContract(actual, slug)
    await page.waitForFunction(title => document.title === `${title} — Tachyon docs`, actual.title)
    await page.waitForFunction(title => document.querySelector('.docs-article > h1')?.textContent.trim() === title, actual.title)
    await page.waitForFunction(hasFiles => (document.querySelectorAll('.docs-article .demo-code-col').length > 0) === hasFiles, actual.files.length > 0)
    await page.waitForFunction(verify => !verify || [...document.querySelectorAll('.docs-article > p')].some(paragraph => paragraph.textContent.includes(`Checked by ${verify}.`)), actual.verify)
    const rendered = await page.locator('.docs-article .demo-code-col').evaluateAll(columns => columns.map(column => ({
      name: column.querySelector('.demo-code-name')?.textContent.trim(),
      code: column.querySelector('pre code')?.textContent
    })))
    WebsiteLuaGate.#featureFiles(actual.files, rendered)
    const link = await page.locator('.docs-pagelinks__link').first().getAttribute('href')
    assert.ok(link.endsWith(actual.source), 'edit link retains the original feature source')
    const groups = await page.locator('.feature-group a').evaluateAll(links => links.map(link => link.getAttribute('href')))
    assert.deepEqual(groups, actual.group ? [actual.groupAnchor] : [], 'real group link matches the service')
    return { id: actual.id, path: actual.path, source: actual.source, title: await page.title(),
      groupAnchor: actual.groupAnchor, files: actual.files.length, renderedNames: rendered.map(file => file.name), hidden: actual.hidden }
  }

  /**
   * @param {FeaturePageProbe} actual
   * @param {string} slug
   * @returns {void}
   */
  static #featureContract(actual, slug) {
    assert.equal(actual.id, slug)
    assert.equal(actual.path, `/docs/features/${slug}`)
    assert.equal(actual.source, actual.found ? `website/content/features/${slug}/feature.json` : 'website/content/features/page.json')
    assert.equal(actual.title, actual.catalogueTitle ?? 'Feature not found')
    assert.deepEqual(actual.files, actual.catalogueFiles, 'typed nested snippet values survive the real Kotlin boundary')
    assert.deepEqual(actual.hidden, ['undefined', 'undefined', 'undefined'], 'provider, hook and private helper stay private')
  }

  /**
   * @param {FeatureSnippet[]} files
   * @param {RenderedFeatureSnippet[]} rendered
   * @returns {void}
   */
  static #featureFiles(files, rendered) {
    assert.equal(rendered.length > 0, files.length > 0, 'known features render snippets; unknown features have none')
    for (const shown of rendered) assert.deepEqual(shown, files.find(file => file.name === shown.name), 'rendered code is the original catalogue text')
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
    await page.waitForFunction(() => [...(globalThis.__tachyonTac?.instances.values() ?? [])].some(owner => owner.quantity && owner.documentation))
    const actual = await page.evaluate(cases => {
      const owner = [...globalThis.__tachyonTac.instances.values()].find(candidate => candidate.quantity && candidate.documentation)
      const service = owner.quantity
      const documentation = owner.documentation
      return { count: documentation.count(), cases: cases.map(([count, noun]) => service.describe(count, noun)), hidden: typeof service.nounFor }
    }, WebsiteLuaGate.quantityCases)
    assert.deepEqual(actual.cases, WebsiteLuaGate.quantityCases.map(([, , label]) => label))
    assert.equal(actual.hidden, 'undefined', 'private F# method stays private')
    const label = `${actual.count} feature${actual.count === 1 ? '' : 's'}.`
    await page.waitForFunction(expected => document.querySelector('section:has(#features) > p')?.textContent.trim().startsWith(expected), label)
    const rendered = await page.locator('section:has(#features) > p').innerText()
    assert.ok(rendered.trim().startsWith(label), 'real documentation feature count uses F# quantity service')
    return { ...actual, rendered }
  }

  static async #install(page, origin) {
    const response = await page.goto(`${origin}/`)
    assert.equal(response.status(), 200)
    await page.waitForFunction(() => globalThis.__tachyonTac?.service('install'))
    const initial = await page.evaluate(() => {
      const install = globalThis.__tachyonTac.service('install')
      return { unix: install.shows('unix'), windows: install.shows('windows'), hidden: typeof install.system }
    })
    assert.deepEqual(initial, { unix: true, windows: false, hidden: 'undefined' })
    await page.waitForFunction(() => document.querySelector('.command__text')?.textContent.startsWith('curl '))
    const windows = page.getByRole('button', { name: 'Windows', exact: true })
    await windows.click()
    await page.waitForFunction(() => document.querySelector('.command__text')?.textContent.startsWith('irm '))
    assert.equal(await windows.getAttribute('aria-pressed'), 'true')
    const selected = await page.evaluate(() => {
      const install = globalThis.__tachyonTac.service('install')
      return { unix: install.shows('unix'), windows: install.shows('windows') }
    })
    assert.deepEqual(selected, { unix: false, windows: true })
    await page.getByRole('button', { name: 'macOS · Linux', exact: true }).click()
    await page.waitForFunction(() => document.querySelector('.command__text')?.textContent.startsWith('curl '))
    assert.equal(await windows.getAttribute('aria-pressed'), 'false')
    assert.equal(await page.locator('.command__text').count(), 1, 'exactly one real installer command is rendered')
    return { initial, selected, restored: await page.locator('.command__text').innerText() }
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

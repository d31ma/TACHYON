#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {'chromium' | 'firefox' | 'webkit'} AppsEngine */
/** @typedef {'bundle' | 'separate'} AppsDeploy */
/** @typedef {{engine: AppsEngine, deploy: AppsDeploy, phase: string, errors: string[], shellScripts: number}} AppsDiagnostics */
/** @typedef {{path: string, heading: string, shell: string, chat: string, kept: boolean, focused: string, title: string}} AppsSnapshot */

class PageAppsBrowserGate {
  static _timeout = 15_000
  static _launchTimeout = 60_000
  static _engines = ['chromium', 'firefox', 'webkit']
  /** @type {AppsDeploy[]} */
  static _deploys = ['bundle', 'separate']
  static _plainHttpPrefetch = 'Prefetch request denied: URL must be secure (HTTPS)'
  static _repository = fileURLToPath(new URL('..', import.meta.url))
  static _fixture = 'crates/tachyon-core/tests/fixtures/tac-page-apps'
  static _shellScript = '/.tachyon/tac-shell.js'
  static _otherTachyon = '0.0.0-other'
  static _plainFiles = {
    'client/manifest.json': '{"name":"Plain"}',
    'client/pages/tac.html': '<page-plain />',
    'client/components/page/plain/tac.html': '<h1>Plain page</h1>',
  }

  /** @returns {Promise<void>} */
  static async run() {
    const binary = PageAppsBrowserGate._binary()
    const engines = PageAppsBrowserGate._selectedEngines()
    for (const deploy of PageAppsBrowserGate._deploys) await PageAppsBrowserGate._runDeploy(binary, engines, deploy)
    await PageAppsBrowserGate._runPlainApp(binary, engines)
  }

  /** @param {string} binary @param {AppsEngine[]} engines @returns {Promise<void>} */
  static async _runPlainApp(binary, engines) {
    const site = await mkdtemp(path.join(tmpdir(), 'tac-plain-app-'))
    try {
      await PageAppsBrowserGate._writePlainApp(site)
      await promisify(execFile)(binary, ['bundle', site])
      await withServer(binary, ['preview', site, '--static', '--port', '0'], async (request) => {
        const origin = new URL((await request('/')).url).origin
        for (const engine of engines) await PageAppsBrowserGate._checkPlainApp(engine, origin)
      })
    } finally {
      await rm(site, { recursive: true, force: true })
    }
  }

  /** @param {string} site @returns {Promise<void>} */
  static async _writePlainApp(site) {
    for (const [file, contents] of Object.entries(PageAppsBrowserGate._plainFiles)) {
      await mkdir(path.dirname(path.join(site, file)), { recursive: true })
      await writeFile(path.join(site, file), contents)
    }
  }

  /** @param {AppsEngine} engine @param {string} origin @returns {Promise<void>} */
  static async _checkPlainApp(engine, origin) {
    const browser = await PageAppsBrowserGate._browserType(engine).launch({ timeout: PageAppsBrowserGate._launchTimeout })
    try {
      const page = await browser.newPage()
      /** @type {string[]} */
      const requested = []
      page.on('request', (request) => requested.push(new URL(request.url()).pathname))
      await page.goto(`${origin}/`, { waitUntil: 'load', timeout: PageAppsBrowserGate._timeout })
      await page.waitForFunction(() => document.querySelector('h1')?.textContent === 'Plain page', null, { timeout: PageAppsBrowserGate._timeout })
      const shellFiles = requested.filter((pathname) => pathname.endsWith('/tac-shell.js') || pathname.endsWith('/shell.json'))
      assert.deepEqual(shellFiles, [], 'a plain app at / asks for no shell file')
      console.log(JSON.stringify({ status: 'pass', engine, deploy: 'plain', browserVersion: browser.version() }))
    } finally {
      await browser.close()
    }
  }

  /** @param {string} binary @param {AppsEngine[]} engines @param {AppsDeploy} deploy @returns {Promise<void>} */
  static async _runDeploy(binary, engines, deploy) {
    const site = await PageAppsBrowserGate._site()
    const shell = path.join(site, 'home')
    try {
      await PageAppsBrowserGate._build(binary, site, deploy)
      await withServer(binary, ['preview', shell, '--static', '--port', '0'], async (request) => {
        const origin = new URL((await request('/')).url).origin
        await PageAppsBrowserGate._checkChatDocument(request, deploy)
        for (const engine of engines) await PageAppsBrowserGate._runEngine(engine, origin, deploy)
        if (deploy === 'separate') await PageAppsBrowserGate._runOtherTachyon(engines, origin, shell)
      })
    } finally {
      await rm(site, { recursive: true, force: true })
    }
  }

  /** @param {string} binary @param {string} site @param {AppsDeploy} deploy @returns {Promise<void>} */
  static async _build(binary, site, deploy) {
    const shell = path.join(site, 'home')
    if (deploy === 'bundle') {
      await promisify(execFile)(binary, ['bundle', shell])
      return
    }
    const chat = path.join(site, 'chat')
    await promisify(execFile)(binary, ['bundle', shell, '--shell-only'])
    await promisify(execFile)(binary, ['bundle', chat])
    await cp(path.join(chat, 'dist', 'web', 'chat'), path.join(shell, 'dist', 'web', 'chat'), { recursive: true })
  }

  /** @param {AppsEngine[]} engines @param {string} origin @param {string} shell @returns {Promise<void>} */
  static async _runOtherTachyon(engines, origin, shell) {
    await PageAppsBrowserGate._markOtherTachyon(shell)
    for (const engine of engines) await PageAppsBrowserGate._checkOtherTachyon(engine, origin)
  }

  /** @param {string} shell @returns {Promise<void>} */
  static async _markOtherTachyon(shell) {
    const file = path.join(shell, 'dist', 'web', 'chat', '.tachyon', 'app.json')
    const table = JSON.parse(await readFile(file, 'utf8'))
    table.tachyon = PageAppsBrowserGate._otherTachyon
    await writeFile(file, JSON.stringify(table))
    await rm(`${file}.br`, { force: true })
    await rm(`${file}.gz`, { force: true })
  }

  /** @param {AppsEngine} engine @param {string} origin @returns {Promise<void>} */
  static async _checkOtherTachyon(engine, origin) {
    const browser = await PageAppsBrowserGate._browserType(engine).launch({ timeout: PageAppsBrowserGate._launchTimeout })
    try {
      const page = await browser.newPage()
      await PageAppsBrowserGate._checkShellLoad(page, origin)
      await page.click('#to-chat')
      const snapshot = await PageAppsBrowserGate._waitForHeading(page, 'Chat page')
      assert.equal(snapshot.kept, false, 'a link to an app built with another TACHYON version loads the whole page')
      assert.equal(snapshot.shell, '', 'an app built with another TACHYON version renders alone, without the shell')
      console.log(JSON.stringify({ status: 'pass', engine, deploy: 'separate', check: 'other-tachyon-version' }))
    } finally {
      await browser.close()
    }
  }

  /** @param {(path: string) => Promise<Response>} request @param {AppsDeploy} deploy @returns {Promise<void>} */
  static async _checkChatDocument(request, deploy) {
    const html = await (await request('/chat/')).text()
    const inline = html.includes('id="tachyon-shell"')
    assert.equal(inline, deploy === 'bundle', `the ${deploy} chat page holds the shell plan only when it is bundled`)
  }

  /** @returns {string} */
  static _binary() {
    const binary = process.env.TAC_BIN
    assert.ok(binary, 'TAC_BIN names the ty binary under test')
    return path.resolve(binary)
  }

  /** @returns {AppsEngine[]} */
  static _selectedEngines() {
    const requested = process.env.TAC_PAGE_APPS_BROWSERS
    if (!requested) return /** @type {AppsEngine[]} */ (PageAppsBrowserGate._engines)
    const engines = requested.split(',')
    for (const engine of engines) assert.ok(PageAppsBrowserGate._engines.includes(engine), `unknown browser ${engine}`)
    return /** @type {AppsEngine[]} */ (engines)
  }

  /** @returns {Promise<string>} */
  static async _site() {
    const site = await mkdtemp(path.join(tmpdir(), 'tac-page-apps-'))
    await cp(path.join(PageAppsBrowserGate._repository, PageAppsBrowserGate._fixture), site, { recursive: true })
    return site
  }

  /** @param {AppsEngine} engine @returns {import('playwright').BrowserType} */
  static _browserType(engine) {
    if (engine === 'firefox') return firefox
    if (engine === 'webkit') return webkit
    return chromium
  }

  /** @param {AppsEngine} engine @param {string} origin @param {AppsDeploy} deploy @returns {Promise<void>} */
  static async _runEngine(engine, origin, deploy) {
    /** @type {AppsDiagnostics} */
    const diagnostics = { engine, deploy, phase: 'starting', errors: [], shellScripts: 0 }
    const browser = await PageAppsBrowserGate._browserType(engine).launch({ timeout: PageAppsBrowserGate._launchTimeout })
    const page = await browser.newPage()
    page.on('request', (request) => PageAppsBrowserGate._countShellScript(request, diagnostics))
    page.on('pageerror', (error) => diagnostics.errors.push(error.message))
    page.on('console', (message) => PageAppsBrowserGate._recordConsole(message, diagnostics))
    try {
      diagnostics.phase = 'shell-load'
      await PageAppsBrowserGate._checkShellLoad(page, origin)
      diagnostics.phase = 'soft-to-chat'
      await PageAppsBrowserGate._checkSoftToChat(page)
      diagnostics.phase = 'back-to-home'
      await PageAppsBrowserGate._checkBack(page)
      diagnostics.phase = 'forward-to-chat'
      await PageAppsBrowserGate._checkForward(page)
      assert.equal(diagnostics.shellScripts, 1, 'the shell app loads tac-shell.js once')
      diagnostics.phase = 'direct-chat-load'
      await PageAppsBrowserGate._checkDirectChat(page, origin)
      diagnostics.phase = 'shell-script-failure'
      await PageAppsBrowserGate._checkShellScriptFailure(browser, origin)
      assert.deepEqual(diagnostics.errors, [])
      console.log(JSON.stringify({ status: 'pass', engine, deploy, browserVersion: browser.version() }))
    } catch (error) {
      console.error(JSON.stringify({ status: 'fail', ...diagnostics }))
      throw error
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Request} request @param {AppsDiagnostics} diagnostics @returns {void} */
  static _countShellScript(request, diagnostics) {
    if (new URL(request.url()).pathname !== PageAppsBrowserGate._shellScript) return
    diagnostics.shellScripts += 1
  }

  /** @param {import('playwright').Browser} browser @param {string} origin @returns {Promise<void>} */
  static async _checkShellScriptFailure(browser, origin) {
    const page = await browser.newPage()
    try {
      await page.route(`**${PageAppsBrowserGate._shellScript}`, (route) => route.abort())
      await page.goto(`${origin}/`, { waitUntil: 'load', timeout: PageAppsBrowserGate._timeout })
      await page.waitForFunction(() => document.querySelector('h1')?.textContent === 'Home page', null, { timeout: PageAppsBrowserGate._timeout })
      const shell = await page.evaluate(() => document.getElementById('shell-count')?.textContent ?? '')
      assert.equal(shell, '', 'without tac-shell.js the page renders alone, not blank')
    } finally {
      await page.close()
    }
  }

  /** @param {import('playwright').ConsoleMessage} message @param {AppsDiagnostics} diagnostics @returns {void} */
  static _recordConsole(message, diagnostics) {
    if (message.type() !== 'error') return
    if (message.text() === PageAppsBrowserGate._plainHttpPrefetch) return
    diagnostics.errors.push(message.text())
  }

  /** @param {import('playwright').Page} page @returns {Promise<AppsSnapshot>} */
  static _snapshot(page) {
    return page.evaluate(() => ({
      path: location.pathname,
      heading: document.querySelector('main h1')?.textContent ?? '',
      shell: document.getElementById('shell-count')?.textContent ?? '',
      chat: document.getElementById('chat-count')?.textContent ?? '',
      kept: globalThis.__softNavigationMark === 'kept',
      focused: document.activeElement?.id ?? '',
      title: document.title,
    }))
  }

  /** @param {import('playwright').Page} page @param {string} heading @returns {Promise<AppsSnapshot>} */
  static async _waitForHeading(page, heading) {
    await page.waitForFunction((text) => document.querySelector('main h1')?.textContent === text, heading, { timeout: PageAppsBrowserGate._timeout })
    return PageAppsBrowserGate._snapshot(page)
  }

  /** @param {import('playwright').Page} page @param {string} origin @returns {Promise<void>} */
  static async _checkShellLoad(page, origin) {
    await page.goto(`${origin}/`, { waitUntil: 'load', timeout: PageAppsBrowserGate._timeout })
    await page.waitForFunction(() => document.documentElement.dataset.tachyonRendered === 'client', null, { timeout: PageAppsBrowserGate._timeout })
    const snapshot = await PageAppsBrowserGate._waitForHeading(page, 'Home page')
    assert.equal(snapshot.shell, 'Shell 0', 'the shell layout renders around the home page')
    await page.click('#shell-add')
    await page.waitForFunction(() => document.getElementById('shell-count')?.textContent === 'Shell 1', null, { timeout: PageAppsBrowserGate._timeout })
    await page.evaluate(() => { globalThis.__softNavigationMark = 'kept' })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkSoftToChat(page) {
    await page.click('#to-chat')
    const snapshot = await PageAppsBrowserGate._waitForHeading(page, 'Chat page')
    assert.equal(snapshot.kept, true, 'a link to another page app does not reload the document')
    assert.match(snapshot.path, /^\/chat\/?$/, 'the address bar shows the chat address')
    assert.equal(snapshot.shell, 'Shell 1', 'the shell keeps its state across the navigation')
    assert.equal(snapshot.chat, 'Chat 0', 'the chat page starts with its own service')
    assert.equal(snapshot.focused, 'chat-title', 'focus moves to the new page heading')
    await page.click('#chat-add')
    await page.waitForFunction(() => document.getElementById('chat-count')?.textContent === 'Chat 1', null, { timeout: PageAppsBrowserGate._timeout })
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkBack(page) {
    await page.goBack()
    const snapshot = await PageAppsBrowserGate._waitForHeading(page, 'Home page')
    assert.equal(snapshot.kept, true, 'Back between page apps does not reload the document')
    assert.equal(snapshot.path, '/', 'Back restores the home address')
    assert.equal(snapshot.shell, 'Shell 1', 'the shell keeps its state after Back')
    assert.equal(snapshot.chat, '', 'the chat page left the outlet')
  }

  /** @param {import('playwright').Page} page @returns {Promise<void>} */
  static async _checkForward(page) {
    await page.goForward()
    const snapshot = await PageAppsBrowserGate._waitForHeading(page, 'Chat page')
    assert.equal(snapshot.kept, true, 'Forward between page apps does not reload the document')
    assert.equal(snapshot.chat, 'Chat 0', 'a page service ends when its page leaves the outlet')
  }

  /** @param {import('playwright').Page} page @param {string} origin @returns {Promise<void>} */
  static async _checkDirectChat(page, origin) {
    await page.goto(`${origin}/chat/`, { waitUntil: 'load', timeout: PageAppsBrowserGate._timeout })
    const snapshot = await PageAppsBrowserGate._waitForHeading(page, 'Chat page')
    assert.equal(snapshot.shell, 'Shell 0', 'a direct visit to a page app renders the shell around it')
    await page.click('#to-home')
    const home = await PageAppsBrowserGate._waitForHeading(page, 'Home page')
    assert.equal(home.path, '/', 'a link from a page app back to the shell works')
  }
}

await PageAppsBrowserGate.run()

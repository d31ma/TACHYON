#!/usr/bin/env node
import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { once } from 'node:events'
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { request } from 'node:http'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'

/** @typedef {import('playwright').BrowserType} BrowserType */
/** @typedef {import('node:child_process').ChildProcess} ChildProcess */
/** @typedef {{browser: string, secureContext: boolean, rendered: string, ownCookie: string, otherCookie: string, hotUpdate: string}} BrowserResult */

class LocalhostSubdomainGate {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static fixture = path.join(LocalhostSubdomainGate.repository, 'crates/tachyon-cli/tests/fixtures/localhost-subdomain')
  static template = 'client/components/home/view/tac.html'
  static readyLine = /http:\/\/app\.localhost:(\d+)\//
  static readyMilliseconds = 60_000
  static stepMilliseconds = 20_000
  static secureBrowsers = ['chromium', 'firefox']
  /** @type {Array<[string, BrowserType]>} */
  static browsers = [['chromium', chromium], ['firefox', firefox], ['webkit', webkit]]

  /** @returns {Promise<void>} */
  static async run() {
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-localhost-subdomain-'))
    cpSync(LocalhostSubdomainGate.fixture, project, { recursive: true })
    const server = spawn(LocalhostSubdomainGate.binary, ['serve', project, '--host', 'app.localhost', '--port', '0', '--no-self-restart'], {
      cwd: LocalhostSubdomainGate.repository,
      env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
      stdio: ['ignore', 'pipe', 'pipe'],
    })
    try {
      const port = await LocalhostSubdomainGate.readyPort(server)
      const foreign = await LocalhostSubdomainGate.foreignHost(port)
      assert.equal(foreign.status, 403, foreign.body)
      assert.match(foreign.body, /TY1366/)
      /** @type {BrowserResult[]} */
      const results = []
      for (const [name, type] of LocalhostSubdomainGate.browsers) {
        results.push(await LocalhostSubdomainGate.probe(name, type, port, project, results.length + 1))
      }
      console.log(JSON.stringify({ foreignHost: foreign.status, results }, null, 2))
      console.log('PASS: localhost subdomain browser gate')
    } finally {
      await LocalhostSubdomainGate.stop(server)
      rmSync(project, { recursive: true, force: true })
    }
  }

  /** @param {ChildProcess} server @returns {Promise<number>} */
  static async readyPort(server) {
    const lines = createInterface({ input: server.stdout })
    const timeout = new Promise((_, reject) => {
      setTimeout(() => reject(new Error('The server did not print an app.localhost address in time.')), LocalhostSubdomainGate.readyMilliseconds).unref()
    })
    const ready = (async () => {
      for await (const line of lines) {
        const match = line.match(LocalhostSubdomainGate.readyLine)
        if (match) return Number(match[1])
      }
      throw new Error('The server stopped before it was ready.')
    })()
    return Promise.race([ready, timeout])
  }

  /** @param {number} port @returns {Promise<{status: number, body: string}>} */
  static foreignHost(port) {
    return new Promise((resolve, reject) => {
      const outgoing = request({ host: '127.0.0.1', port, path: '/', headers: { host: 'evil.example' } }, answer => {
        let body = ''
        answer.setEncoding('utf8')
        answer.on('data', chunk => { body += chunk })
        answer.on('end', () => resolve({ status: answer.statusCode ?? 0, body }))
      })
      outgoing.on('error', reject)
      outgoing.end()
    })
  }

  /** @param {string} name @param {BrowserType} type @param {number} port @param {string} project @param {number} version @returns {Promise<BrowserResult>} */
  static async probe(name, type, port, project, version) {
    const browser = await type.launch({ headless: true })
    try {
      const context = await browser.newContext()
      const page = await context.newPage()
      const appOrigin = `http://app.localhost:${port}`
      await page.goto(`${appOrigin}/`, { waitUntil: 'load', timeout: LocalhostSubdomainGate.stepMilliseconds })
      const rendered = await page.locator('h1').innerText({ timeout: LocalhostSubdomainGate.stepMilliseconds })
      const secureContext = await page.evaluate(() => window.isSecureContext)
      if (LocalhostSubdomainGate.secureBrowsers.includes(name)) assert.equal(secureContext, true, `${name} treats app.localhost as a secure context`)
      const ownCookie = await page.evaluate(async () => {
        await fetch('/session', { method: 'POST' })
        const answer = await fetch('/session')
        return (await answer.json()).cookie
      })
      assert.match(ownCookie, /session=app-only/, `${name} sends the cookie back to app.localhost`)
      const hotUpdate = await LocalhostSubdomainGate.hotUpdate(page, project, version)
      const otherPage = await context.newPage()
      await otherPage.goto(`http://other.localhost:${port}/`, { waitUntil: 'load', timeout: LocalhostSubdomainGate.stepMilliseconds })
      const otherCookie = await otherPage.evaluate(async () => (await (await fetch('/session')).json()).cookie)
      assert.doesNotMatch(otherCookie, /session=app-only/, `${name} keeps the app.localhost cookie away from other.localhost`)
      return { browser: name, secureContext, rendered, ownCookie, otherCookie, hotUpdate }
    } finally {
      await browser.close()
    }
  }

  /** @param {import('playwright').Page} page @param {string} project @param {number} version @returns {Promise<string>} */
  static async hotUpdate(page, project, version) {
    const file = path.join(project, LocalhostSubdomainGate.template)
    const text = readFileSync(file, 'utf8').replace(/Home \d+/, `Home ${version}`)
    writeFileSync(file, text)
    const expected = `Home ${version}`
    await page.waitForFunction(heading => document.querySelector('h1')?.textContent?.trim() === heading, expected, { timeout: LocalhostSubdomainGate.stepMilliseconds })
    return expected
  }

  /** @param {ChildProcess} server @returns {Promise<void>} */
  static async stop(server) {
    if (server.exitCode !== null) return
    const exited = once(server, 'exit')
    server.kill('SIGTERM')
    await exited
  }
}

await LocalhostSubdomainGate.run()

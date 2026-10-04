#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cp, mkdtemp, readFile, rm } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

/** @typedef {{engine: string, version: string, count: string, heard: string, message: string, direct: unknown[], remote: unknown[]}} EngineProof */

class CppServicesBrowserGate {
  /** @type {string} */
  static fixture = fileURLToPath(new URL('../crates/tachyon-core/tests/fixtures/tac-cpp-services', import.meta.url))
  /** @type {number} */
  static timeout = 20_000
  /** @type {Record<string, import('playwright').BrowserType>} */
  static engines = { chromium, firefox, webkit }

  /** @returns {Promise<void>} */
  static async run() {
    const binary = process.env.TAC_BIN
    assert.ok(binary, 'TAC_BIN names the ty binary to check')
    const root = await mkdtemp(path.join(os.tmpdir(), 'tac-cpp-services-'))
    try {
      await cp(CppServicesBrowserGate.fixture, root, { recursive: true })
      const bundle = spawnSync(path.resolve(binary), ['bundle', root, '--no-incremental'], { encoding: 'utf8' })
      assert.equal(bundle.status, 0, `${bundle.stdout}${bundle.stderr}`)
      const wasm = await readFile(path.join(root, 'dist', 'web', '.tachyon', 'services', 'cpp', 'services.wasm'))
      const serve = ['serve', root, '--host', '127.0.0.1', '--port', '0', '--out-dir', 'dist/web', '--no-watch', '--no-self-restart', '--no-bundle']
      const proofs = await withServer(path.resolve(binary), serve, async request => {
        const origin = new URL((await request('/')).url).origin
        /** @type {EngineProof[]} */
        const found = []
        for (const name of CppServicesBrowserGate.selected()) found.push(await CppServicesBrowserGate.probe(name, origin))
        return found
      })
      console.log(JSON.stringify({ status: 'pass', wasmBytes: wasm.length, proofs }))
    } finally {
      await rm(root, { recursive: true, force: true })
    }
  }

  /** @returns {string[]} */
  static selected() {
    const wanted = process.env.TAC_BROWSER_ENGINES
    return wanted ? wanted.split(',') : Object.keys(CppServicesBrowserGate.engines)
  }

  /** @param {import('playwright').Page} page @param {string} selector @param {string} expected @returns {Promise<string>} */
  static async text(page, selector, expected) {
    const timeout = CppServicesBrowserGate.timeout
    await page.waitForFunction(([target, wanted]) => document.querySelector(target)?.textContent?.trim() === wanted, [selector, expected], { timeout })
    return (await page.locator(selector).textContent())?.trim() ?? ''
  }

  /** @param {string} name @param {string} origin @returns {Promise<EngineProof>} */
  static async probe(name, origin) {
    const browser = await CppServicesBrowserGate.engines[name].launch({ timeout: CppServicesBrowserGate.timeout })
    try {
      const page = await browser.newPage()
      /** @type {string[]} */
      const errors = []
      page.on('pageerror', error => errors.push(String(error)))
      await page.goto(origin, { waitUntil: 'networkidle', timeout: CppServicesBrowserGate.timeout })
      await CppServicesBrowserGate.text(page, '#label', 'Tally')
      await CppServicesBrowserGate.text(page, '#count', '0')
      await page.click('#add')
      const count = await CppServicesBrowserGate.text(page, '#count', '2')
      await page.click('#place')
      const heard = await CppServicesBrowserGate.text(page, '#heard', '21')
      await CppServicesBrowserGate.text(page, '#status', 'paid')
      await page.click('#ask')
      const message = await CppServicesBrowserGate.text(page, '#message', 'Hello from Yon')
      await CppServicesBrowserGate.text(page, '#loads', '1')
      const remote = await page.evaluate(async () => {
        const Service = await import('/.tachyon/services/GreetingService.js')
        const owner = new Service.default()
        try {
          const shouted = await owner.shout('hey')
          const lost = await owner.lost().then(() => 'answered', (error) => String(error.code))
          return [shouted, lost]
        } finally {
          owner.__tachyonDispose()
        }
      })
      assert.deepEqual(remote, ['hey #7!', 'HTTP_STATUS'])
      const direct = await page.evaluate(async () => {
        const Service = await import('/.tachyon/services/TallyService.js')
        const owner = new Service.default()
        try {
          const refused = (() => { try { owner.add('many'); return 'accepted' } catch (error) { return String(error.message) } })()
          return [owner.ticket(), owner.ticket(), owner.doubled({ status: 'open', total: 3, notes: ['a'] }), owner.count, refused]
        } finally {
          owner.__tachyonDispose()
        }
      })
      assert.deepEqual(direct.slice(0, 4), [101, 102, { status: 'open', total: 6, notes: ['a'] }, 2])
      assert.match(String(direct[4]), /not a number/)
      assert.deepEqual(errors, [])
      return { engine: name, version: browser.version(), count, heard, message, direct, remote }
    } finally {
      await browser.close()
    }
  }
}

await CppServicesBrowserGate.run()

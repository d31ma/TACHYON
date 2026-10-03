#!/usr/bin/env node
import assert from 'node:assert/strict'
import { cp, mkdtemp, rm } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class LuaNullArgumentsBrowserGate {
  static fixture = fileURLToPath(new URL('../crates/tachyon-core/tests/fixtures/tac-lua-null-arguments', import.meta.url))
  static timeout = 15_000
  static expected = 'nil 5'

  static async run() {
    const binary = process.env.TAC_BIN
    assert.ok(binary, 'TAC_BIN names the ty binary to check')
    const root = await mkdtemp(path.join(os.tmpdir(), 'tac-lua-null-arguments-'))
    try {
      await cp(LuaNullArgumentsBrowserGate.fixture, root, { recursive: true })
      const proof = await withServer(path.resolve(binary), ['preview', root, '--static', '--out-dir', 'dist', '--port', '0'], async request => {
        const origin = new URL((await request('/')).url).origin
        return LuaNullArgumentsBrowserGate.probe(origin)
      })
      console.log(JSON.stringify({ status: 'pass', proof }))
    } finally {
      await rm(root, { recursive: true, force: true })
    }
  }

  static async probe(origin) {
    const browser = await chromium.launch({ timeout: LuaNullArgumentsBrowserGate.timeout })
    try {
      const page = await browser.newPage()
      const errors = []
      page.on('pageerror', error => errors.push(String(error)))
      await page.goto(origin, { waitUntil: 'networkidle', timeout: LuaNullArgumentsBrowserGate.timeout })
      const answer = page.locator('#pair-answer')
      await answer.waitFor({ timeout: LuaNullArgumentsBrowserGate.timeout })
      await page.waitForFunction(() => document.querySelector('#pair-answer')?.textContent?.trim() !== '', undefined, { timeout: LuaNullArgumentsBrowserGate.timeout })
      const rendered = (await answer.textContent())?.trim()
      const direct = await page.evaluate(async () => {
        const Service = await import('/.tachyon/services/PairService.js')
        const owner = new Service.default()
        try {
          return [owner.describe(null, 5), owner.describe('a', null)]
        } finally {
          owner.__tachyonDispose()
        }
      })
      assert.equal(rendered, LuaNullArgumentsBrowserGate.expected, 'a page call with a null first argument keeps 5 in the second parameter')
      assert.deepEqual(direct, [LuaNullArgumentsBrowserGate.expected, 'a nil'])
      assert.deepEqual(errors, [])
      return { rendered, direct, browserVersion: browser.version() }
    } finally {
      await browser.close()
    }
  }
}

await LuaNullArgumentsBrowserGate.run()

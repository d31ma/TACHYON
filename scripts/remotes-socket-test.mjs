#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class RemotesSocketTest {
  static repository = fileURLToPath(new URL('..', import.meta.url))
  static binary = process.env.TAC_BIN
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'remotes-socket-'))

  static write(name, text) {
    const target = path.join(this.fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static prepare() {
    const source = path.join(this.repository, 'website/content/features/sockets')
    const feature = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
    for (const file of feature.files) {
      const selected = file.name.endsWith('.js') || file.name.endsWith('.html') || file.name.includes('/models/') && file.name.endsWith('.ts')
      if (selected) this.write(file.name, readFileSync(path.join(source, file.name), 'utf8'))
    }
    this.write('client/manifest.json', JSON.stringify({ tac: { visibility: { check: false } } }))
  }

  static async run() {
    assert.ok(this.binary, 'Set TAC_BIN to a qualified compiler checkpoint')
    this.prepare()
    await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', '0'], async request => {
      const response = await request('/')
      assert.equal(response.status, 200)
      const browser = await chromium.launch({ headless: true })
      try {
        const page = await browser.newPage()
        const errors = []
        page.on('pageerror', error => errors.push(error.message))
        await page.goto(response.url)
        await page.waitForFunction(() => document.body.textContent.includes('Welcome to the room'), { timeout: 15000 })
        await page.getByRole('button', { name: 'Say hello' }).click()
        await page.waitForFunction(() => document.body.textContent.includes('Hello from a tab'), { timeout: 15000 })
        assert.deepEqual(errors, [])
      } finally {
        await browser.close()
      }
    })
    console.log('Published remotes socket client passed: real server, welcome, send, event delivery and view update in Chromium.')
  }
}

await RemotesSocketTest.run()

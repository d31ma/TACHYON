#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import playwright from 'playwright'

/**
 * A quick check of the calls the web gates make, run first in a branded browser so a broken
 * driver setup (safaridriver not enabled, Chrome not on the device) shows before the long gates.
 */
class AdapterCheck {
  static browsers = (process.env.TAC_ADAPTER_BROWSERS ?? 'chromium').split(',')
  static page = `<!doctype html><title>Adapter check</title><body>
<label for="name">Name</label><input id="name"><button>Save</button><ul></ul>
<shadow-host></shadow-host><p id="out">start</p>
<script>
customElements.define('shadow-host', class extends HTMLElement {
  connectedCallback() {
    const root = this.attachShadow({ mode: 'open' })
    root.innerHTML = '<span id="inner">shadow text</span><button>Shadow</button>'
    root.querySelector('button').onclick = () => { document.getElementById('out').textContent = 'shadow clicked' }
  }
})
document.querySelector('body > button').onclick = () => {
  const item = document.createElement('li')
  item.textContent = document.getElementById('name').value
  document.querySelector('ul').append(item)
}
setTimeout(() => { throw new Error('late failure') }, 300)
</script>`

  /** @param {string} origin @param {import('playwright').BrowserType} type */
  static async check(origin, type) {
    const browser = await type.launch({ timeout: 30_000 })
    try {
      const context = await browser.newContext()
      const page = await context.newPage()
      const errors = []
      page.on('pageerror', (error) => errors.push(error.message))
      await page.goto(`${origin}/`, { waitUntil: 'networkidle' })
      await page.getByLabel('Name').fill('Ada')
      await page.getByRole('button', { name: 'Save' }).click()
      await page.locator('li').nth(0).waitFor({ timeout: 5_000 })
      assert.deepEqual(await page.locator('li').allTextContents(), ['Ada'])
      assert.equal(await page.evaluate(([first, second]) => first + second, [2, 3]), 5)
      assert.equal(await page.evaluate('1 + 1'), 2)
      assert.equal(await page.evaluate(() => Promise.resolve('later')), 'later')
      assert.equal(await page.locator('shadow-host #inner').textContent(), 'shadow text')
      await page.getByRole('button', { name: 'Shadow', exact: true }).click()
      await page.waitForFunction(() => document.getElementById('out').textContent === 'shadow clicked')
      assert.equal(await page.locator('button').count(), 2)
      await page.locator('#out').waitFor({ state: 'visible' })
      await page.locator('#missing').waitFor({ state: 'detached' })
      const second = await context.newPage()
      await second.goto(`${origin}/`)
      assert.equal(await second.title(), 'Adapter check')
      await page.waitForTimeout(500)
      await page.evaluate(() => 0)
      assert.deepEqual(errors, ['late failure'])
      console.log(`ok - ${browser.version()}: navigation, evaluate, locators through an open shadow root, typing, clicks, two tabs and page errors`)
    } finally {
      await browser.close()
    }
  }

  static async main() {
    const server = createServer((request, response) => response.writeHead(200, { 'content-type': 'text/html' }).end(AdapterCheck.page))
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    try {
      for (const name of AdapterCheck.browsers) await AdapterCheck.check(`http://127.0.0.1:${server.address().port}`, playwright[name])
    } finally {
      server.close()
    }
  }
}

await AdapterCheck.main()

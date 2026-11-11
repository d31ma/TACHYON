#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { chromium } from 'playwright'

class FocusBrowserCheck {
  static sources = ['tac-focus.js', 'tac-web.js'].map((name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url))

  static page = `<!doctype html><html lang="en"><body>
    <button id="before">Before</button>
    <div id="host">
      <button id="open" data-focus="open">Open</button>
      <div id="dialog" data-focus="dialog" role="dialog" tabindex="-1">
        <input id="name" data-focus="name">
        <button id="hidden" hidden>Hidden</button>
        <button id="off" disabled>Off</button>
        <button id="save" data-focus="save">Save</button>
      </div>
    </div>
    <button id="after">After</button>
  </body></html>`

  static async run() {
    const browser = await chromium.launch()
    try {
      const page = await browser.newPage()
      await page.setContent(FocusBrowserCheck.page)
      const runtime = (await Promise.all(FocusBrowserCheck.sources.map((source) => readFile(source, 'utf8')))).join('\n')
      await page.addScriptTag({ content: `${runtime}\n${FocusBrowserCheck.connect}` })
      await FocusBrowserCheck.trapKeepsTabInside(page)
      await FocusBrowserCheck.releaseReturnsFocus(page)
      console.log('focus: trap keeps Tab and Shift+Tab inside, a click outside comes back, release returns focus')
    } finally {
      await browser.close()
    }
  }

  static connect = `
    window.owner = {}
    window.failures = []
    TacWebApis.events = { send: (owner, name, value) => window.failures.push(value.message) }
    const host = document.getElementById('host')
    TacFocus.connect({
      render: () => Promise.resolve(),
      views: (caller) => caller === window.owner ? [{ owner: 'component', host }] : [],
      owner: (element) => host.contains(element) ? 'component' : null,
    })
    window.focusCall = (call, name) => TacWebApis.calling(window.owner, () => TacFocus.api[call](name))
  `

  static async active(page) {
    return page.evaluate(() => document.activeElement?.id ?? '')
  }

  static async call(page, call, name) {
    await page.evaluate(([callName, value]) => { window.focusCall(callName, value); TacFocus.settle() }, [call, name])
  }

  static async trapKeepsTabInside(page) {
    await page.focus('#open')
    await FocusBrowserCheck.call(page, 'trap', 'dialog')
    assert.equal(await FocusBrowserCheck.active(page), 'name')
    await page.keyboard.press('Tab')
    assert.equal(await FocusBrowserCheck.active(page), 'save')
    await page.keyboard.press('Tab')
    assert.equal(await FocusBrowserCheck.active(page), 'name')
    await page.keyboard.press('Shift+Tab')
    assert.equal(await FocusBrowserCheck.active(page), 'save')
    await page.click('#after')
    assert.equal(await FocusBrowserCheck.active(page), 'name')
  }

  static async releaseReturnsFocus(page) {
    await FocusBrowserCheck.call(page, 'release', null)
    assert.equal(await FocusBrowserCheck.active(page), 'open')
    await page.keyboard.press('Tab')
    assert.equal(await FocusBrowserCheck.active(page), 'name')
    assert.deepEqual(await page.evaluate(() => window.failures), [])
  }
}

await FocusBrowserCheck.run()

#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { chromium, firefox, webkit } from 'playwright'

class FocusBrowserCheck {
  static sources = ['tac-focus.js', 'tac-web.js'].map((name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url))

  static engines = [chromium, firefox, webkit]

  static buttonsTakeFocus = ['chromium', 'firefox']

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
      <shadow-field id="shadow" data-focus="shadow"></shadow-field>
      <passing-field id="passing" data-focus="passing"><input id="passed" aria-label="Passed"></passing-field>
      <span id="label" data-focus="label">Label</span>
    </div>
    <button id="after">After</button>
    <input id="outside" aria-label="Outside">
  </body></html>`

  static fields = `
    customElements.define('shadow-field', class extends HTMLElement {
      constructor() {
        super()
        this.attachShadow({ mode: 'open', delegatesFocus: true }).innerHTML = '<input id="inner" aria-label="Inner">'
      }
    })
    customElements.define('passing-field', class extends HTMLElement {
      focus() { this.querySelector('input').focus() }
    })
  `

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

  static async run() {
    const runtime = (await Promise.all(FocusBrowserCheck.sources.map((source) => readFile(source, 'utf8')))).join('\n')
    for (const engine of FocusBrowserCheck.engines) await FocusBrowserCheck.runEngine(engine, runtime)
  }

  static async runEngine(engine, runtime) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage()
      await page.setContent(FocusBrowserCheck.page)
      await page.addScriptTag({ content: `${FocusBrowserCheck.fields}\n${runtime}\n${FocusBrowserCheck.connect}` })
      await FocusBrowserCheck.trapKeepsFocusInside(page, engine)
      await FocusBrowserCheck.releaseReturnsFocus(page)
      await FocusBrowserCheck.moveIntoShadowRoot(page)
      await FocusBrowserCheck.moveToPassedChild(page)
      await FocusBrowserCheck.moveToLabelFails(page)
      console.log(`focus (${engine.name()}): trap keeps focus inside and release returns focus; a move to an element that passes focus to a child succeeds; a move that cannot focus fails`)
    } finally {
      await browser.close()
    }
  }

  static async active(page) {
    return page.evaluate(() => {
      let active = document.activeElement
      while (active?.shadowRoot?.activeElement) active = active.shadowRoot.activeElement
      return active?.id ?? ''
    })
  }

  static async failures(page) {
    return page.evaluate(() => window.failures.splice(0))
  }

  static async call(page, call, name) {
    await page.evaluate(([callName, value]) => { window.focusCall(callName, value); TacFocus.settle() }, [call, name])
  }

  static async trapKeepsFocusInside(page, engine) {
    await page.focus('#open')
    await FocusBrowserCheck.call(page, 'trap', 'dialog')
    assert.equal(await FocusBrowserCheck.active(page), 'name')
    if (FocusBrowserCheck.buttonsTakeFocus.includes(engine.name())) await FocusBrowserCheck.buttonsStayOutside(page)
    await page.click('#outside')
    assert.equal(await FocusBrowserCheck.active(page), 'name')
  }

  static async buttonsStayOutside(page) {
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
    assert.deepEqual(await FocusBrowserCheck.failures(page), [])
  }

  static async moveIntoShadowRoot(page) {
    await page.focus('#before')
    await FocusBrowserCheck.call(page, 'move', 'shadow')
    assert.equal(await FocusBrowserCheck.active(page), 'inner')
    assert.deepEqual(await FocusBrowserCheck.failures(page), [])
  }

  static async moveToPassedChild(page) {
    await page.focus('#before')
    await FocusBrowserCheck.call(page, 'move', 'passing')
    assert.equal(await FocusBrowserCheck.active(page), 'passed')
    assert.deepEqual(await FocusBrowserCheck.failures(page), [])
  }

  static async moveToLabelFails(page) {
    await page.focus('#before')
    await FocusBrowserCheck.call(page, 'move', 'label')
    assert.equal(await FocusBrowserCheck.active(page), 'before')
    const [message] = await FocusBrowserCheck.failures(page)
    assert.match(message ?? '', /data-focus="label" cannot take focus/)
  }
}

await FocusBrowserCheck.run()

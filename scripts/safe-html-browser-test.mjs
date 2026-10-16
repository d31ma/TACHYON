#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { chromium } from 'playwright'

class SafeHtmlCheck {
  static source = new URL('../crates/tachyon-core/src/template/tac-safe-html.js', import.meta.url)

  static attacks = [
    '<script>window.__pwned = 1</script>',
    '<img src=x onerror="window.__pwned = 2">',
    '<a href="javascript:window.__pwned = 3">x</a>',
    '<a href="  JaVaScRiPt:window.__pwned = 4">x</a>',
    '<a href="java&#x09;script:window.__pwned = 5">x</a>',
    '<iframe srcdoc="<script>parent.__pwned = 6</script>"></iframe>',
    '<svg><script>window.__pwned = 7</script></svg>',
    '<svg onload="window.__pwned = 8"></svg>',
    '<math><mtext><table><mglyph><style><img src=x onerror="window.__pwned = 9">',
    '<div style="background:url(javascript:window.__pwned = 10)">x</div>',
    '<object data="javascript:window.__pwned = 11"></object>',
    '<embed src="javascript:window.__pwned = 12">',
    '<form action="javascript:window.__pwned = 13"><button>x</button></form>',
    '<details open ontoggle="window.__pwned = 14">x</details>',
    '<img src="data:text/html;base64,PHNjcmlwdD5wYXJlbnQuX19wd25lZD0xNTwvc2NyaXB0Pg==">',
    '<noscript><p title="</noscript><img src=x onerror=window.__pwned=16>"></noscript>',
    '<template><img src=x onerror="window.__pwned = 17"></template>',
    '<a href="vbscript:msgbox(18)">x</a>',
    '<style>@import "javascript:window.__pwned = 19";</style>',
    '<meta http-equiv="refresh" content="0;url=javascript:window.__pwned = 20">',
    '<blockquote cite="javascript:window.__pwned = 21">q</blockquote>',
    '<img srcset="javascript:window.__pwned = 22 1x" src="https://example.com/a.png">',
    '<A HREF="JAVASCRIPT:window.__pwned = 23" ONCLICK="window.__pwned = 23">x</A>',
  ]

  static async run() {
    const browser = await chromium.launch()
    try {
      const page = await browser.newPage()
      await page.goto('about:blank')
      await page.setContent('<!doctype html><html><body><div id="out"></div></body></html>')
      await page.addScriptTag({ content: `${await readFile(SafeHtmlCheck.source, 'utf8')}\nwindow.TacSafeHtml = TacSafeHtml` })
      await SafeHtmlCheck.attacksAreInert(page)
      await SafeHtmlCheck.mailIsKept(page)
      await SafeHtmlCheck.limitsHold(page)
      await SafeHtmlCheck.rawTextHostsAreRefused(page)
      console.log('safe-html: attacks are inert, mail formatting is kept, limits hold, raw-text hosts are refused')
    } finally {
      await browser.close()
    }
  }

  static async render(page, html) {
    return page.evaluate((value) => {
      const out = document.getElementById('out')
      out.replaceChildren(window.TacSafeHtml.fragment(value))
      return out.innerHTML
    }, html)
  }

  static async inspect(page, html) {
    return page.evaluate((value) => {
      const out = document.getElementById('out')
      out.replaceChildren(window.TacSafeHtml.fragment(value))
      const elements = [...out.querySelectorAll('*')]
      return {
        tags: elements.map((element) => element.localName),
        handlers: elements.flatMap((element) => element.getAttributeNames().filter((name) => name.startsWith('on'))),
        urls: elements.flatMap((element) => ['href', 'src', 'srcset', 'cite', 'action', 'data', 'srcdoc'].map((name) => element.getAttribute(name)).filter(Boolean)),
        styles: elements.map((element) => element.getAttribute('style')).filter(Boolean),
      }
    }, html)
  }

  static async attacksAreInert(page) {
    const refused = new Set(['script', 'iframe', 'svg', 'math', 'object', 'embed', 'form', 'button', 'style', 'meta', 'template', 'noscript'])
    for (const attack of SafeHtmlCheck.attacks) {
      const found = await SafeHtmlCheck.inspect(page, attack)
      assert.deepEqual(found.handlers, [], attack)
      for (const url of found.urls) assert.match(url, /^(https?:|mailto:|tel:|data:image\/)/i, attack)
      for (const style of found.styles) assert.doesNotMatch(style, /url\(|expression|javascript/i, attack)
      for (const tag of found.tags) assert.ok(!refused.has(tag), `${attack}: ${tag}`)
    }
    await page.waitForTimeout(200)
    assert.equal(await page.evaluate(() => window.__pwned), undefined, 'no attack ran')
  }

  static async mailIsKept(page) {
    const html = await SafeHtmlCheck.render(page, '<p style="color: red; position: fixed; background-image: url(https://t.example/p.gif)">Hi <b>Ada</b></p>'
      + '<table width="100%" cellpadding="4"><tr><td align="center">cell</td></tr></table>'
      + '<a href="https://example.com/x" id="clobber" name="clobber" class="page" onclick="x()">link</a>'
      + '<img src="https://example.com/logo.png" alt="logo"><img src="data:image/png;base64,iVBORw0KGgo=" alt="inline">'
      + '<custom-tag><em>kept text</em></custom-tag>')
    assert.match(html, /<p style="color: red;?">Hi <b>Ada<\/b><\/p>/)
    assert.match(html, /<table width="100%" cellpadding="4"><tbody><tr><td align="center">cell<\/td><\/tr><\/tbody><\/table>/)
    assert.match(html, /<a href="https:\/\/example.com\/x" rel="noopener noreferrer nofollow" target="_blank">link<\/a>/)
    assert.match(html, /<img alt="logo" src="https:\/\/example.com\/logo.png">/)
    assert.match(html, /<img alt="inline" src="data:image\/png;base64,iVBORw0KGgo=">/)
    assert.match(html, /<em>kept text<\/em>/)
    assert.doesNotMatch(html, /custom-tag|clobber|class=|position|url\(/)
    const styled = await SafeHtmlCheck.render(page, '<span style="text-decoration: underline; white-space: pre; border-radius: 4px">s</span>')
    assert.match(styled, /text-decoration-line: underline/, styled)
    assert.match(styled, /white-space(?:-collapse)?: pre/, styled)
    assert.match(styled, /border-top-left-radius: 4px/, styled)
  }

  static async rawTextHostsAreRefused(page) {
    const refused = await page.evaluate(() => ['style', 'textarea', 'title', 'noscript'].map((tag) => {
      try { window.TacSafeHtml.fill(document.createElement(tag), '<b>x</b>'); return null } catch (error) { return error.name }
    }))
    assert.deepEqual(refused, ['TypeError', 'TypeError', 'TypeError', 'TypeError'])
    const filled = await page.evaluate(() => window.TacSafeHtml.fill(document.createElement('div'), '<b>x</b>').childNodes.length)
    assert.equal(filled, 1)
  }

  static async limitsHold(page) {
    const refused = await page.evaluate(() => {
      try { window.TacSafeHtml.fragment('x'.repeat(1048577)); return null } catch (error) { return error.name }
    })
    assert.equal(refused, 'RangeError')
    const deep = await page.evaluate(() => {
      try { window.TacSafeHtml.fragment('<b>x</b>'.repeat(20001)); return null } catch (error) { return error.name }
    })
    assert.equal(deep, 'RangeError')
  }
}

await SafeHtmlCheck.run()

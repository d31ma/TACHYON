import { chromium } from 'playwright'
import { withServer } from '/Users/iyor/Library/CloudStorage/Dropbox/myOrganizations/DELMA/TACHYON/scripts/release/server-probe.mjs'
const binary = '/Volumes/ANNEX/build/cargo-target/debug/ty'
const browser = await chromium.launch()
try {
  await withServer(binary, ['preview', '/tmp/svc-scratch', '--static', '--port', '0'], async request => {
    const origin = new URL((await request('/')).url).origin
    const page = await browser.newPage()
    const errors = []
    page.on('pageerror', e => errors.push(e.message))
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()) })
    await page.goto(origin + '/')
    await page.waitForFunction(() => document.querySelectorAll('[data-tachyon-active="true"]').length === 2)
    console.log(await page.locator('body').innerText())
    await page.locator('span').first().click()
    await page.waitForTimeout(200)
    console.log('after click:', await page.locator('body').innerText())
    console.log(await page.evaluate(() => Object.keys(sessionStorage)))
    console.log('errors', errors)
    await page.close()
  })
} finally { await browser.close() }

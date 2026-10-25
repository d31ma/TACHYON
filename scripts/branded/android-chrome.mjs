import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { setTimeout as sleep } from 'node:timers/promises'

/**
 * Chrome on an Android emulator or device, reached over adb and the Chrome DevTools protocol
 * (`chrome_devtools_remote`), shaped like a Playwright browser. Loopback pages the gates serve on
 * the host reach the device through `adb reverse` on the same port.
 */
export class AndroidChrome {
  static package = process.env.TAC_BRANDED_ANDROID_PACKAGE ?? 'com.android.chrome'
  static activity = 'com.google.android.apps.chrome.Main'
  static socket = 'chrome_devtools_remote'
  static flags = '_ --disable-fre --no-default-browser-check --no-first-run --disable-features=SigninPromo'
  static commandLine = '/data/local/tmp/chrome-command-line'
  static startMilliseconds = 60_000
  static pollMilliseconds = 500
  static loopback = new Set(['127.0.0.1', 'localhost', '[::1]'])

  /** @param {string[]} args */
  static async adb(args) {
    const serial = process.env.ANDROID_SERIAL ? ['-s', process.env.ANDROID_SERIAL] : []
    const { stdout } = await promisify(execFile)(process.env.TAC_BRANDED_ADB ?? 'adb', [...serial, ...args], { timeout: 60_000 })
    return stdout.trim()
  }

  /** @param {import('playwright').BrowserType} chromium */
  static async launch(chromium) {
    await AndroidChrome.adb(['shell', `echo '${AndroidChrome.flags}' > ${AndroidChrome.commandLine}`])
    await AndroidChrome.adb(['shell', 'am', 'set-debug-app', '--persistent', AndroidChrome.package])
    await AndroidChrome.adb(['shell', 'am', 'force-stop', AndroidChrome.package])
    await AndroidChrome.adb(['shell', 'am', 'start', '-n', `${AndroidChrome.package}/${AndroidChrome.activity}`, '-d', 'about:blank'])
    const port = Number(await AndroidChrome.adb(['forward', 'tcp:0', `localabstract:${AndroidChrome.socket}`]))
    const endpoint = `http://127.0.0.1:${port}`
    const deadline = Date.now() + AndroidChrome.startMilliseconds
    let version
    while (!version && Date.now() < deadline) {
      version = await fetch(`${endpoint}/json/version`).then((response) => response.json(), () => undefined)
      if (!version) await sleep(AndroidChrome.pollMilliseconds)
    }
    if (!version) throw new Error(`Chrome on the Android device did not open ${AndroidChrome.socket} within ${AndroidChrome.startMilliseconds} ms`)
    const browser = await chromium.connectOverCDP(endpoint)
    return new AndroidChrome(browser, port, version).wrap()
  }

  /** @param {import('playwright').Browser} browser @param {number} port @param {Record<string,string>} version */
  constructor(browser, port, version) {
    this.browser = browser
    this.port = port
    this.details = version
    this.userAgent = version['User-Agent']
    /** @type {Set<number>} */
    this.reversed = new Set()
    /** @type {Set<import('playwright').Page>} */
    this.opened = new Set()
  }

  /** @param {string} url */
  async reach(url) {
    const parsed = URL.canParse(url) ? new URL(url) : undefined
    const port = Number(parsed?.port)
    if (!parsed || !AndroidChrome.loopback.has(parsed.hostname) || !port || this.reversed.has(port)) return
    await AndroidChrome.adb(['reverse', `tcp:${port}`, `tcp:${port}`])
    this.reversed.add(port)
  }

  /** @param {import('playwright').Page} page */
  page(page) {
    this.opened.add(page)
    const goto = page.goto.bind(page)
    page.goto = async (url, options) => {
      await this.reach(url)
      return goto(url, options)
    }
    return page
  }

  /** @param {import('playwright').BrowserContext} context @param {boolean} shared */
  context(context, shared) {
    const newPage = context.newPage.bind(context)
    const close = context.close.bind(context)
    context.newPage = async () => this.page(await newPage())
    context.close = async () => {
      if (!shared) return close()
      for (const page of context.pages().filter((open) => this.opened.has(open))) await page.close()
    }
    return context
  }

  wrap() {
    const browser = this.browser
    const newContext = browser.newContext.bind(browser)
    const close = browser.close.bind(browser)
    browser.newContext = async (options) => {
      const own = await newContext(options).then((context) => this.context(context, false), (error) => {
        console.error(`branded: Chrome on Android gave no new browser context (${error.message}); using its default context`)
        return undefined
      })
      return own ?? this.context(browser.contexts()[0], true)
    }
    browser.newPage = async (options) => (await browser.newContext(options)).newPage()
    browser.close = async () => {
      for (const page of this.opened) await page.close().catch(() => {})
      for (const port of this.reversed) await AndroidChrome.adb(['reverse', '--remove', `tcp:${port}`]).catch(() => {})
      await AndroidChrome.adb(['forward', '--remove', `tcp:${this.port}`]).catch(() => {})
      await close()
    }
    browser.userAgent = this.userAgent
    return browser
  }
}

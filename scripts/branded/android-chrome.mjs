import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { setTimeout as sleep } from 'node:timers/promises'
import { BrandedUnsupported } from './webdriver-browser.mjs'

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
  static mediaOptions = ['colorScheme', 'reducedMotion', 'forcedColors']
  static pageOptions = [...AndroidChrome.mediaOptions, 'locale', 'timezoneId', 'viewport', 'extraHTTPHeaders', 'userAgent']
  static contextOptions = ['permissions', 'geolocation', 'offline']

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

  /** @param {string} url */
  unreached(url) {
    const parsed = new URL(url)
    return AndroidChrome.loopback.has(parsed.hostname) && Number(parsed.port) > 0 && !this.reversed.has(Number(parsed.port))
  }

  /**
   * Chrome on Android gives no new browser context over CDP, so the options a gate passes to
   * newContext are applied to each page of the shared context instead; any other option refuses.
   * @param {Record<string, any>} options
   */
  static checkOptions(options) {
    const known = [...AndroidChrome.pageOptions, ...AndroidChrome.contextOptions]
    const unknown = Object.keys(options).filter((name) => options[name] !== undefined && !known.includes(name))
    if (unknown.length) throw new BrandedUnsupported('Chrome on Android', `browser.newContext({ ${unknown.join(', ')} })`, 'its one shared browser context cannot take these options')
  }

  /** @param {import('playwright').Page} page @param {Record<string, any>} options */
  static async applyPageOptions(page, options) {
    const media = Object.fromEntries(AndroidChrome.mediaOptions.filter((name) => options[name] !== undefined).map((name) => [name, options[name]]))
    if (Object.keys(media).length) await page.emulateMedia(media)
    if (options.viewport) await page.setViewportSize(options.viewport)
    if (options.extraHTTPHeaders) await page.setExtraHTTPHeaders(options.extraHTTPHeaders)
    if (!options.locale && !options.timezoneId && !options.userAgent) return
    const session = await page.context().newCDPSession(page)
    if (options.locale) await session.send('Emulation.setLocaleOverride', { locale: options.locale })
    if (options.timezoneId) await session.send('Emulation.setTimezoneOverride', { timezoneId: options.timezoneId })
    const userAgent = options.userAgent ?? await page.evaluate(() => navigator.userAgent)
    if (options.userAgent || options.locale) await session.send('Emulation.setUserAgentOverride', { userAgent, acceptLanguage: options.locale })
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

  /**
   * A page may reach a loopback port the gate never navigated to (a Yon origin beside the page),
   * so a request to a port without an adb reverse route opens one before it continues.
   * @param {import('playwright').BrowserContext} context @param {boolean} shared @param {Record<string, any>} options
   */
  async context(context, shared, options = {}) {
    const newPage = context.newPage.bind(context)
    const close = context.close.bind(context)
    const unreached = (url) => this.unreached(String(url))
    const reachFirst = async (route) => {
      await this.reach(route.request().url())
      await route.fallback()
    }
    await context.route(unreached, reachFirst)
    if (shared) {
      AndroidChrome.checkOptions(options)
      if (options.permissions) await context.grantPermissions(options.permissions)
      if (options.geolocation) await context.setGeolocation(options.geolocation)
      if (options.offline !== undefined) await context.setOffline(options.offline)
    }
    context.newPage = async () => {
      const page = this.page(await newPage())
      if (shared) await AndroidChrome.applyPageOptions(page, options)
      return page
    }
    context.close = async () => {
      await context.unroute(unreached, reachFirst)
      if (!shared) return close()
      for (const page of context.pages().filter((open) => this.opened.has(open))) await page.close()
      await context.clearPermissions()
      await context.setOffline(false)
    }
    return context
  }

  wrap() {
    const browser = this.browser
    const newContext = browser.newContext.bind(browser)
    const close = browser.close.bind(browser)
    browser.newContext = async (options = {}) => {
      const own = await newContext(options).catch((error) => {
        console.error(`branded: Chrome on Android gave no new browser context (${error.message}); using its default context with the options applied to each page`)
        return undefined
      })
      return own ? this.context(own, false) : this.context(browser.contexts()[0], true, options)
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

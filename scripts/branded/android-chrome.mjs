import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { setTimeout as sleep } from 'node:timers/promises'
import { BrandedUnsupported } from './webdriver-browser.mjs'
import { ShareSheet } from './share-sheet.mjs'

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
  static deviceOptions = { hasTouch: true, isMobile: true }
  static shareSheetWindows = ['ChooserActivity', 'ResolverActivity']
  static shareSheetMilliseconds = 5_000

  /** @param {string[]} args */
  static async adb(args) {
    const serial = process.env.ANDROID_SERIAL ? ['-s', process.env.ANDROID_SERIAL] : []
    const { stdout } = await promisify(execFile)(process.env.TAC_BRANDED_ADB ?? 'adb', [...serial, ...args], { timeout: 60_000 })
    return stdout.trim()
  }

  static hostRules = '--host-resolver-rules='

  /** @param {string[]} args @returns {string[]} */
  static mappedHosts(args) {
    const rules = args.filter((argument) => argument.startsWith(AndroidChrome.hostRules)).flatMap((argument) => argument.slice(AndroidChrome.hostRules.length).split(','))
    return rules.map((rule) => rule.trim().split(/\s+/)).filter(([verb, , target]) => verb === 'MAP' && AndroidChrome.loopback.has(target)).map(([, host]) => host)
  }

  /** @param {import('playwright').BrowserType} chromium @param {{args?:string[]}} [options] */
  static async launch(chromium, options = {}) {
    const args = options.args ?? []
    const ignored = args.filter((argument) => !argument.startsWith(AndroidChrome.hostRules))
    if (ignored.length) console.error(`branded: Chrome on Android takes only ${AndroidChrome.hostRules} from the launch arguments; ignoring ${ignored.join(' ')}`)
    const rules = args.filter((argument) => argument.startsWith(AndroidChrome.hostRules)).map((argument) => `${AndroidChrome.hostRules}"${argument.slice(AndroidChrome.hostRules.length)}"`)
    const flags = [AndroidChrome.flags, ...rules].join(' ')
    await AndroidChrome.adb(['shell', `echo '${flags}' > ${AndroidChrome.commandLine}`])
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
    return new AndroidChrome(browser, port, version, AndroidChrome.mappedHosts(args)).wrap()
  }

  /** @param {import('playwright').Browser} browser @param {number} port @param {Record<string,string>} version @param {string[]} mapped */
  constructor(browser, port, version, mapped) {
    this.browser = browser
    this.loopback = new Set([...AndroidChrome.loopback, ...mapped])
    this.port = port
    this.details = version
    this.userAgent = version['User-Agent']
    /** @type {Set<number>} */
    this.reversed = new Set()
    /** @type {Set<import('playwright').Page>} */
    this.opened = new Set()
    /** @type {Set<string>} */
    this.visited = new Set()
    /** @type {Set<import('playwright').BrowserContext>} */
    this.routed = new Set()
    this.views = 0
  }

  /** @param {string} url */
  async reach(url) {
    const parsed = URL.canParse(url) ? new URL(url) : undefined
    const port = Number(parsed?.port)
    if (!parsed || !this.loopback.has(parsed.hostname) || !port || this.reversed.has(port)) return
    await AndroidChrome.adb(['reverse', `tcp:${port}`, `tcp:${port}`])
    this.reversed.add(port)
  }

  /** @param {string} url */
  unreached(url) {
    const parsed = new URL(url)
    return this.loopback.has(parsed.hostname) && Number(parsed.port) > 0 && !this.reversed.has(Number(parsed.port))
  }

  /**
   * Chrome on Android gives no new browser context over CDP, so the options a gate passes to
   * newContext are applied to each page of the shared context instead; any other option refuses.
   * @param {Record<string, any>} options
   */
  static checkOptions(options) {
    const known = [...AndroidChrome.pageOptions, ...AndroidChrome.contextOptions]
    const asDevice = (name) => Object.hasOwn(AndroidChrome.deviceOptions, name) && options[name] === AndroidChrome.deviceOptions[name]
    const unknown = Object.keys(options).filter((name) => options[name] !== undefined && !known.includes(name) && !asDevice(name))
    if (unknown.length) throw new BrandedUnsupported('Chrome on Android', `browser.newContext({ ${unknown.map((name) => `${name}: ${JSON.stringify(options[name])}`).join(', ')} })`, 'its one shared browser context cannot take these options, and a phone is always a touch device')
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

  /**
   * The system share sheet waits for a person; pressing Back closes it, as a person would, and
   * Chrome then rejects navigator.share() with an AbortError. Back is pressed only while the sheet
   * has the focus, so it never moves the page back.
   */
  static async closeShareSheet() {
    const deadline = Date.now() + AndroidChrome.shareSheetMilliseconds
    let focused = await AndroidChrome.focusedWindow()
    let sheet = AndroidChrome.shareSheetWindows.some((name) => focused.includes(name))
    while (!sheet && Date.now() < deadline) {
      await sleep(AndroidChrome.pollMilliseconds)
      focused = await AndroidChrome.focusedWindow()
      sheet = AndroidChrome.shareSheetWindows.some((name) => focused.includes(name))
    }
    console.error(`branded: navigator.share() was still waiting after ${ShareSheet.closeMilliseconds} ms; ${sheet ? 'pressing Back on the Android share sheet' : `no share sheet has the focus (${focused}), so it was closed for the person with an AbortError`}`)
    if (sheet) await AndroidChrome.adb(['shell', 'input', 'keyevent', 'KEYCODE_BACK'])
    return sheet
  }

  /** @returns {Promise<string>} */
  static async focusedWindow() {
    const windows = await AndroidChrome.adb(['shell', 'dumpsys', 'window'])
    return windows.split('\n').find((line) => line.includes('mCurrentFocus'))?.trim() ?? ''
  }

  /** @param {import('playwright').Page} page */
  async page(page) {
    await ShareSheet.install(page, AndroidChrome.closeShareSheet)
    this.opened.add(page)
    const goto = page.goto.bind(page)
    page.goto = async (url, options) => {
      await this.reach(url)
      if (URL.canParse(url) && new URL(url).origin !== 'null') this.visited.add(new URL(url).origin)
      return goto(url, options)
    }
    return page
  }

  /**
   * A page may reach a loopback port the gate never navigated to (a Yon origin beside the page),
   * so a request to a port without an adb reverse route opens one before it continues.
   * @param {import('playwright').BrowserContext} context
   */
  async route(context) {
    if (this.routed.has(context)) return
    this.routed.add(context)
    await context.route((url) => this.unreached(String(url)), async (route) => {
      await this.reach(route.request().url())
      await route.fallback()
    })
  }

  /** @param {import('playwright').BrowserContext} context */
  async own(context) {
    await this.route(context)
    const newPage = context.newPage.bind(context)
    context.newPage = async () => await this.page(await newPage())
    return context
  }

  /**
   * Chrome on Android gives no new browser context over CDP, so each context a gate asks for is a
   * view of the one shared context: its own pages and options, the shared storage and permissions.
   * @param {import('playwright').BrowserContext} context @param {Record<string, any>} options
   */
  async shared(context, options) {
    AndroidChrome.checkOptions(options)
    await this.route(context)
    if (options.permissions) await context.grantPermissions(options.permissions)
    if (options.geolocation) await context.setGeolocation(options.geolocation)
    if (options.offline !== undefined) await context.setOffline(options.offline)
    this.views += 1
    /** @type {Set<import('playwright').Page>} */
    const pages = new Set()
    const newPage = async () => {
      const page = await this.page(await context.newPage())
      pages.add(page)
      page.once('close', () => pages.delete(page))
      await AndroidChrome.applyPageOptions(page, options)
      return page
    }
    let closed = false
    const close = async () => {
      if (closed) return
      closed = true
      this.views -= 1
      if (this.views === 0) await this.forget(context, [...pages][0])
      for (const page of [...pages]) await page.close()
      if (this.views > 0) return
      await context.clearPermissions()
      await context.setOffline(false)
    }
    const view = { newPage, close, pages: () => [...pages] }
    return new Proxy(context, {
      get(target, property) {
        if (Object.hasOwn(view, property)) return view[property]
        const value = Reflect.get(target, property)
        return typeof value === 'function' ? value.bind(target) : value
      },
    })
  }

  /**
   * A new Playwright context starts with empty storage. Here every context shares Chrome's one
   * context, so when the last of them closes, the storage and cookies of the origins they visited
   * are cleared.
   * @param {import('playwright').BrowserContext} context @param {import('playwright').Page|undefined} open
   */
  async forget(context, open) {
    if (this.visited.size === 0) return
    const page = open ?? await context.newPage()
    const session = await context.newCDPSession(page)
    for (const origin of this.visited) await session.send('Storage.clearDataForOrigin', { origin, storageTypes: 'all' })
    await session.detach()
    if (!open) await page.close()
    await context.clearCookies()
    this.visited.clear()
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
      return own ? this.own(own) : this.shared(browser.contexts()[0], options)
    }
    browser.newPage = async (options) => {
      const context = await browser.newContext(options)
      const page = await context.newPage()
      page.once('close', () => context.close().catch(() => {}))
      return page
    }
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

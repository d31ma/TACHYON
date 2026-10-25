import { appendFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { AndroidChrome } from './android-chrome.mjs'
import { ShareSheet } from './share-sheet.mjs'
import { BrandedBrowser, BrandedUnsupported } from './webdriver-browser.mjs'

/**
 * Loaded with `node --import` by scripts/branded-browser-gate.mjs. It points the Playwright
 * launcher a gate asks for at the branded browser named by TAC_BRANDED_BROWSER, and makes the
 * launchers the run does not cover refuse, so a gate can only pass in the branded browser.
 */
class BrandedLaunch {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
  static engines = ['chromium', 'firefox', 'webkit']
  static webkitDriver = process.env.TAC_BRANDED_WEBKIT_BROWSER ?? '/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/MiniBrowser'

  static plans = {
    chrome: { engine: 'chromium', label: 'Google Chrome', channel: 'chrome', closeShareSheet: ShareSheet.withoutSheet('Google Chrome'), launch: (type, options) => type.launch({ ...options, channel: 'chrome' }) },
    msedge: { engine: 'chromium', label: 'Microsoft Edge', channel: 'msedge', closeShareSheet: ShareSheet.withoutSheet('Microsoft Edge'), launch: (type, options) => type.launch({ ...options, channel: 'msedge' }) },
    'android-chrome': { engine: 'chromium', label: 'Chrome on Android', launch: (type, options) => AndroidChrome.launch(type, options) },
    safari: {
      engine: 'webkit',
      label: 'Safari',
      launch: () => BrandedBrowser.launch({ name: 'Safari', engine: 'webkit', driver: process.env.TAC_BRANDED_SAFARIDRIVER ?? '/usr/bin/safaridriver', args: ['--port', '{port}'], capabilities: { browserName: 'safari' } }),
    },
    'ios-safari': {
      engine: 'webkit',
      label: 'Safari on the iOS simulator',
      launch: () => BrandedBrowser.launch({ name: 'Safari on the iOS simulator', engine: 'webkit', driver: process.env.TAC_BRANDED_SAFARIDRIVER ?? '/usr/bin/safaridriver', args: ['--port', '{port}'], capabilities: BrandedLaunch.simulator() }),
    },
    'webkitgtk-webdriver': {
      engine: 'webkit',
      label: 'WebKitGTK MiniBrowser over WebDriver (a stand-in for checking the Safari adapter off a Mac)',
      launch: () => BrandedBrowser.launch({ name: 'WebKitGTK MiniBrowser', engine: 'webkit', driver: 'WebKitWebDriver', args: ['--port={port}'], capabilities: { browserName: 'MiniBrowser', 'webkitgtk:browserOptions': { binary: BrandedLaunch.webkitDriver, args: ['--automation'] } } }),
    },
  }

  static simulator() {
    const udid = process.env.TAC_BRANDED_SIMULATOR_UDID
    return { browserName: 'safari', platformName: 'iOS', 'safari:useSimulator': true, ...(udid ? { 'safari:deviceUDID': udid } : { 'safari:deviceType': 'iPhone' }) }
  }

  /** @param {string} name @param {import('playwright').Browser & {userAgent?:string}} browser */
  static record(name, browser) {
    const receipt = process.env.TAC_BRANDED_RECEIPT
    const version = browser.version()
    const label = BrandedLaunch.plans[name].label
    const line = { browser: name, version: /^\d/.test(version) ? `${label} ${version}` : version, userAgent: browser.userAgent ?? null, gate: path.relative(BrandedLaunch.repository, process.argv[1] ?? ''), at: new Date().toISOString() }
    console.error(`branded: launched ${line.version} for ${name} (${line.userAgent ?? 'no user agent read'})`)
    if (receipt) appendFileSync(receipt, `${JSON.stringify(line)}\n`)
  }

  /** @param {import('playwright').Browser} browser */
  static traceFailures(browser) {
    const watch = (owner) => owner.on('response', (response) => {
      if (response.status() >= 400) console.error(`branded: ${response.status()} for ${response.url()}`)
    })
    const newContext = browser.newContext.bind(browser)
    const newPage = browser.newPage.bind(browser)
    browser.newContext = async (options) => {
      const context = await newContext(options)
      watch(context)
      return context
    }
    browser.newPage = async (options) => {
      const page = await newPage(options)
      watch(page)
      return page
    }
    return browser
  }

  /** @param {import('playwright').Browser} browser @param {() => Promise<boolean>} close */
  static closeShareSheets(browser, close) {
    const newContext = browser.newContext.bind(browser)
    browser.newContext = async (options) => {
      const context = await newContext(options)
      await ShareSheet.install(context, close)
      return context
    }
    return browser
  }

  static install() {
    const name = process.env.TAC_BRANDED_BROWSER ?? ''
    const plan = BrandedLaunch.plans[name]
    if (!plan) throw new Error(`TAC_BRANDED_BROWSER must be one of ${Object.keys(BrandedLaunch.plans).join(', ')}, not ${JSON.stringify(name)}`)
    const playwright = createRequire(path.join(BrandedLaunch.repository, 'package.json'))('playwright')
    for (const engine of BrandedLaunch.engines) {
      const type = playwright[engine]
      const launch = type.launch.bind(type)
      const persistent = type.launchPersistentContext.bind(type)
      const refuse = (call) => async () => {
        throw new BrandedUnsupported(plan.label, `${engine}.${call}()`, `this run covers ${plan.label}, which stands in for ${plan.engine} only`)
      }
      type.launchPersistentContext = refuse('launchPersistentContext')
      type.launchServer = refuse('launchServer')
      type.connect = refuse('connect')
      if (engine !== plan.engine) {
        type.launch = refuse('launch')
        continue
      }
      if (plan.channel) {
        type.launchPersistentContext = async (folder, options = {}) => {
          const context = await persistent(folder, { ...options, channel: plan.channel })
          BrandedLaunch.record(name, context.browser() ?? { version: () => `${plan.label} (persistent context)` })
          return context
        }
      }
      type.launch = async (options = {}) => {
        const browser = await plan.launch({ launch, connectOverCDP: type.connectOverCDP.bind(type) }, options)
        BrandedLaunch.record(name, browser)
        if (plan.engine === 'chromium') BrandedLaunch.traceFailures(browser)
        if (plan.closeShareSheet) BrandedLaunch.closeShareSheets(browser, plan.closeShareSheet)
        return browser
      }
    }
  }
}

BrandedLaunch.install()

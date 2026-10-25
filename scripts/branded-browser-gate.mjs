#!/usr/bin/env node
import { createWriteStream, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { finished } from 'node:stream/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { SupervisedCommand } from './supervised-command.mjs'
import { BrandedUnsupported } from './branded/webdriver-browser.mjs'

/** @typedef {{name:string,file:string,engines:string[],webdriverNeeds?:string,androidNeeds?:string}} BrandedGate */
/** @typedef {{gate:string,outcome:'pass'|'fail'|'unsupported',seconds:number,log:string,browsers:string[],note?:string}} BrandedResult */

/**
 * Runs the Tac web gates in a branded browser on its own system (row 65 of the iteration-4 runs
 * owed): Safari on macOS and on the iOS simulator, Chrome and Edge on Windows, Chrome on Android.
 * Each gate runs unchanged; scripts/branded/preload.mjs points its Playwright launcher at the
 * branded browser and refuses the engines the run does not cover.
 *
 *   node scripts/branded-browser-gate.mjs --browser=safari [--only=page-program,held-clients] [--logs=dir]
 */
class BrandedBrowserGate {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  static preload = path.join(BrandedBrowserGate.repository, 'scripts/branded/preload.mjs')
  static webdriver = ['safari', 'ios-safari', 'webkitgtk-webdriver']
  static browsers = { safari: 'webkit', 'ios-safari': 'webkit', chrome: 'chromium', msedge: 'chromium', 'android-chrome': 'chromium', 'webkitgtk-webdriver': 'webkit' }
  static engineVariables = [
    'TAC_PAGE_PROGRAM_BROWSERS', 'TAC_ASYNC_LISTENER_BROWSERS', 'TAC_HELD_CLIENT_BROWSERS', 'TAC_WINDOW_BROWSERS', 'TAC_CACHED_CLIENT_BROWSERS',
    'TAC_WEB_APIS_BROWSERS', 'TAC_STORAGE_BROWSERS', 'TAC_COMPONENT_SCOPE_BROWSERS', 'TAC_LOGIC_CONTROLS_BROWSERS', 'TAC_SVG_NAMESPACE_BROWSERS',
    'TAC_SCAFFOLD_BROWSERS', 'TAC_WASM_OFFLINE_BROWSERS', 'TAC_ADAPTER_BROWSERS', 'TAC_AUTHORED_NODE_BROWSERS', 'TAC_OFFLINE_BROWSERS', 'TAC_YON_READS_BROWSERS', 'TAC_BROWSER_HTTP_BROWSERS', 'TAC_PYTHON_BROWSERS',
  ]

  /** @type {BrandedGate[]} */
  static gates = [
    { name: 'adapter', file: 'scripts/branded/adapter-check.mjs', engines: ['chromium', 'webkit'] },
    { name: 'page-program', file: 'scripts/page-program-browser-gate.mjs', engines: ['chromium', 'webkit'] },
    { name: 'async-listener', file: 'scripts/async-listener-browser-gate.mjs', engines: ['chromium', 'webkit'] },
    { name: 'held-clients', file: 'scripts/held-clients-browser-gate.mjs', engines: ['chromium', 'webkit'] },
    { name: 'cached-client', file: 'scripts/cached-client-browser-test.mjs', engines: ['chromium', 'webkit'] },
    { name: 'window-controls', file: 'scripts/window-controls-browser-test.mjs', engines: ['chromium', 'webkit'], webdriverNeeds: 'context.addInitScript()' },
    { name: 'web-apis', file: 'scripts/web-apis-browser-test.mjs', engines: ['chromium'] },
    { name: 'storage', file: 'scripts/storage-browser-test.mjs', engines: ['chromium', 'webkit'], webdriverNeeds: 'context.addInitScript(), context.setOffline() and several open contexts', androidNeeds: 'several open browser contexts, each with its own storage' },
    { name: 'component-scope', file: 'scripts/component-scope-browser-gate.mjs', engines: ['chromium', 'webkit'] },
    { name: 'logic-controls', file: 'scripts/logic-controls-browser-gate.mjs', engines: ['chromium', 'webkit'] },
    { name: 'svg-namespace', file: 'scripts/svg-namespace-browser-gate.mjs', engines: ['chromium', 'webkit'], webdriverNeeds: 'page.route()' },
    { name: 'scaffold', file: 'scripts/scaffold-browser-gate.mjs', engines: ['chromium', 'webkit'] },
    { name: 'authored-node', file: 'scripts/authored-node-browser-gate.mjs', engines: ['chromium', 'webkit'], webdriverNeeds: 'page.addInitScript() and page.route()' },
    { name: 'yon-reads', file: 'scripts/yon-reads-browser-test.mjs', engines: ['chromium', 'webkit'] },
    { name: 'preview-offline', file: 'scripts/preview-offline-browser-gate.mjs', engines: ['chromium', 'webkit'], webdriverNeeds: 'launchPersistentContext() and context.setOffline()' },
    { name: 'wasm-offline', file: 'scripts/wasm-offline-browser-gate.mjs', engines: ['chromium', 'webkit'], webdriverNeeds: 'launchPersistentContext() and context.setOffline()', androidNeeds: 'a persistent context' },
    { name: 'pwa-install', file: 'scripts/pwa-install-browser-gate.mjs', engines: ['chromium'], androidNeeds: 'a persistent context and a DevTools Protocol session' },
    { name: 'website', file: 'scripts/website-browser-test.mjs', engines: ['chromium'] },
    { name: 'runtime', file: 'scripts/runtime-browser-test.mjs', engines: ['chromium'] },
  ]

  /** @param {string} prefix */
  static option(prefix) {
    return process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length)
  }

  /** @param {string} browser @returns {BrandedGate[]} */
  static chosen(browser) {
    const engine = BrandedBrowserGate.browsers[browser]
    const named = BrandedBrowserGate.option('--only=')?.split(',').filter((name) => name !== '')
    const unknown = (named ?? []).filter((name) => !BrandedBrowserGate.gates.some((gate) => gate.name === name))
    if (unknown.length) throw new Error(`no branded gate named ${unknown.join(', ')}; the gates are ${BrandedBrowserGate.gates.map((gate) => gate.name).join(', ')}`)
    const gates = BrandedBrowserGate.gates.filter((gate) => named?.includes(gate.name) ?? (gate.engines.includes(engine) && !BrandedBrowserGate.needs(gate, browser)))
    const wrongEngine = gates.filter((gate) => !gate.engines.includes(engine))
    if (wrongEngine.length) throw new Error(`${wrongEngine.map((gate) => gate.name).join(', ')} only launch ${wrongEngine[0].engines.join(' and ')}, which ${browser} does not stand in for`)
    return gates
  }

  /** @param {BrandedGate} gate @param {string} browser @returns {string|undefined} */
  static needs(gate, browser) {
    if (BrandedBrowserGate.webdriver.includes(browser)) return gate.webdriverNeeds
    return browser === 'android-chrome' ? gate.androidNeeds : undefined
  }

  /** @param {string} browser @param {string} engine @param {string} receipt */
  static environment(browser, engine, receipt) {
    const engines = Object.fromEntries(BrandedBrowserGate.engineVariables.map((name) => [name, engine]))
    return { TAC_BIN: CargoArtifacts.path('release/ty'), ...process.env, ...engines, TAC_BRANDED_BROWSER: browser, TAC_BRANDED_RECEIPT: receipt }
  }

  /** @param {string} receipt @returns {string[]} */
  static launched(receipt) {
    if (!existsSync(receipt)) return []
    const lines = readFileSync(receipt, 'utf8').split('\n').filter(Boolean).map((line) => JSON.parse(line))
    return [...new Set(lines.map((line) => `${line.version}${line.userAgent ? ` (${line.userAgent})` : ''}`))]
  }

  /** @param {BrandedGate} gate @param {string} browser @param {string} logs @returns {Promise<BrandedResult>} */
  static async run(gate, browser, logs) {
    const log = path.join(logs, `${gate.name}.txt`)
    const receipt = path.join(logs, `${gate.name}.launches.jsonl`)
    rmSync(receipt, { force: true })
    const output = createWriteStream(log)
    const started = Date.now()
    const timeoutMilliseconds = Number(BrandedBrowserGate.option('--timeout-ms=') ?? SupervisedCommand.defaultTimeoutMilliseconds)
    const env = BrandedBrowserGate.environment(browser, BrandedBrowserGate.browsers[browser], receipt)
    const command = await SupervisedCommand.start(process.execPath, [`--import=${pathToFileURL(BrandedBrowserGate.preload).href}`, gate.file], { cwd: BrandedBrowserGate.repository, env, timeoutMilliseconds })
    command.child.stdout.pipe(output, { end: false })
    command.child.stderr.pipe(output, { end: false })
    const result = await command.completion
    output.end()
    await finished(output)
    const text = readFileSync(log, 'utf8')
    const browsers = BrandedBrowserGate.launched(receipt)
    const seconds = Math.round((Date.now() - started) / 1000)
    const unsupported = text.split('\n').find((line) => line.includes(BrandedUnsupported.marker))
    if (unsupported) return { gate: gate.name, outcome: 'unsupported', seconds, log, browsers, note: unsupported.trim() }
    const passed = result.status === 0 && result.cleanupConfirmed && !result.error
    if (passed && browsers.length === 0) return { gate: gate.name, outcome: 'fail', seconds, log, browsers, note: 'the gate exited 0 without launching the branded browser' }
    return { gate: gate.name, outcome: passed ? 'pass' : 'fail', seconds, log, browsers, note: passed ? undefined : (result.error?.message ?? `exit ${result.status ?? result.signal}`) }
  }

  static async main() {
    const browser = BrandedBrowserGate.option('--browser=') ?? process.env.TAC_BRANDED_BROWSER ?? ''
    if (!(browser in BrandedBrowserGate.browsers)) throw new Error(`--browser= must be one of ${Object.keys(BrandedBrowserGate.browsers).join(', ')}`)
    const logs = path.resolve(BrandedBrowserGate.option('--logs=') ?? path.join(tmpdir(), `tachyon-branded-${browser}`))
    mkdirSync(logs, { recursive: true })
    /** @type {BrandedResult[]} */
    const results = []
    const named = BrandedBrowserGate.option('--only=') !== undefined
    const engine = BrandedBrowserGate.browsers[browser]
    const left = named ? [] : BrandedBrowserGate.gates.filter((gate) => gate.engines.includes(engine) && BrandedBrowserGate.needs(gate, browser))
    for (const gate of left) console.log(`LEFT OUT ${gate.name}: it needs ${BrandedBrowserGate.needs(gate, browser)}, which ${browser} cannot give a gate (name it in --only= to try anyway)`)
    for (const gate of BrandedBrowserGate.chosen(browser)) {
      const result = await BrandedBrowserGate.run(gate, browser, logs)
      results.push(result)
      console.log(`${result.outcome.toUpperCase()} ${gate.name} ${result.seconds}s in ${result.browsers.join('; ') || 'no branded browser'}${result.note ? ` - ${result.note}` : ''} (${result.log})`)
    }
    const leftOut = left.map((gate) => ({ gate: gate.name, needs: BrandedBrowserGate.needs(gate, browser) }))
    const summary = { browser, host: `${process.platform} ${process.arch}`, finished: new Date().toISOString(), results, leftOut }
    writeFileSync(path.join(logs, 'summary.json'), `${JSON.stringify(summary, null, 2)}\n`)
    const counts = Object.groupBy(results, (result) => result.outcome)
    console.log(`branded ${browser}: ${counts.pass?.length ?? 0} passed, ${counts.fail?.length ?? 0} failed, ${counts.unsupported?.length ?? 0} need a call ${browser} cannot make (summary ${path.join(logs, 'summary.json')})`)
    if (results.some((result) => result.outcome !== 'pass')) process.exitCode = 1
  }
}

await BrandedBrowserGate.main()

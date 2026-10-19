import { CargoArtifacts } from './cargo-artifacts.mjs'
import { RelaunchStorageProbe } from './native/relaunch-storage-probe.mjs'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { dirname, join, relative, resolve, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

assert.equal(process.platform, 'darwin', 'Phase 4 macOS evidence requires macOS.')

class Phase4Evidence {
  static repository = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = join(
    Phase4Evidence.repository,
    'crates/tachyon-cli/tests/fixtures/phase4-macos',
  )
  static evidenceDirectory = join(Phase4Evidence.repository, 'target/phase4-evidence')
  static temporary = mkdtempSync(join(tmpdir(), 'tachyon-phase4-'))
  static project = join(Phase4Evidence.temporary, 'project')
  static helper = join(Phase4Evidence.temporary, 'phase4-macos-accessibility')
  static visualHelper = join(Phase4Evidence.temporary, 'phase4-visual-compare')
  static nativeScreenshot = join(Phase4Evidence.evidenceDirectory, 'native-macos.png')
  static webScreenshot = join(Phase4Evidence.evidenceDirectory, 'mobile-web.png')
  static accessibilityPath = join(Phase4Evidence.evidenceDirectory, 'accessibility.json')
  static visualPath = join(Phase4Evidence.evidenceDirectory, 'visual-comparison.json')
  static reportPath = join(Phase4Evidence.evidenceDirectory, 'report.json')
  static bundleIdentifier = `dev.tachyon.phase-four-evidence.g${process.pid}`
  static processName = `TachyonPhaseFour${process.pid}`
  static lifecyclePath = join(
    process.env.HOME,
    `Library/Logs/Tachyon/${Phase4Evidence.bundleIdentifier}.jsonl`,
  )
  static websiteDataFolders = [
    join(process.env.HOME, 'Library/WebKit', Phase4Evidence.bundleIdentifier),
    join(process.env.HOME, 'Library/Caches', Phase4Evidence.bundleIdentifier),
  ]
  static relaunchStoragePrefix = 'Relaunch storage: '
  static relaunchStorageStyle =
    'position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap'
}

function run(program, commandArguments, options = {}) {
  const result = spawnSync(program, commandArguments, {
    cwd: Phase4Evidence.repository,
    encoding: 'utf8',
    stdio: 'pipe',
    ...options,
  })
  assert.equal(
    result.status,
    0,
    `${program} ${commandArguments.join(' ')} failed:\n${result.stderr || result.stdout}`,
  )
  return result.stdout
}

function stopApplication() {
  spawnSync('/usr/bin/osascript', [
    '-e',
    `tell application id "${Phase4Evidence.bundleIdentifier}" to quit`,
  ])
}

const pause = (milliseconds) =>
  new Promise((resolvePause) => setTimeout(resolvePause, milliseconds))

async function retry(label, callback, attempts = 40) {
  let lastError
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      return callback()
    } catch (error) {
      lastError = error
      await pause(150)
    }
  }
  throw new Error(`${label} did not become ready: ${lastError}`)
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

function prepareProject() {
  run(process.execPath, [join(Phase4Evidence.repository, 'scripts/native/apple-protocol-test.mjs')], {
    timeout: 90000, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
  })
  stopApplication()
  cpSync(Phase4Evidence.fixture, Phase4Evidence.project, {
    recursive: true,
    filter: (source) => relative(Phase4Evidence.fixture, source).split(sep)[0] !== 'dist',
  })
  const manifestPath = join(Phase4Evidence.project, 'client', 'manifest.json')
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))
  manifest.name = Phase4Evidence.processName
  manifest.tac.application.id = Phase4Evidence.bundleIdentifier
  writeFileSync(manifestPath, JSON.stringify(manifest))
  writeRelaunchStorageFixture()
  mkdirSync(Phase4Evidence.evidenceDirectory, { recursive: true })
  rmSync(Phase4Evidence.lifecyclePath, { force: true })
}

function writeProjectFile(relativePath, source) {
  const file = join(Phase4Evidence.project, relativePath)
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, source)
}

function writeRelaunchStorageFixture() {
  writeProjectFile('client/shared/relaunch-storage.js', RelaunchStorageProbe.pageModule())
  writeProjectFile('client/services/StorageCheckService.js', `import { RelaunchStorage } from '/shared/relaunch-storage.js';
@Service()
export default class StorageCheckService {
  static _PREFIX = ${JSON.stringify(Phase4Evidence.relaunchStoragePrefix)};
  @Needs
  taskScheduler;
  report = StorageCheckService._PREFIX + '${RelaunchStorageProbe.CHECKING}';
  @OnStart()
  initialize() {
    void this._check();
  }
  async _check() {
    this.report = StorageCheckService._PREFIX + await RelaunchStorage.check();
    await this.taskScheduler.render();
  }
}
`)
  writeProjectFile('client/components/storage/check/tac.html',
    `<p style="${Phase4Evidence.relaunchStorageStyle}">{storageCheck.report}</p>\n`)
  const pagePath = join(Phase4Evidence.project, 'client/pages/tac.html')
  const page = readFileSync(pagePath, 'utf8').trimEnd()
  writeFileSync(pagePath, `${page}\n<storage-check hydrate="load" />\n`)
}

function nativeProcessIdentifier() {
  const identifiers = run('/usr/bin/pgrep', ['-x', Phase4Evidence.processName]).trim().split('\n')
  assert.equal(identifiers.length, 1, `expected one ${Phase4Evidence.processName} process`)
  return Number(identifiers[0])
}

async function readRelaunchStorage(label) {
  return retry(label, () => {
    const snapshot = JSON.parse(run(Phase4Evidence.helper, [Phase4Evidence.processName]))
    const element = snapshot.elements.find(
      (candidate) => candidate.value?.startsWith(Phase4Evidence.relaunchStoragePrefix),
    )
    assert.ok(element, 'the relaunch storage text is not in the Accessibility tree yet')
    const result = element.value.slice(Phase4Evidence.relaunchStoragePrefix.length)
    assert.notEqual(result, RelaunchStorageProbe.CHECKING, 'the page is still checking its storage')
    return result
  }, 200)
}

async function checkFirstLaunchStorage() {
  const result = await readRelaunchStorage('first launch storage write')
  assert.equal(result, RelaunchStorageProbe.WRITTEN, `the first launch must write web storage, got ${result}`)
  return { result, processIdentifier: nativeProcessIdentifier() }
}

async function waitForNativeExit(label) {
  await retry(label, () => {
    const result = spawnSync('/usr/bin/pgrep', ['-x', Phase4Evidence.processName])
    assert.notEqual(result.status, 0)
    return true
  }, 200)
}

async function checkRelaunchStorage(app, firstLaunch) {
  run('/usr/bin/open', ['-n', app])
  try {
    const result = await readRelaunchStorage('relaunched storage read')
    const processIdentifier = nativeProcessIdentifier()
    assert.notEqual(processIdentifier, firstLaunch.processIdentifier, 'the relaunch must be a new process')
    assert.equal(result, RelaunchStorageProbe.KEPT, `web storage was not kept after the app quit, got ${result}`)
    process.stdout.write(
      `macOS relaunch storage passed: launch ${firstLaunch.processIdentifier} ${firstLaunch.result}, ` +
        `relaunch ${processIdentifier} read ${result}\n`,
    )
    return { firstLaunch, relaunch: { result, processIdentifier } }
  } finally {
    stopApplication()
    await waitForNativeExit('relaunched native process termination')
  }
}

function compileHelpers() {
  run('/usr/bin/xcrun', [
    'swiftc',
    '-warnings-as-errors',
    '-O',
    '-framework',
    'AppKit',
    '-framework',
    'ApplicationServices',
    join(Phase4Evidence.repository, 'scripts/phase4-macos-accessibility.swift'),
    '-o',
    Phase4Evidence.helper,
  ])
  run('/usr/bin/xcrun', [
    'swiftc',
    '-warnings-as-errors',
    '-O',
    '-framework',
    'CoreGraphics',
    '-framework',
    'ImageIO',
    join(Phase4Evidence.repository, 'scripts/phase4-visual-compare.swift'),
    '-o',
    Phase4Evidence.visualHelper,
  ])
}

function buildNativeApp() {
  const buildOutput = run(Phase4Evidence.binary, ['build', Phase4Evidence.project, '--target', 'macos'])
  const output = join(Phase4Evidence.project, 'dist/macos')
  const app = join(output, `${Phase4Evidence.processName}.app`)
  assert.ok(existsSync(join(app, `Contents/MacOS/${Phase4Evidence.processName}`)))

  const nativeIndex = JSON.parse(readFileSync(join(output, 'native-index.json'), 'utf8'))
  assert.equal(nativeIndex.contract_version, 2)
  assert.equal(nativeIndex.entry_document, 'index.html')
  const host = JSON.parse(readFileSync(join(output, 'tachyon.host.json'), 'utf8'))
  assert.equal(host.schemaVersion, 3)
  assert.equal(host.renderMode, 'bundle')
  assert.deepEqual(host.companions, [])
  assert.deepEqual(readFileSync(join(output, 'web/index.html')),
    readFileSync(join(app, 'Contents/Resources/WebBundle/index.html')))
  return { buildOutput, output, app, nativeIndex, host }
}

async function checkNativeAccessibility() {
  await retry('native rendered Accessibility controls', () => {
    const snapshot = JSON.parse(run(Phase4Evidence.helper, [Phase4Evidence.processName]))
    assert.ok(snapshot.elements.some(element => element.role === 'AXButton' && element.label === 'Increase count'))
    assert.ok(snapshot.elements.some(element => element.role === 'AXTextField' && element.label === 'Customer name'))
    return snapshot
  })
  const accessibility = JSON.parse(
    run(Phase4Evidence.helper, [Phase4Evidence.processName, '--interact']),
  )
  writeFileSync(
    Phase4Evidence.accessibilityPath,
    `${JSON.stringify(accessibility, null, 2)}\n`,
  )
  assert.deepEqual(accessibility.interactions, [
    'increment',
    'input',
    'disclosure',
  ])
  const elements = accessibility.elements
  assert.ok(
    elements.some(
      (element) =>
        element.role === 'AXButton' &&
        element.label === 'Increase count',
    ),
  )
  assert.ok(
    elements.some(
      (element) =>
        element.value === '1',
    ),
  )
  assert.ok(
    elements.some(
      (element) =>
        element.role === 'AXTextField' &&
        element.label === 'Customer name' &&
        element.value === 'Ada',
    ),
  )
  assert.ok(
    elements.some(
      (element) =>
        element.role === 'AXDisclosureTriangle' &&
        (element.label === 'Implementation details' || element.title === 'Implementation details') &&
        ['true', '1'].includes(element.value),
    ),
  )
  assert.ok(
    elements.some(
      (element) => element.value === 'Disclosure content is native.',
    ),
  )
  assert.ok(
    elements.some(
      (element) =>
        element.role === 'AXGroup' && element.label === 'Sales chart',
    ),
  )
  return { accessibility, elements }
}

function captureNativeWindow() {
  const bounds = run('/usr/bin/osascript', [
    '-e',
    `tell application "System Events" to tell process "${Phase4Evidence.processName}"`,
    '-e',
    'set frontmost to true',
    '-e',
    'set p to position of window 1',
    '-e',
    'set s to size of window 1',
    '-e',
    'return (item 1 of p as text) & "," & (item 2 of p as text) & "," & (item 1 of s as text) & "," & (item 2 of s as text)',
    '-e',
    'end tell',
  ]).trim()
  const boundsMatch = bounds.match(
    /^(-?\d+),(-?\d+),(\d+),(\d+)$/,
  )
  assert.ok(boundsMatch, `unexpected native window bounds: ${bounds}`)
  const nativeWidth = Number(boundsMatch[3])
  const nativeHeight = Number(boundsMatch[4])
  assert.equal(nativeWidth, 420)
  assert.ok(
    nativeHeight >= 600 && nativeHeight <= 780,
    `native window height ${nativeHeight} is outside the usable parity range`,
  )
  run('/usr/sbin/screencapture', [
    '-x',
    `-R${bounds}`,
    Phase4Evidence.nativeScreenshot,
  ])
  return { nativeWidth, nativeHeight }
}

function createWebServer(output) {
  const webRoot = join(output, 'web')
  return createServer((request, response) => {
    const pathname = decodeURIComponent(
      new URL(request.url || '/', 'http://127.0.0.1').pathname,
    )
    const requested = resolve(webRoot, pathname === '/' ? 'index.html' : `.${pathname}`)
    if (requested !== webRoot && !requested.startsWith(`${webRoot}${sep}`)) {
      response.writeHead(403).end()
      return
    }
    try {
      const bytes = readFileSync(requested)
      const type = requested.endsWith('.js')
        ? 'text/javascript; charset=utf-8'
        : requested.endsWith('.css')
          ? 'text/css; charset=utf-8'
          : 'text/html; charset=utf-8'
      response.writeHead(200, { 'content-type': type }).end(bytes)
    } catch {
      response.writeHead(404).end()
    }
  })
}

function listen(server) {
  return new Promise((resolveListen, rejectListen) => {
    server.once('error', rejectListen)
    server.listen(0, '127.0.0.1', resolveListen)
  })
}

async function checkWebPage(browser, url, nativeWidth, nativeHeight) {
  const page = await browser.newPage({
    viewport: { width: nativeWidth, height: nativeHeight },
  })
  await page.goto(url, { waitUntil: 'load' })
  await page.waitForFunction(
    () => document.querySelector('button') && document.querySelector('output')?.textContent === '0',
  )
  await page.getByRole('button', { name: 'Increase count' }).click()
  await page.getByRole('textbox', { name: 'Customer name' }).fill('Ada')
  await page.locator('summary').click()
  await assert.doesNotReject(() =>
    page.getByRole('status', { name: 'Current count' }).waitFor(),
  )
  assert.equal(
    await page.getByRole('status', { name: 'Current count' }).textContent(),
    '1',
  )
  assert.equal(
    await page.getByRole('textbox', { name: 'Customer name' }).inputValue(),
    'Ada',
  )
  assert.equal(await page.locator('details').getAttribute('open'), '')
  assert.equal(
    await page.getByText('Disclosure content is native.').isVisible(),
    true,
  )

  const webSemantics = await page.evaluate(() => [
    {
      role: 'main',
      name: document.querySelector('main')?.getAttribute('aria-label') || '',
    },
    {
      role: 'heading',
      name: document.querySelector('h1')?.textContent?.trim() || '',
    },
    {
      role: 'button',
      name: document.querySelector('button')?.getAttribute('aria-label') || '',
    },
    {
      role: 'status',
      name: document.querySelector('output')?.getAttribute('aria-label') || '',
    },
    {
      role: 'textbox',
      name: document.querySelector('input')?.getAttribute('aria-label') || '',
    },
    {
      role: 'group',
      name: document.querySelector('details')?.getAttribute('aria-label') || '',
    },
    {
      role: 'group',
      name:
        document.querySelector('fancy-chart')?.getAttribute('aria-label') || '',
    },
  ])
  assert.deepEqual(webSemantics, [
    { role: 'main', name: 'Phase Four evidence' },
    { role: 'heading', name: 'Native rendering' },
    { role: 'button', name: 'Increase count' },
    { role: 'status', name: 'Current count' },
    { role: 'textbox', name: 'Customer name' },
    { role: 'group', name: 'Implementation details' },
    { role: 'group', name: 'Sales chart' },
  ])
  await page.screenshot({ path: Phase4Evidence.webScreenshot })
  return webSemantics
}

function compareVisuals() {
  const visualOutput = run(Phase4Evidence.visualHelper, [Phase4Evidence.nativeScreenshot, Phase4Evidence.webScreenshot])
  writeFileSync(Phase4Evidence.visualPath, visualOutput)
  const visual = JSON.parse(visualOutput)
  assert.equal(visual.passed, true)
  return visual
}

async function checkLifecycle() {
  await waitForNativeExit('native process termination')
  const lifecycle = readFileSync(Phase4Evidence.lifecyclePath, 'utf8')
    .trim()
    .split('\n')
    .map((line) => JSON.parse(line))
  const lifecycleEvents = lifecycle.map((entry) => entry.event)
  for (const event of [
    'controller.created',
    'controller.mounted',
    'controller.active',
    'bridge.ready',
    'controller.destroyed',
  ]) {
    assert.ok(lifecycleEvents.includes(event), `missing lifecycle event ${event}`)
  }
  assert.ok(
    lifecycleEvents.indexOf('controller.created') <
      lifecycleEvents.indexOf('controller.mounted'),
  )
  assert.ok(
    lifecycleEvents.indexOf('controller.mounted') <
      lifecycleEvents.indexOf('controller.active'),
  )
  assert.ok(
    lifecycleEvents.indexOf('controller.active') <
      lifecycleEvents.indexOf('controller.destroyed'),
  )
  return lifecycleEvents
}

function writeReport(evidence) {
  const {
    buildOutput, elements, nativeIndex, host, webSemantics, lifecycleEvents, accessibility, visual, relaunchStorage,
  } = evidence
  const report = {
    contractVersion: 1,
    target: 'macos',
    build: buildOutput.trim(),
    nativeSemantics: elements,
    nativeIndex,
    host,
    webSemantics,
    lifecycleEvents,
    interactions: accessibility.interactions,
    visual,
    relaunchStorage,
    artifacts: {
      accessibility: {
        path: 'accessibility.json',
        sha256: sha256(Phase4Evidence.accessibilityPath),
      },
      nativeScreenshot: {
        path: 'native-macos.png',
        sha256: sha256(Phase4Evidence.nativeScreenshot),
      },
      webScreenshot: {
        path: 'mobile-web.png',
        sha256: sha256(Phase4Evidence.webScreenshot),
      },
      visualComparison: {
        path: 'visual-comparison.json',
        sha256: sha256(Phase4Evidence.visualPath),
      },
    },
  }
  writeFileSync(Phase4Evidence.reportPath, `${JSON.stringify(report, null, 2)}\n`)
  process.stdout.write(
    `Phase 4 macOS acceptance passed; evidence: ${Phase4Evidence.reportPath}\n`,
  )
}

function removeTemporaryData() {
  rmSync(Phase4Evidence.temporary, { force: true, recursive: true })
  for (const folder of Phase4Evidence.websiteDataFolders) {
    rmSync(folder, { force: true, recursive: true })
  }
}

async function main() {
  let browser
  let server
  let applicationStarted = false
  try {
    prepareProject()
    compileHelpers()
    const { buildOutput, output, app, nativeIndex, host } = buildNativeApp()

    run('/usr/bin/open', ['-n', app])
    applicationStarted = true
    const { accessibility, elements } = await checkNativeAccessibility()
    const firstLaunch = await checkFirstLaunchStorage()
    const { nativeWidth, nativeHeight } = captureNativeWindow()

    server = createWebServer(output)
    await listen(server)
    const address = server.address()
    assert.equal(typeof address, 'object')
    const url = `http://127.0.0.1:${address.port}/`

    browser = await chromium.launch({ headless: true })
    const webSemantics = await checkWebPage(browser, url, nativeWidth, nativeHeight)
    const visual = compareVisuals()

    stopApplication()
    applicationStarted = false
    const lifecycleEvents = await checkLifecycle()
    const relaunchStorage = await checkRelaunchStorage(app, firstLaunch)
    writeReport({
      buildOutput, elements, nativeIndex, host, webSemantics, lifecycleEvents, accessibility, visual, relaunchStorage,
    })
  } finally {
    if (browser) await browser.close()
    if (server) {
      await new Promise((resolveClose) => server.close(resolveClose))
    }
    if (applicationStarted) stopApplication()
    if (process.env.TAC_KEEP_NATIVE_FIXTURE === '1') console.error(`Native fixture retained: ${Phase4Evidence.project}`)
    else removeTemporaryData()
  }
}

await main()

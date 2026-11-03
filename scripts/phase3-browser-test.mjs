import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { once } from 'node:events'
import { createInterface } from 'node:readline'
import { chromium } from 'playwright'

class Phase3Test {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static fixture = 'crates/tachyon-cli/tests/fixtures/phase3-browser'
}

function buildFixture() {
  const build = spawnSync(Phase3Test.binary, ['build', Phase3Test.fixture, '--no-incremental'], {
    encoding: 'utf8',
    stdio: 'pipe',
    timeout: 120000,
  })
  assert.equal(build.status, 0, build.stderr)
}

async function waitForAddress(server) {
  const lines = createInterface({ input: server.stdout })
  const readinessTimeout = new Promise((_, reject) => {
    const timer = setTimeout(
      () => reject(new Error('Development server readiness timed out.')),
      15_000,
    )
    timer.unref()
  })
  return await Promise.race([
    (async () => {
      for await (const line of lines) {
        const match = line.match(/http:\/\/127\.0\.0\.1:\d+\//)
        if (match) return match[0]
      }
      throw new Error('Development server exited before reporting readiness.')
    })(),
    readinessTimeout,
  ])
}

async function waitForActivation(page, errors) {
  await page.waitForFunction(() => {
    const loaded = document.querySelector(
      'tachyon-component[data-tachyon-component="demo-load"]',
    )
    const failure = document.querySelector(
      'tachyon-component[data-tachyon-component="demo-failure"]',
    )
    return loaded?.dataset.activated === 'Loaded'
      && failure?.dataset.tachyonMountError === 'activation_failed'
  }).catch(async error => {
    const state = await page.locator('tachyon-component').evaluateAll(roots => roots.map(root => ({ ...root.dataset, owner: root.tachyonComponent?.instance?.constructor?.name, fields: Object.keys(root.tachyonComponent?.instance || {}) })))
    throw new Error(`${error.message}\nComponent state: ${JSON.stringify(state)}\nBrowser errors: ${errors.join(' | ')}`)
  })
}

async function checkComponentStates(page) {
  const state = await page.locator('tachyon-component:not([data-tachyon-component="demo-shell"])').evaluateAll((roots) =>
    roots.map((root) => ({
      component: root.dataset.tachyonComponent,
      activated: root.dataset.activated || null,
      active: root.dataset.tachyonActive || null,
      error: root.dataset.tachyonMountError || null,
      text: root.textContent.trim(),
    })),
  )
  assert.deepEqual(state, [
    {
      component: 'demo-load',
      activated: 'Loaded',
      active: 'true',
      error: null,
      text: 'Loaded',
    },
    {
      component: 'demo-interactive',
      activated: null,
      active: null,
      error: null,
      text: 'Interactive',
    },
    {
      component: 'demo-failure',
      activated: null,
      active: 'false',
      error: 'activation_failed',
      text: 'Preserved',
    },
    {
      component: 'demo-never',
      activated: null,
      active: null,
      error: null,
      text: 'Never',
    },
  ])
}

async function checkInteractiveReplay(page) {
  await page.getByRole('button', { name: 'Interactive', exact: true }).click()
  await page.waitForFunction(() => {
    const root = document.querySelector(
      'tachyon-component[data-tachyon-component="demo-interactive"]',
    )
    return root?.dataset.activated === 'Interactive' && root?.dataset.replayed === '1'
  })
}

async function stopServer(server) {
  if (server.exitCode !== null) return
  const exited = once(server, 'exit')
  server.kill('SIGTERM')
  await exited
}

async function main() {
  buildFixture()

  const server = spawn(Phase3Test.binary, ['dev', Phase3Test.fixture, '--port', '0'], {
    env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let browser
  try {
    const address = await waitForAddress(server)

    browser = await chromium.launch({ headless: true })
    const page = await browser.newPage()
    const errors = []
    page.on('pageerror', error => errors.push(String(error)))
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()) })
    await page.goto(address, { waitUntil: 'load' })
    await waitForActivation(page, errors)
    await checkComponentStates(page)
    await checkInteractiveReplay(page)
  } finally {
    if (browser) await browser.close()
    await stopServer(server)
  }
}

await main()

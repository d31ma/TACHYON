#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { once } from 'node:events'
import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

class HotUpdateTest {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  static binary = process.env.TAC_BIN
    ?? CargoArtifacts.path('debug', process.platform === 'win32' ? 'ty.exe' : 'ty')
  static fixture = path.join(HotUpdateTest.repository, 'crates/tachyon-cli/tests/fixtures/hot-update-browser')
}

function projectPaths(project) {
  const component = path.join(project, 'client/components/hot-counter')
  return {
    templatePath: path.join(component, 'tac.html'),
    modulePath: path.join(project, 'client/services/HotCounterService.js'),
    stylePath: path.join(component, 'tac.css'),
  }
}

async function waitForAddress(server) {
  const lines = createInterface({ input: server.stdout })
  return await Promise.race([
    (async () => {
      for await (const line of lines) {
        const match = line.match(/http:\/\/127\.0\.0\.1:\d+\//)
        if (match) return match[0]
      }
      throw new Error('Development server exited before reporting readiness.')
    })(),
    new Promise((_, reject) => {
      const timer = setTimeout(
        () => reject(new Error('Development server readiness timed out.')),
        20_000,
      )
      timer.unref()
    }),
  ])
}

async function checkHotUpdates(page, address, paths, originalTemplate) {
  const { templatePath, modulePath, stylePath } = paths
  const errors = []
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text())
  })
  page.on('pageerror', (error) => errors.push(String(error)))

  await page.goto(address, { waitUntil: 'load' })
  await page.locator('tachyon-component[data-tachyon-active="true"]').waitFor()
  await page.getByRole('button', { name: 'Increase', exact: true }).click()
  await page.waitForFunction(() => document.querySelector('#count')?.textContent === '2')
  await page.getByText('More', { exact: true }).click()
  await page.locator('#draft').fill('developer draft')

  writeFileSync(stylePath, 'button { color: rgb(29, 78, 216); }\n')
  await page.waitForFunction(() =>
    document.documentElement.dataset.tachyonHot === 'css'
      && getComputedStyle(document.querySelector('#increment')).color === 'rgb(29, 78, 216)',
  )
  assert.equal(await page.textContent('#count'), '2', 'CSS HMR must retain component state')

  const nextModule = readFileSync(modulePath, 'utf8').replace(
    "version = 'one'",
    "version = 'two'",
  )
  writeFileSync(modulePath, nextModule)
  await page.waitForFunction(() => {
    const root = document.querySelector('tachyon-component[data-tachyon-component="hot-counter"]')
    return document.documentElement.dataset.tachyonHot === 'island'
      && root?.dataset.moduleVersion === 'two'
      && document.querySelector('#module-version')?.textContent === 'two'
  })
  assert.equal(await page.textContent('#count'), '2', 'service HMR must restore declared hot state')
  assert.equal(await page.locator('#details').evaluate((node) => node.open), true)
  assert.equal(await page.locator('#draft').inputValue(), 'developer draft')
  assert.equal(await page.evaluate(() => document.activeElement?.id), 'draft')
  assert.equal(await page.evaluate(() => sessionStorage.getItem('tachyon-hot-disposed')), 'true')
  assert.equal(await page.evaluate(() => sessionStorage.getItem('tachyon-hot-aborted')), 'true')

  writeFileSync(
    templatePath,
    '<section aria-label="Hot counter"><logic :else>broken</logic></section>\n',
  )
  await page.locator('#tachyon-hot-diagnostics').waitFor()
  assert.equal(await page.textContent('#count'), '2', 'failed builds must retain last-good DOM')
  assert.match(await page.textContent('#tachyon-hot-diagnostics'), /TY1302/)

  writeFileSync(templatePath, originalTemplate)
  await page.waitForFunction(() =>
    !document.querySelector('#tachyon-hot-diagnostics')
      && document.querySelector('#count')?.textContent === '1',
  )
  assert.deepEqual(errors, [], `browser errors: ${errors.join(' | ')}`)
}

async function stopServer(server) {
  if (server.exitCode !== null) return
  const exited = once(server, 'exit')
  server.kill('SIGTERM')
  await exited
}

async function main() {
  const project = mkdtempSync(path.join(tmpdir(), 'tachyon-hot-update-'))
  cpSync(HotUpdateTest.fixture, project, { recursive: true })

  const paths = projectPaths(project)
  const originalTemplate = readFileSync(paths.templatePath, 'utf8')

  const server = spawn(HotUpdateTest.binary, ['dev', project, '--port', '0'], {
    cwd: HotUpdateTest.repository,
    env: { ...process.env, TAC_ALLOW_UNTESTED: '1' },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let browser

  try {
    const address = await waitForAddress(server)

    browser = await chromium.launch({ headless: true })
    const page = await browser.newPage()
    await checkHotUpdates(page, address, paths, originalTemplate)
    console.log('PASS: semantic hot-update browser gate')
  } finally {
    if (browser) await browser.close()
    await stopServer(server)
    rmSync(project, { recursive: true, force: true })
  }
}

await main()

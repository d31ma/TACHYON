#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'
import { withServer } from './release/server-probe.mjs'

class HeldClientsGate {
  static repository = fileURLToPath(new URL('..', import.meta.url))
  static features = path.join(HeldClientsGate.repository, 'website/content/features')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static engines = { chromium, firefox, webkit }
  static browsers = (process.env.TAC_HELD_CLIENT_BROWSERS ?? 'chromium,firefox,webkit').split(',')
  static languages = (process.env.TAC_HELD_CLIENT_LANGUAGES ?? 'rs,go,dart,swift,kt,java,cs,fs,hpp').split(',')
  static manifest = { tac: { visibility: { check: false }, duplication: { check: false } } }
  static page = '<chat-room hydrate="load" />\n<ticks-stream hydrate="load" />\n'
  static buildTimeout = 900_000
  static waitTimeout = 20_000
  static ticks = ['ANVIL-1 · 3', 'ANVIL-1 · 2', 'ANVIL-1 · 1']

  static write(project, name, text) {
    const target = path.join(project, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static chosen(name, language) {
    const server = name.startsWith('server/') && (name.endsWith('.js') || name.includes('/models/') && name.endsWith('.ts'))
    const client = name.startsWith('client/') && (name.endsWith(`.${language}`) || name.startsWith('client/components/'))
    return server || client
  }

  static project(language) {
    const project = mkdtempSync(path.join(tmpdir(), `tachyon-held-clients-${language}-`))
    for (const feature of ['sockets', 'realtime']) {
      const source = path.join(HeldClientsGate.features, feature)
      const { files } = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
      for (const { name } of files.filter((file) => HeldClientsGate.chosen(file.name, language))) {
        HeldClientsGate.write(project, name, readFileSync(path.join(source, name), 'utf8'))
      }
    }
    HeldClientsGate.write(project, 'client/pages/tac.html', HeldClientsGate.page)
    HeldClientsGate.write(project, 'client/manifest.json', `${JSON.stringify(HeldClientsGate.manifest)}\n`)
    return project
  }

  static build(project, language) {
    const built = spawnSync(HeldClientsGate.binary, ['build', project], {
      encoding: 'utf8', timeout: HeldClientsGate.buildTimeout, killSignal: 'SIGKILL', maxBuffer: 16 * 1024 * 1024,
    })
    if (built.status !== 0) throw new Error(`${language}: the held-client fixture did not build: ${built.error?.message || `${built.stdout}${built.stderr}`}`)
  }

  static async shows(page, text) {
    await page.waitForFunction((wanted) => document.body.textContent.includes(wanted), text, { timeout: HeldClientsGate.waitTimeout })
  }

  static async check(browserType, origin, language) {
    const label = `${browserType.name()} ${language}`
    const browser = await browserType.launch({ timeout: 30_000 })
    try {
      const context = await browser.newContext()
      const errors = []
      const [first, second] = [await context.newPage(), await context.newPage()]
      for (const page of [first, second]) {
        page.on('pageerror', (error) => errors.push(error.message))
        await page.goto(origin)
        await HeldClientsGate.shows(page, 'Welcome to the room')
      }
      console.log(`ok - ${label}: the service's socket client opens with its component and hears the welcome`)
      await first.getByRole('button', { name: 'Say hello' }).click()
      for (const page of [first, second]) await HeldClientsGate.shows(page, 'Hello from a tab')
      console.log(`ok - ${label}: the service's send writes to the socket and both tabs hear the message`)
      for (const tick of HeldClientsGate.ticks) await HeldClientsGate.shows(first, tick)
      console.log(`ok - ${label}: the service's stream client hears every tick of the stream`)
      assert.deepEqual(errors, [], `${label}: the page reported no errors`)
    } finally {
      await browser.close()
    }
  }

  static async run(language) {
    const project = HeldClientsGate.project(language)
    try {
      HeldClientsGate.build(project, language)
      const serve = ['serve', project, '--host', '127.0.0.1', '--port', '0', '--out-dir', 'dist', '--no-watch', '--no-self-restart', '--no-bundle']
      await withServer(HeldClientsGate.binary, serve, async (request) => {
        const response = await request('/')
        assert.equal(response.status, 200)
        const origin = new URL(response.url).origin
        for (const name of HeldClientsGate.browsers) await HeldClientsGate.check(HeldClientsGate.engines[name], origin, language)
      }, { readinessTimeoutMilliseconds: 120_000 })
    } finally {
      rmSync(project, { recursive: true, force: true })
    }
  }

  static async main() {
    for (const language of HeldClientsGate.languages) await HeldClientsGate.run(language)
    console.log(`held stream and socket clients passed for ${HeldClientsGate.languages.join(', ')} in ${HeldClientsGate.browsers.join(', ')}`)
  }
}

await HeldClientsGate.main()

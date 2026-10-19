#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { spawnSync } from 'node:child_process'
import { cpSync, createReadStream, existsSync, mkdtempSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

class AsyncListenerGate {
  static example = fileURLToPath(new URL('../website/content/docs/cookbook/signal-driven-ui/', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static engines = ['chromium', 'firefox', 'webkit']
  static browsers = (process.env.TAC_ASYNC_LISTENER_BROWSERS ?? AsyncListenerGate.engines.join(',')).split(',')
  static extensions = { swift: '.swift', rust: '.rs' }
  static languages = (process.env.TAC_ASYNC_LISTENER_LANGUAGES ?? Object.keys(AsyncListenerGate.extensions).join(',')).split(',')
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static manifest = { tac: { duplication: { check: false } } }
  static buildTimeout = 900_000
  static names = ['Ada', 'Lin']

  static project(language) {
    const extension = AsyncListenerGate.extensions[language]
    if (!extension) throw new Error(`no async listener example for ${language}; the languages are ${Object.keys(AsyncListenerGate.extensions).join(', ')}`)
    const project = mkdtempSync(path.join(tmpdir(), `tachyon-async-listener-${language}-`))
    const client = path.join(AsyncListenerGate.example, 'client')
    cpSync(path.join(client, 'pages'), path.join(project, 'client/pages'), { recursive: true })
    cpSync(path.join(client, 'components'), path.join(project, 'client/components'), { recursive: true })
    for (const folder of ['models', 'remotes', 'services', 'setups']) {
      for (const file of readdirSync(path.join(client, folder)).filter((name) => name.endsWith(extension))) {
        cpSync(path.join(client, folder, file), path.join(project, 'client', folder, file))
      }
    }
    writeFileSync(path.join(project, 'client/manifest.json'), `${JSON.stringify(AsyncListenerGate.manifest)}\n`)
    return project
  }

  static build(project) {
    const built = spawnSync(AsyncListenerGate.binary, ['build', project], {
      encoding: 'utf8', timeout: AsyncListenerGate.buildTimeout, killSignal: 'SIGKILL', maxBuffer: 16 * 1024 * 1024,
    })
    return { ...built, output: `${built.stdout ?? ''}${built.stderr ?? ''}` }
  }

  static handler(project) {
    const root = path.join(project, 'dist')
    return (request, response) => {
      const url = new URL(request.url, 'http://localhost')
      let file = path.join(root, decodeURIComponent(url.pathname))
      if (existsSync(file) && statSync(file).isDirectory()) file = path.join(file, 'index.html')
      if (!existsSync(file)) {
        response.writeHead(404).end()
        return
      }
      response.writeHead(200, { 'content-type': AsyncListenerGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' })
      createReadStream(file).pipe(response)
    }
  }

  static expect(actual, wanted, label) {
    if (JSON.stringify(actual) !== JSON.stringify(wanted)) throw new Error(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`)
    console.log(`ok - ${label}`)
  }

  static async save(page, name, count) {
    await page.getByLabel('Name').fill(name)
    await page.getByRole('button', { name: 'Save' }).click()
    await page.locator('li').nth(count - 1).waitFor({ timeout: 15_000 })
  }

  static async check(page, label) {
    for (const [index, name] of AsyncListenerGate.names.entries()) await AsyncListenerGate.save(page, name, index + 1)
    const listed = await page.locator('li').allTextContents()
    AsyncListenerGate.expect(listed.map((text) => text.trim()), AsyncListenerGate.names, `${label}: the async listener reloads every saved name through the client`)
  }

  static async run(project, language, launchers) {
    const server = createServer(AsyncListenerGate.handler(project))
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const origin = `http://127.0.0.1:${server.address().port}`
    try {
      for (const name of AsyncListenerGate.browsers) {
        const browser = await launchers[name].launch({ timeout: 30_000 })
        try {
          const page = await browser.newPage()
          const errors = []
          page.on('pageerror', (error) => errors.push(String(error)))
          await page.goto(`${origin}/`, { waitUntil: 'networkidle' })
          await AsyncListenerGate.check(page, `${name} ${language}`)
          AsyncListenerGate.expect(errors.join('\n'), '', `${name} ${language}: the page threw no error`)
        } finally {
          await browser.close()
        }
      }
    } finally {
      server.close()
    }
  }

  static async language(language, launchers) {
    const project = AsyncListenerGate.project(language)
    try {
      const built = AsyncListenerGate.build(project)
      if (built.status !== 0) throw new Error(`${language} async listener example build failed: ${built.error?.message || built.output}`)
      await AsyncListenerGate.run(project, language, launchers)
    } finally {
      rmSync(project, { recursive: true, force: true })
    }
  }

  static async main() {
    const unknown = AsyncListenerGate.browsers.filter((name) => !AsyncListenerGate.engines.includes(name))
    if (unknown.length) throw new Error(`unknown browser ${unknown.join(', ')}; nothing was run`)
    const launchers = await import('playwright')
    for (const language of AsyncListenerGate.languages) await AsyncListenerGate.language(language, launchers)
  }
}

await AsyncListenerGate.main()

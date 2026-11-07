#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import { spawnSync } from 'node:child_process'
import { cpSync, createReadStream, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

class PageProgramGate {
  static fixtures = fileURLToPath(new URL('./page-program/', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('release/ty')
  static engines = ['chromium', 'firefox', 'webkit']
  static browsers = (process.env.TAC_PAGE_PROGRAM_BROWSERS ?? PageProgramGate.engines.join(',')).split(',')
  static programs = ['shared', 'separate']
  static mimeTypes = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' }
  static manifest = { tac: { duplication: { check: false } } }
  static measuring = { tac: { duplication: { check: false }, budgets: { script: 0, webassembly: 0 } } }
  static routeSize = /TY1337\]?:? Route '\/' loads (\d+) KB of (script|WebAssembly)/g
  static settleTimeout = 15_000
  static settleStep = 50
  static buildTimeout = 900_000
  static remembered = 'hello'

  static languages() {
    const chosen = process.env.TAC_PAGE_PROGRAM_LANGUAGES?.split(',')
    const present = readdirSync(PageProgramGate.fixtures).filter((name) => existsSync(path.join(PageProgramGate.fixtures, name, 'expected.json')))
    const unknown = (chosen ?? []).filter((name) => !present.includes(name))
    if (unknown.length) throw new Error(`no page program fixture for ${unknown.join(', ')}; the fixtures are ${present.join(', ')}`)
    return chosen ?? present.filter((name) => !PageProgramGate.owed(name))
  }

  static owed(language) {
    const { owed } = JSON.parse(readFileSync(path.join(PageProgramGate.fixtures, language, 'expected.json'), 'utf8'))
    if (owed) console.log(`owed - ${language}: ${owed}`)
    return Boolean(owed)
  }

  static expected(language) {
    const { program } = JSON.parse(readFileSync(path.join(PageProgramGate.fixtures, language, 'expected.json'), 'utf8'))
    if (!PageProgramGate.programs.includes(program)) throw new Error(`${language}/expected.json must say "shared" or "separate", not ${JSON.stringify(program)}`)
    return program
  }

  static project(language, manifest) {
    const project = mkdtempSync(path.join(tmpdir(), `tachyon-page-program-${language}-`))
    cpSync(path.join(PageProgramGate.fixtures, 'shared'), project, { recursive: true })
    cpSync(path.join(PageProgramGate.fixtures, language), project, { recursive: true })
    rmSync(path.join(project, 'expected.json'))
    writeFileSync(path.join(project, 'client/manifest.json'), `${JSON.stringify(manifest)}\n`)
    return project
  }

  static build(project) {
    const built = spawnSync(PageProgramGate.binary, ['build', project], {
      encoding: 'utf8', timeout: PageProgramGate.buildTimeout, killSignal: 'SIGKILL', maxBuffer: 16 * 1024 * 1024,
    })
    return { ...built, output: `${built.stdout ?? ''}${built.stderr ?? ''}` }
  }

  static sizes(language) {
    const project = PageProgramGate.project(language, PageProgramGate.measuring)
    try {
      const built = PageProgramGate.build(project)
      const sizes = { script: 0, WebAssembly: 0 }
      for (const [, kilobytes, kind] of built.output.matchAll(PageProgramGate.routeSize)) sizes[kind] = Number(kilobytes)
      if (!sizes.script) throw new Error(`${language}: the zero-budget build did not report the route size: ${built.error?.message || built.output}`)
      return sizes
    } finally {
      rmSync(project, { recursive: true, force: true })
    }
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
      response.writeHead(200, { 'content-type': PageProgramGate.mimeTypes[path.extname(file)] ?? 'application/octet-stream' })
      createReadStream(file).pipe(response)
    }
  }

  static expect(actual, wanted, label) {
    if (actual !== wanted) throw new Error(`${label}: expected ${JSON.stringify(wanted)}, got ${JSON.stringify(actual)}`)
    console.log(`ok - ${label}`)
  }

  static state(page) {
    return page.evaluate(() => {
      const read = (name) => {
        const service = __tachyonTac.service(name)
        return { identity: service.identity, count: service.count, recalled: service.recalled }
      }
      return { writer: read('writer'), reader: read('reader') }
    })
  }

  static async call(page, role, method, ...values) {
    await page.evaluate(([name, member, given]) => __tachyonTac.service(name)[member](...given), [role, method, values])
  }

  static async settled(page, done) {
    const deadline = Date.now() + PageProgramGate.settleTimeout
    let state = await PageProgramGate.state(page)
    while (!done(state) && Date.now() < deadline) {
      await page.waitForTimeout(PageProgramGate.settleStep)
      state = await PageProgramGate.state(page)
    }
    return state
  }

  static async check(page, name, language, program) {
    const label = `${name} ${language} (${program})`
    await PageProgramGate.call(page, 'writer', 'inspect')
    await PageProgramGate.call(page, 'reader', 'inspect')
    const opened = await PageProgramGate.settled(page, (state) => Boolean(state.writer.identity && state.reader.identity))
    PageProgramGate.expect(Boolean(opened.writer.identity && opened.reader.identity), true, `${label}: both services read the Setup's adapter through their client`)
    PageProgramGate.expect(opened.writer.identity === opened.reader.identity, program === 'shared', `${label}: the two services ${program === 'shared' ? 'share one' : 'still get separate'} Setup configuration`)
    await PageProgramGate.call(page, 'writer', 'increment')
    await PageProgramGate.call(page, 'reader', 'inspect')
    const changed = await PageProgramGate.settled(page, (state) => state.writer.count === 1)
    PageProgramGate.expect(changed.writer.count, 1, `${label}: the writer's increment reaches its adapter`)
    PageProgramGate.expect(changed.reader.count, program === 'shared' ? 1 : 0, `${label}: the reader ${program === 'shared' ? 'sees' : 'does not see'} the writer's increment through its own client`)
    await PageProgramGate.call(page, 'writer', 'remember', PageProgramGate.remembered)
    await PageProgramGate.call(page, 'reader', 'recall')
    const recalled = await PageProgramGate.settled(page, (state) => state.reader.recalled === PageProgramGate.remembered)
    PageProgramGate.expect(recalled.reader.recalled, PageProgramGate.remembered, `${label}: the reader's store finds the record the writer's store saved`)
  }

  static async run(project, language, program, launchers) {
    const server = createServer(PageProgramGate.handler(project))
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const origin = `http://127.0.0.1:${server.address().port}`
    try {
      for (const name of PageProgramGate.browsers) {
        const browser = await launchers[name].launch({ timeout: 30_000 })
        try {
          const page = await browser.newPage()
          const errors = []
          page.on('pageerror', (error) => errors.push(String(error)))
          await page.goto(`${origin}/`, { waitUntil: 'networkidle' })
          await PageProgramGate.check(page, name, language, program)
          PageProgramGate.expect(errors.join('\n'), '', `${name} ${language}: the page threw no error`)
        } finally {
          await browser.close()
        }
      }
    } finally {
      server.close()
    }
  }

  static async language(language, launchers) {
    const program = PageProgramGate.expected(language)
    const sizes = PageProgramGate.sizes(language)
    console.log(`size - ${language}: route '/' loads ${sizes.script} KB of script and ${sizes.WebAssembly} KB of WebAssembly (TY1337)`)
    const project = PageProgramGate.project(language, PageProgramGate.manifest)
    try {
      const built = PageProgramGate.build(project)
      if (built.status !== 0) throw new Error(`${language} page program fixture build failed: ${built.error?.message || built.output}`)
      await PageProgramGate.run(project, language, program, launchers)
      return { language, program, ...sizes }
    } finally {
      rmSync(project, { recursive: true, force: true })
    }
  }

  static async main() {
    const unknown = PageProgramGate.browsers.filter((name) => !PageProgramGate.engines.includes(name))
    if (unknown.length) throw new Error(`unknown browser ${unknown.join(', ')}; nothing was run`)
    const launchers = await import('playwright')
    const results = []
    for (const language of PageProgramGate.languages()) results.push(await PageProgramGate.language(language, launchers))
    const receipt = process.env.TAC_PAGE_PROGRAM_RECEIPT
    if (receipt) {
      mkdirSync(path.dirname(path.resolve(receipt)), { recursive: true })
      writeFileSync(receipt, `${JSON.stringify({ browsers: PageProgramGate.browsers, results }, null, 2)}\n`)
    }
    console.table(results)
  }
}

await PageProgramGate.main()

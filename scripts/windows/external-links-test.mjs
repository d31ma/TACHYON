#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { rmSync } from 'node:fs'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { PageChecksFixture } from '../native/page-checks-fixture.mjs'
import { WindowsCommand } from './windows-command.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsExternalLinksTest {
  static fixture = WindowsGate.fixture('windows-external-links-')
  static handshake = path.join(WindowsExternalLinksTest.fixture, 'handshake')
  static application = 'LinkGate'
  static applicationId = 'dev.tachyon.link-gate'
  static openedPath = '/opened?b=1'
  static settleMilliseconds = 5_000
  static browserMilliseconds = 60_000
  static processesBefore = new Set()
  static clickedAt = 0

  /** @param {string} localLink */
  static files(localLink) {
    return {
      'client/manifest.json': JSON.stringify({
        name: 'Link Gate',
        start_url: '/',
        tac: { application: { id: this.applicationId, version: '0.0.1', build: 3 } },
      }),
      'client/pages/tac.html': '<home-panel />\n',
      'client/components/home/panel/tac.html': `<main aria-label="Home route"><h1>Home route</h1>
<a href="${localLink}">Open local page</a>
<a href="https://example.com/a?b=1">Open example</a>
<a href="mailto:someone@example.com">Write mail</a>
</main>
`,
    }
  }

  static async localPage() {
    const port = await PageChecksFixture.freePort()
    const page = { origin: `http://127.0.0.1:${port}`, requests: [] }
    page.server = createServer((request, response) => {
      page.requests.push(request.url)
      response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' })
      response.end('<!doctype html><title>Tachyon link check</title><p>The Windows default browser opened this link. You can close this tab.</p>')
    })
    await new Promise((resolve) => page.server.listen(port, '127.0.0.1', resolve))
    return page
  }

  /** @param {string} log */
  static failures(log) {
    return WindowsGate.readLines(log).filter(line => line.includes('"event":"link.external.failed"')).length
  }

  /** @param {Set<string>} before @param {number} since */
  static async startedSince(before, since) {
    const table = await WindowsCommand.processes()
    return [...new Set(table.filter(entry => !before.has(`${entry.pid}:${entry.started}`) && Number(entry.started) >= since).map(entry => entry.name))]
  }

  static async snapshot() {
    return new Set((await WindowsCommand.processes()).map(entry => `${entry.pid}:${entry.started}`))
  }

  static fileTimeNow() {
    return Date.now() * 10_000 + 116_444_736_000_000_000
  }

  /** @param {string} log @param {string} link @param {() => Promise<void>} observe @param {string[]} report */
  static async checkLink(log, link, observe, report) {
    const before = this.failures(log)
    await observe()
    const failed = this.failures(log) - before
    const started = await this.startedSince(this.processesBefore, this.clickedAt)
    const opened = started.length ? started.join(', ') : 'no new process (the handler may already have been running)'
    const outcome = failed === 0 ? 'OK' : 'FAILED'
    report.push(`${outcome}: ${link} -> ShellExecuteW ${failed === 0 ? 'succeeded (no link.external.failed record)' : 'returned 32 or less (link.external.failed)'}; started: ${opened}`)
    console.log(report.at(-1))
  }

  /** @param {string} log @param {{ origin: string, requests: string[] }} page @param {string[]} report */
  static answers(log, page, report) {
    const prepare = async () => {
      this.processesBefore = await this.snapshot()
      this.clickedAt = this.fileTimeNow()
    }
    return {
      'before-local': prepare,
      'local-clicked': () => this.checkLink(log, `${page.origin}${this.openedPath}`, async () => {
        const deadline = Date.now() + this.browserMilliseconds
        while (!page.requests.includes(this.openedPath) && Date.now() < deadline) await sleep(250)
        assert.ok(page.requests.includes(this.openedPath), `The default browser never fetched ${page.origin}${this.openedPath} (it saw ${JSON.stringify(page.requests)})`)
        console.log(`OK: the default browser fetched ${page.origin}${this.openedPath}`)
      }, report),
      'before-example': prepare,
      'example-clicked': () => this.checkLink(log, 'https://example.com/a?b=1', () => sleep(this.settleMilliseconds), report),
      'before-mail': prepare,
      'mail-clicked': () => this.checkLink(log, 'mailto:someone@example.com', () => sleep(this.settleMilliseconds), report),
    }
  }

  /** @param {string} log */
  static checkLifecycleLog(log) {
    const records = WindowsGate.readLines(log)
    assert.ok(records.length > 0, `the host wrote no lifecycle log at ${log}`)
    const elsewhere = records.filter(line => !line.includes('"route":"/"'))
    assert.deepEqual(elsewhere, [], 'the app left its first page')
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    const page = await this.localPage()
    const report = []
    try {
      for (const [name, text] of Object.entries(this.files(`${page.origin}${this.openedPath}`))) WindowsGate.write(this.fixture, name, text)
      await WindowsGate.build(ty, this.fixture)
      const executable = WindowsGate.executable(this.fixture, this.application)
      const log = WindowsGate.lifecycleLog(this.applicationId)
      rmSync(log, { force: true })
      const app = WindowsGate.launch(executable)
      try {
        const probe = WindowsGate.startProbe(app.pid, [
          ['wait', ['Home route', 'Open local page', 'Open example', 'Write mail']],
          ['roles', [['Heading', 'Home route'], ['Hyperlink', 'Open local page'], ['Hyperlink', 'Open example'], ['Hyperlink', 'Write mail']]],
          ['ask', 'before-local'], ['click', 'Open local page'], ['ask', 'local-clicked'],
          ['ask', 'before-example'], ['click', 'Open example'], ['ask', 'example-clicked'],
          ['ask', 'before-mail'], ['click', 'Write mail'], ['ask', 'mail-clicked'],
          ['wait', ['Home route', 'Open local page', 'Open example', 'Write mail']],
        ], this.handshake)
        await WindowsGate.answerAll(probe, this.handshake, this.answers(log, page, report))
        assert.ok(!app.exited, 'the app exited after the links were clicked')
      } finally {
        await app.stop()
      }
      this.checkLifecycleLog(log)
    } finally {
      await new Promise((resolve) => page.server.close(resolve))
    }
    const failed = report.filter(line => line.startsWith('FAILED'))
    assert.deepEqual(failed, [], 'the host recorded a failed launch; is a default app set for that scheme?')
    console.log(`PASS: a WebView2 page in the built Windows app cancelled its http, https and mailto navigations, handed each link to ShellExecuteW (the default browser fetched ${page.origin}${this.openedPath}) and stayed on Home route`)
  }
}

await WindowsExternalLinksTest.main()

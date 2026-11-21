#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { PeersNativeProbe } from './peers-native-probe.mjs'
import { SframeCallProbe } from './sframe-call-probe.mjs'

class MacOsSframeCallTest {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static temporary = mkdtempSync(path.join(tmpdir(), 'tachyon-sframe-call-macos-'))
  static project = path.join(MacOsSframeCallTest.temporary, 'project')
  static helper = path.join(MacOsSframeCallTest.temporary, 'accessibility')
  static processName = `TachyonCall${process.pid}`
  static bundleIdentifier = `dev.tachyon.sframe-call.g${process.pid}`
  static template = (name) => path.join(MacOsSframeCallTest.repository, 'crates/tachyon-core/src/template', name)
  static scripts = { '/tac-media.js': 'tac-media.js', '/.tachyon/tac-media-frames.js': 'tac-media-frames.js' }
  static waitRounds = 240
  static waitStep = 500
  static framesWaitMs = 30000
  static settleMs = 5000

  /** @param {string} program @param {string[]} args */
  static run(program, args) {
    const result = spawnSync(program, args, { cwd: MacOsSframeCallTest.repository, encoding: 'utf8', timeout: 900000, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`)
    return result.stdout
  }

  static pause(milliseconds) {
    return new Promise(resolve => setTimeout(resolve, milliseconds))
  }

  /** @param {number} turnPort */
  static async mailbox(turnPort) {
    const boxes = { '/to-app': [], '/to-browser': [] }
    const scripts = {}
    for (const [route, name] of Object.entries(MacOsSframeCallTest.scripts)) scripts[route] = readFileSync(MacOsSframeCallTest.template(name), 'utf8')
    const page = SframeCallProbe.browserPage(turnPort)
    const server = createServer((request, response) => MacOsSframeCallTest.answer(request, response, { boxes, scripts, page }))
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    server.reset = () => { boxes['/to-app'].length = 0; boxes['/to-browser'].length = 0 }
    return server
  }

  static answer(request, response, { boxes, scripts, page }) {
    const route = new URL(request.url, 'http://127.0.0.1').pathname
    const headers = { 'access-control-allow-origin': '*', 'cache-control': 'no-store' }
    if (scripts[route]) return response.writeHead(200, { ...headers, 'content-type': 'text/javascript' }).end(scripts[route])
    if (!boxes[route]) return response.writeHead(200, { ...headers, 'content-type': 'text/html' }).end(page)
    if (request.method === 'GET') return response.writeHead(200, { ...headers, 'content-type': 'application/json' }).end(JSON.stringify(boxes[route].splice(0)))
    let body = ''
    request.on('data', (chunk) => { body += chunk })
    request.on('end', () => {
      boxes[route].push(JSON.parse(body))
      response.writeHead(204, headers).end()
    })
    return null
  }

  /** @param {number} turnPort @param {string} mailbox */
  static stage(turnPort, mailbox) {
    const settings = { id: MacOsSframeCallTest.bundleIdentifier, name: MacOsSframeCallTest.processName, turnPort, mailbox }
    for (const [name, text] of Object.entries(SframeCallProbe.files(settings))) {
      const file = path.join(MacOsSframeCallTest.project, name)
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, text)
    }
  }

  static compileHelper() {
    MacOsSframeCallTest.run('/usr/bin/xcrun', ['swiftc', '-O', '-framework', 'AppKit', '-framework', 'ApplicationServices', path.join(MacOsSframeCallTest.repository, 'scripts/phase4-macos-accessibility.swift'), '-o', MacOsSframeCallTest.helper])
  }

  static build() {
    MacOsSframeCallTest.run(MacOsSframeCallTest.binary, ['build', MacOsSframeCallTest.project, '--target', 'macos'])
    const app = path.join(MacOsSframeCallTest.project, 'dist/macos', `${MacOsSframeCallTest.processName}.app`)
    assert.ok(existsSync(app), `the build made no app at ${app}`)
    return path.join(app, 'Contents/MacOS', MacOsSframeCallTest.processName)
  }

  static read() {
    const result = spawnSync(MacOsSframeCallTest.helper, [MacOsSframeCallTest.processName], { encoding: 'utf8', timeout: 30000 })
    if (result.status !== 0) return ''
    const element = JSON.parse(result.stdout).elements.find(candidate => candidate.value?.startsWith(SframeCallProbe.PREFIX))
    return element ? element.value.slice(SframeCallProbe.PREFIX.length) : ''
  }

  static async appResult() {
    let text = ''
    for (let round = 0; round < MacOsSframeCallTest.waitRounds; round++) {
      text = MacOsSframeCallTest.read()
      if (text && text !== SframeCallProbe.CHECKING) return text
      await MacOsSframeCallTest.pause(MacOsSframeCallTest.waitStep)
    }
    return text || 'no result in the Accessibility tree'
  }

  /** @param {string} text */
  static parsed(text) {
    const match = /^done decoded=(\d+) events=(.*)$/.exec(text)
    assert.ok(match, `the macOS app finished the call, got: ${text}`)
    return { decoded: Number(match[1]), events: match[2] ? match[2].split(',') : [] }
  }

  static async browserFrames(page, expectFrames) {
    const deadline = Date.now() + MacOsSframeCallTest.framesWaitMs
    while (expectFrames && Date.now() < deadline && (await page.evaluate(() => window.decoded())) === 0) await page.waitForTimeout(MacOsSframeCallTest.waitStep)
    if (!expectFrames) await page.waitForTimeout(MacOsSframeCallTest.settleMs)
    return page.evaluate(() => window.result())
  }

  static async call({ browser, mailbox, executable, key, expectFrames }) {
    mailbox.reset()
    const page = await browser.newPage()
    let app
    try {
      await page.goto(`http://127.0.0.1:${mailbox.address().port}/?key=${key}`)
      app = spawn(executable, [], { stdio: 'ignore' })
      const native = MacOsSframeCallTest.parsed(await MacOsSframeCallTest.appResult())
      const web = await MacOsSframeCallTest.browserFrames(page, expectFrames)
      return { native, web }
    } finally {
      app?.kill('SIGTERM')
      await MacOsSframeCallTest.pause(MacOsSframeCallTest.waitStep)
      await page.close()
    }
  }

  static cleanup(turn, mailbox, browser) {
    turn?.kill('SIGTERM')
    mailbox?.close()
    void browser?.close()
    for (const folder of [path.join(homedir(), 'Library/WebKit', MacOsSframeCallTest.bundleIdentifier), path.join(homedir(), 'Library/Caches', MacOsSframeCallTest.bundleIdentifier)]) rmSync(folder, { recursive: true, force: true })
    rmSync(MacOsSframeCallTest.temporary, { recursive: true, force: true })
  }

  static async main() {
    assert.equal(process.platform, 'darwin', 'the macOS SFrame call check needs macOS')
    const turnPort = await PeersNativeProbe.freePort()
    const turn = PeersNativeProbe.startTurn(turnPort)
    const mailbox = await MacOsSframeCallTest.mailbox(turnPort)
    let browser
    try {
      MacOsSframeCallTest.stage(turnPort, `http://127.0.0.1:${mailbox.address().port}`)
      MacOsSframeCallTest.compileHelper()
      const executable = MacOsSframeCallTest.build()
      browser = await chromium.launch()
      const same = await MacOsSframeCallTest.call({ browser, mailbox, executable, key: 'same', expectFrames: true })
      console.log('same key', JSON.stringify(same))
      const wrong = await MacOsSframeCallTest.call({ browser, mailbox, executable, key: 'wrong', expectFrames: false })
      console.log('wrong key', JSON.stringify(wrong))
      assert.ok(same.native.decoded > 0, 'the macOS app decodes the browser frames sealed with the same key')
      assert.ok(same.native.events.includes('MediaFrameKeyReady'), 'the macOS app reports its native frame key ready')
      assert.ok(same.web.decoded > 0, 'the browser decodes the macOS app frames sealed with the native frame key')
      assert.ok(same.web.events.includes('MediaFrameKeyReady'), 'the browser reports its page frame key ready')
      assert.equal(wrong.native.decoded, 0, 'the macOS app decodes nothing sealed with another key')
      assert.ok(wrong.native.events.includes('MediaFrameRejected'), 'the macOS app reports the rejected frames')
      assert.equal(wrong.web.decoded, 0, 'the browser decodes nothing sealed with another key')
      assert.ok(wrong.web.events.includes('MediaFrameRejected'), 'the browser reports the rejected frames')
      console.log('PASS: a built macOS app (native frame key from the Keychain key, HPKE to the media worker) and Chromium held one SFrame call through a local TURN relay; each side decoded the other\'s frames with the same key, and both decoded nothing and reported rejected frames with a wrong key')
    } finally {
      MacOsSframeCallTest.cleanup(turn, mailbox, browser)
    }
  }
}

await MacOsSframeCallTest.main()

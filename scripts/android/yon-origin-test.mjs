#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'
import { withServer } from '../release/server-probe.mjs'
import { YonReachFixture } from '../yon-reach-fixture.mjs'

class AndroidYonOriginTest {
  static repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..')
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
  static adb = path.join(AndroidYonOriginTest.sdk ?? '', 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb')
  static packageName = 'dev.tachyon.yon_reach'
  static applicationName = 'Yon Reach'
  static appOrigin = 'https://appassets.tachyon.local'
  static timeout = 30000
  static nativeFiles = {
    'client/models/Greeting.kt': 'data class Greeting(val message: String)\n',
    'client/remotes/GreetingClient.kt': `import models.Greeting

@HTTPClient("/api")
interface GreetingClient {
    @GetRequest("/greeting")
    suspend fun read(): Greeting
}
`,
    'client/setups/GreetingSetup.kt': `@Setup
class GreetingSetup {
    companion object {
        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()
    }
}
`,
    'client/services/NativeReachService.kt': `@Service
class NativeReachService {
    @Needs
    private val greetingClient: GreetingClient
    var message: String = ""

    suspend fun ask() {
        message = greetingClient.read().message
    }

    fun processId(): Int = android.os.Process.myPid()
}
`,
    'client/components/kotlin/reach/tac.html': '<button on:click="nativeReach.ask()">Ask Yon natively</button>\n<p id="native-reach">{nativeReach.message}</p>\n',
    'server/models/Greeting.ts': 'export interface Greeting {\n  message: string;\n}\n',
    'server/controllers/GreetingController.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/greeting")
@Public
export default class GreetingController {
  /** @returns {Greeting} */
  @GetRoute
  getGreeting() {
    return { message: 'Hello from Yon to Kotlin' };
  }
}
`,
  }

  static run(command, args, options = {}) {
    const result = spawnSync(command, args, { cwd: this.repository, encoding: 'utf8', timeout: 240000, maxBuffer: 2 * 1024 * 1024, ...options })
    assert.equal(result.status, 0, `${command} ${args.join(' ')}: ${result.error || result.stderr || result.stdout}`)
    return result.stdout.trim()
  }

  static pause(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds))
  }

  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address()
        server.close(() => resolve(port))
      })
    })
  }

  static binary() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN
    this.run('cargo', ['build', '--locked', '--bin', 'ty'])
    return path.join(JSON.parse(this.run('cargo', ['metadata', '--format-version', '1', '--no-deps'])).target_directory, 'debug/ty')
  }

  static executableName() {
    return this.applicationName.replace(/[^A-Za-z0-9]/g, '').slice(0, 64)
  }

  static files() {
    const setup = path.join(this.repository, 'website/content/docs/calling-the-server/browser-http-kotlin/client/setups/GreetingSetup.kt')
    return { ...this.nativeFiles, 'client/setups/GreetingSetup.kt': readFileSync(setup, 'utf8') }
  }

  static writeFixture(project, port) {
    return new YonReachFixture({
      directory: project,
      yonOrigin: `http://localhost:${port}`,
      appOrigins: [this.appOrigin],
      manifest: { name: this.applicationName, start_url: '/', tac: { application: { id: 'dev.tachyon.yon-reach', version: '0.1.0', build: 1 } } },
      files: this.files(),
      elements: ['kotlin-reach'],
    }).write()
  }

  static build(binary, project) {
    this.run(binary, ['build', project, '--target', 'android'])
    const out = path.join(project, 'dist/android')
    const host = JSON.parse(readFileSync(path.join(out, 'tachyon.host.json'), 'utf8'))
    assert.deepEqual(host.companions, [{ service: 'nativeReach', language: 'Kotlin' }])
    const executable = this.executableName()
    return path.join(out, executable, `${executable}.apk`)
  }

  static async installAndLaunch(apk) {
    this.run(this.adb, ['wait-for-device'])
    this.run(this.adb, ['install', '-r', apk])
    this.run(this.adb, ['shell', 'pm', 'clear', this.packageName])
    this.run(this.adb, ['shell', 'am', 'start', '-W', '-n', `${this.packageName}/.MainActivity`])
    let pid
    for (let attempt = 0; attempt < 60; attempt++) {
      pid = this.run(this.adb, ['shell', 'pidof', this.packageName])
      if (this.run(this.adb, ['shell', 'cat', '/proc/net/unix']).includes(`webview_devtools_remote_${pid}`)) break
      await this.pause(500)
    }
    assert.match(pid, /^\d+$/)
    return pid
  }

  static async answer(page, method, route, click) {
    const answered = page.waitForResponse((response) => response.request().method() === method && new URL(response.url()).pathname === route, { timeout: this.timeout })
    const [response] = await Promise.all([answered, click()])
    return response
  }

  static async checkEveryPath(page, pid) {
    const shows = (text) => page.waitForFunction((expected) => document.body.textContent.includes(expected), text, { timeout: this.timeout })
    const press = (name) => page.getByRole('button', { name }).click()
    try { await page.getByRole('button', { name: 'Load note' }).waitFor({ timeout: this.timeout }) }
    catch (error) { console.error('Android startup document:', page.url(), (await page.content()).slice(0, 4096)); throw error }
    await press('Load note')
    await shows('Note from Yon')
    console.log('PASS: Android page reached Yon with an HTTP GET')
    await press('Save note')
    await shows('Saved from another origin')
    console.log('PASS: Android page reached Yon with an HTTP POST')
    await shows('ANVIL-1 · 3')
    console.log('PASS: Android page received a Yon stream')
    await shows('Welcome to the room')
    await press('Say hello')
    await shows('Hello from a tab')
    console.log('PASS: Android page sent and received over a Yon socket')
    await press('Send topic')
    await shows('Topic from Yon')
    console.log('PASS: Android page received a Yon topic')
    const refusedEntry = await this.answer(page, 'POST', '/api/entries', () => press('Send invalid entry'))
    assert.equal(refusedEntry.status(), 422)
    await shows('Entry refused by Yon')
    console.log('PASS: Android page saw Yon refuse an invalid value with 422')
    const anonymous = await this.answer(page, 'GET', '/api/account', () => press('Call without token'))
    assert.equal(anonymous.status(), 401)
    await shows('Refused without a token')
    console.log('PASS: Android page saw a signed-in route answer 401 without a token')
    const signedIn = await this.answer(page, 'GET', '/api/account', () => press('Call with token'))
    assert.equal(signedIn.status(), 200)
    await shows('Signed in as ada')
    console.log('PASS: Android page reached a signed-in route with an Authorization header')
    await press('Ask Yon natively')
    await shows('Hello from Yon to Kotlin')
    const nativePid = await page.evaluate(async () => __tachyonTac.service('nativeReach').processId())
    assert.equal(nativePid, Number(pid), 'The Kotlin HTTP call must run inside the Android app process')
    console.log('PASS: Android Kotlin service reached Yon through the native transport in the app process')
  }

  static async main() {
    assert.ok(this.sdk, 'ANDROID_HOME or ANDROID_SDK_ROOT is required')
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-android-yon-origin-'))
    const port = await this.freePort()
    let browser
    let forwarded
    let reversed = false
    try {
      this.writeFixture(project, port)
      const binary = this.binary()
      const apk = this.build(binary, project)
      await withServer(binary, ['serve', project, '--no-watch', '--no-bundle', '--out-dir', 'dist/android', '--port', String(port)], async () => {
        this.run(this.adb, ['wait-for-device'])
        this.run(this.adb, ['reverse', `tcp:${port}`, `tcp:${port}`])
        reversed = true
        const pid = await this.installAndLaunch(apk)
        forwarded = this.run(this.adb, ['forward', 'tcp:0', `localabstract:webview_devtools_remote_${pid}`])
        browser = await chromium.connectOverCDP(`http://127.0.0.1:${forwarded}`, { noDefaults: true })
        const page = browser.contexts()[0].pages()[0]
        assert.ok(page, 'Android WebView exposes a live document')
        page.on('pageerror', (error) => console.error('Android page error:', error.message))
        page.on('console', (message) => { if (message.type() === 'error') console.error('Android console:', message.text()) })
        await this.checkEveryPath(page, pid)
      })
    } finally {
      if (browser) await browser.close()
      if (forwarded) this.run(this.adb, ['forward', '--remove', `tcp:${forwarded}`])
      if (reversed) this.run(this.adb, ['reverse', '--remove', `tcp:${port}`])
      spawnSync(this.adb, ['shell', 'am', 'force-stop', this.packageName], { encoding: 'utf8' })
      rmSync(project, { recursive: true, force: true })
    }
  }
}

await AndroidYonOriginTest.main()

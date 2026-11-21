#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdtemp, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class RecorderBrowserCheck {
  static template = (name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url)

  static files = { '/tac-media.js': 'tac-media.js', '/tac-recorder.js': 'tac-recorder.js' }

  static engines = [
    { engine: chromium, options: { args: ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream', '--autoplay-policy=no-user-gesture-required'] }, testPicker: true },
    { engine: firefox, options: { firefoxUserPrefs: { 'media.navigator.streams.fake': true, 'media.navigator.permission.disabled': true } }, testPicker: false },
    { engine: webkit, options: {}, testPicker: false, persistent: true },
  ]

  static recordMs = 5000

  static waitMs = 30000

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Recorder</title>
    <meta name="tachyon-browser-apis" content="files microphone camera"></head><body>
    <script>
      window.events = []
      const reply = (event) => window.events.push(event)
      const media = () => globalThis[Symbol.for('tachyon.tac.media')]
      const recorder = () => globalThis[Symbol.for('tachyon.tac.recorder')]
      window.useTestPicker = () => {
        globalThis.showSaveFilePicker = async (options) => {
          window.pickedName = options.suggestedName
          const folder = await (await navigator.storage.getDirectory()).getDirectoryHandle('picked', { create: true })
          window.pickedHandle = await folder.getFileHandle(options.suggestedName, { create: true })
          return window.pickedHandle
        }
      }
      window.capture = () => {
        for (const name of ['mic1', 'mic2', 'mic3']) media().captureMicrophone(reply, name, '', false)
        media().captureCamera(reply, 'cam1', '', 'low')
        media().captureCamera(reply, 'cam2', '', 'low')
      }
      window.supported = () => recorder().supported()
      window.call = (name, ...values) => { try { recorder()[name](reply, ...values); return 'ok' } catch (error) { return error.message } }
      window.savedBytes = async () => {
        const handle = window.pickedHandle ?? await (await (await navigator.storage.getDirectory()).getDirectoryHandle('tachyon-recordings')).getFileHandle(window.savedName)
        return [...new Uint8Array(await (await handle.getFile()).arrayBuffer())]
      }
    </script>
    <script src="/tac-media.js"></script>
    <script src="/tac-recorder.js"></script>
  </body></html>`

  static async serve() {
    const scripts = {}
    for (const [path, name] of Object.entries(RecorderBrowserCheck.files)) scripts[path] = await readFile(RecorderBrowserCheck.template(name), 'utf8')
    const server = createServer((request, response) => {
      const script = scripts[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? RecorderBrowserCheck.page)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    return server
  }

  static async run() {
    const server = await RecorderBrowserCheck.serve()
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const setup of RecorderBrowserCheck.engines) await RecorderBrowserCheck.runEngine(setup, url)
    } finally {
      server.close()
    }
  }

  static async runEngine(setup, url) {
    const name = setup.engine.name()
    const browser = await RecorderBrowserCheck.launch(setup)
    try {
      const context = setup.persistent ? browser : await browser.newContext({ acceptDownloads: true })
      await context.grantPermissions(['microphone', 'camera']).catch(() => null)
      const page = await context.newPage()
      await page.goto(url)
      if (setup.testPicker) await page.evaluate(() => window.useTestPicker())
      const result = await RecorderBrowserCheck.record(page)
      console.log(name, JSON.stringify({ ...result, bytes: undefined, size: result.bytes?.length }))
      RecorderBrowserCheck.check(name, result, setup)
    } finally {
      await browser.close()
    }
  }

  static async launch(setup) {
    if (!setup.persistent) return setup.engine.launch(setup.options)
    const profile = await mkdtemp(path.join(tmpdir(), 'tachyon-recorder-'))
    return setup.engine.launchPersistentContext(profile, { ...setup.options, acceptDownloads: true })
  }

  static async waitFor(page, done) {
    const deadline = Date.now() + RecorderBrowserCheck.waitMs
    while (Date.now() < deadline) {
      const events = await page.evaluate(() => window.events)
      if (done(events)) return events
      await page.waitForTimeout(200)
    }
    return page.evaluate(() => window.events)
  }

  static async record(page) {
    const supported = await page.evaluate(() => window.supported())
    if (!supported) return { supported, refused: await page.evaluate(() => window.call('start', 'meeting', 'mic1', 'cam1', 'Meeting', '')) }
    await page.evaluate(() => window.capture())
    const captured = await RecorderBrowserCheck.waitFor(page, (events) => events.filter((event) => event[0] === 'MediaTrackStarted').length >= 5)
    const capturedCount = captured.filter((event) => event[0] === 'MediaTrackStarted').length
    await page.evaluate(() => { window.events = [] })
    const downloadSeen = page.waitForEvent('download', { timeout: RecorderBrowserCheck.waitMs }).catch(() => null)
    const start = await page.evaluate(() => window.call('start', 'meeting', 'mic1 mic2', 'cam1', 'Meeting', ''))
    const started = await RecorderBrowserCheck.waitFor(page, (events) => events.some((event) => event[0] === 'RecordingStarted' || event[0] === 'RecordingFailed'))
    const startedEvent = started.find((event) => event[0] === 'RecordingStarted' || event[0] === 'RecordingFailed')
    if (startedEvent?.[0] !== 'RecordingStarted') return { supported, capturedCount, start, startedEvent }
    await page.evaluate((file) => { window.savedName = file }, startedEvent[1].fileName)
    await page.waitForTimeout(RecorderBrowserCheck.recordMs / 2)
    const changes = await page.evaluate(() => [window.call('addAudio', 'meeting', 'mic3'), window.call('showVideo', 'meeting', 'cam2'), window.call('pause', 'meeting'), window.call('resume', 'meeting')])
    await page.waitForTimeout(RecorderBrowserCheck.recordMs / 2)
    await page.evaluate(() => window.call('stop', 'meeting'))
    const events = await RecorderBrowserCheck.waitFor(page, (list) => list.some((event) => event[0] === 'RecordingStopped' || event[0] === 'RecordingFailed'))
    const download = await Promise.race([downloadSeen, new Promise((resolve) => setTimeout(() => resolve(null), 2000))])
    const bytes = await page.evaluate(() => window.savedBytes())
    return {
      supported, capturedCount, start, startedEvent, changes,
      names: events.map((event) => event[0]),
      stopped: events.find((event) => event[0] === 'RecordingStopped')?.[1] ?? null,
      failed: events.find((event) => event[0] === 'RecordingFailed')?.[1] ?? null,
      downloadName: download?.suggestedFilename() ?? '',
      pickedName: await page.evaluate(() => window.pickedName ?? ''),
      bytes,
    }
  }

  static check(name, result, setup) {
    if (!result.supported) {
      assert.equal(result.refused, 'Recording is not available here (unsupported).', `${name}: start says unsupported when supported() is false`)
      console.log(`${name}: no save picker and no writable private file, so the recorder answers unsupported here`)
      return
    }
    assert.equal(result.capturedCount, 5, `${name}: three microphones and two cameras captured`)
    assert.equal(result.start, 'ok', `${name}: start is accepted`)
    assert.equal(result.startedEvent?.[0], 'RecordingStarted', `${name}: the recording starts (${JSON.stringify(result.startedEvent)})`)
    assert.deepEqual(result.changes, ['ok', 'ok', 'ok', 'ok'], `${name}: addAudio, showVideo, pause and resume are accepted`)
    assert.equal(result.failed, null, `${name}: no failure`)
    assert.ok(result.names.includes('RecordingPaused') && result.names.includes('RecordingResumed'), `${name}: pause and resume answer`)
    assert.ok(result.stopped && result.stopped.bytes > 0 && result.stopped.bytes === result.bytes.length, `${name}: the stop event counts every byte on disk (${result.stopped?.bytes} vs ${result.bytes.length})`)
    assert.ok(result.stopped.seconds >= 3, `${name}: about 5 seconds recorded (${result.stopped.seconds})`)
    const format = result.startedEvent[1].format
    if (setup.testPicker) assert.equal(result.pickedName, result.startedEvent[1].fileName, `${name}: the save picker gets the file name`)
    else assert.equal(result.downloadName, result.startedEvent[1].fileName, `${name}: the private file is downloaded at stop`)
    if (format === 'mp4') assert.equal(String.fromCharCode(...result.bytes.slice(4, 8)), 'ftyp', `${name}: an MP4 file starts with its ftyp box`)
    else RecorderBrowserCheck.checkWebm(name, result)
  }

  static checkWebm(name, result) {
    assert.deepEqual(result.bytes.slice(0, 4), [0x1a, 0x45, 0xdf, 0xa3], `${name}: a WebM file starts with its EBML header`)
    const at = RecorderBrowserCheck.find(result.bytes, [0x44, 0x89, 0x88])
    assert.ok(at > 0, `${name}: the WebM file has a Duration element`)
    const duration = new DataView(new Uint8Array(result.bytes.slice(at + 3, at + 11)).buffer).getFloat64(0)
    assert.ok(Math.abs(duration / 1000 - result.stopped.seconds) < 1, `${name}: the WebM duration (${duration} ms) matches the recording`)
  }

  static find(bytes, pattern) {
    for (let index = 0; index <= bytes.length - pattern.length; index += 1) if (pattern.every((value, offset) => bytes[index + offset] === value)) return index
    return -1
  }
}

await RecorderBrowserCheck.run()
console.log('recorder browser checks passed')

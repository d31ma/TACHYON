#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class MediaFramesBrowserCheck {
  static template = (name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url)

  static files = { '/tac-media.js': 'tac-media.js', '/.tachyon/tac-media-frames.js': 'tac-media-frames.js' }

  static engines = [chromium, firefox, webkit]

  static settleMs = 6000

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Frames</title></head><body>
    <canvas id="source" width="320" height="240"></canvas>
    <video id="seen" autoplay muted playsinline></video>
    <script>
      const keys = { alice: '000102030405060708090a0b0c0d0e0f', mallory: '0f0e0d0c0b0a09080706050403020100' }
      const bytes = (hex) => new Uint8Array(hex.match(/../g).map((pair) => parseInt(pair, 16)))
      globalThis[Symbol.for('tachyon.tac.crypto.hooks')] = Object.freeze({
        key: (name) => keys[name] ? crypto.subtle.importKey('raw', bytes(keys[name]), 'HKDF', false, ['deriveKey', 'deriveBits']) : Promise.resolve(null),
      })
      const peers = new Map()
      globalThis[Symbol.for('tachyon.tac.peers.hooks')] = Object.freeze({ connection: (id) => peers.get(id) ?? null })
      const canvas = document.getElementById('source')
      const paint = canvas.getContext('2d')
      let tick = 0
      setInterval(() => { tick += 1; paint.fillStyle = 'hsl(' + (tick * 7 % 360) + ' 80% 50%)'; paint.fillRect(0, 0, 320, 240); paint.fillStyle = '#fff'; paint.fillText(String(tick), 20 + tick % 200, 120) }, 33)
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
      const gathered = (peer) => new Promise((resolve) => {
        if (peer.iceGatheringState === 'complete') return resolve(peer.localDescription.sdp)
        peer.addEventListener('icegatheringstatechange', () => peer.iceGatheringState === 'complete' && resolve(peer.localDescription.sdp))
      })
      const decoded = async (peer) => {
        let frames = 0
        for (const report of (await peer.getStats()).values()) if (report.type === 'inbound-rtp' && report.kind === 'video') frames += report.framesDecoded ?? 0
        return frames
      }
      const media = () => globalThis[Symbol.for('tachyon.tac.media')]
      window.events = []
      const reply = (event) => window.events.push(event[0])
      window.send = async () => {
        const peer = new RTCPeerConnection()
        peers.set('peer-a', peer)
        peer.addTrack(canvas.captureStream(30).getVideoTracks()[0])
        media().encryptFrames(reply, 'peer-a', true)
        media().useFrameKey(reply, 7, 'alice')
        await wait(500)
        media().sendFrameKey(reply, 7)
        await peer.setLocalDescription(await peer.createOffer())
        return gathered(peer)
      }
      window.accept = (answer) => peers.get('peer-a').setRemoteDescription({ type: 'answer', sdp: answer })
      window.receive = async (scenario, offer) => {
        const peer = new RTCPeerConnection()
        peers.set('peer-b', peer)
        peer.ontrack = (event) => { document.getElementById('seen').srcObject = new MediaStream([event.track]) }
        if (scenario.receiverEncrypts) media().encryptFrames(reply, 'peer-b', true)
        if (scenario.receiverKey) media().useFrameKey(reply, 7, scenario.receiverKey)
        await wait(500)
        await peer.setRemoteDescription({ type: 'offer', sdp: offer })
        await peer.setLocalDescription(await peer.createAnswer())
        return gathered(peer)
      }
      window.result = async () => ({ decoded: await decoded(peers.get('peer-b')), events: window.events })
      window.codec = () => (peers.get('peer-a').getSenders()[0].getParameters().codecs ?? [])[0]?.mimeType ?? ''
    </script>
    <script src="/tac-media.js"></script>
  </body></html>`

  static async serve() {
    const scripts = {}
    for (const [path, name] of Object.entries(MediaFramesBrowserCheck.files)) scripts[path] = await readFile(MediaFramesBrowserCheck.template(name), 'utf8')
    const server = createServer((request, response) => {
      const script = scripts[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? MediaFramesBrowserCheck.page)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    return server
  }

  static async run() {
    const server = await MediaFramesBrowserCheck.serve()
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const engine of MediaFramesBrowserCheck.engines) await MediaFramesBrowserCheck.runEngine(engine, url)
    } finally {
      server.close()
    }
  }

  static async call(browser, url, scenario) {
    const sender = await browser.newPage()
    const receiver = await browser.newPage()
    await Promise.all([sender.goto(url), receiver.goto(url)])
    const offer = await sender.evaluate(() => window.send())
    const answer = await receiver.evaluate(([given, sdp]) => window.receive(given, sdp), [scenario, offer])
    await sender.evaluate((sdp) => window.accept(sdp), answer)
    await receiver.waitForTimeout(MediaFramesBrowserCheck.settleMs)
    const seen = await receiver.evaluate(() => window.result())
    const sent = await sender.evaluate(() => ({ codec: window.codec(), events: window.events }))
    await Promise.all([sender.close(), receiver.close()])
    return { decoded: seen.decoded, codec: sent.codec, sentEvents: sent.events, events: seen.events }
  }

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage()
      await page.goto(url)
      const supported = await page.evaluate(() => typeof RTCRtpScriptTransform === 'function')
      assert.equal(supported, true, `${engine.name()} has RTCRtpScriptTransform`)
      const scenario = (options) => MediaFramesBrowserCheck.call(browser, url, options)
      const sameKey = await scenario({ receiverEncrypts: true, receiverKey: 'alice' })
      const wrongKey = await scenario({ receiverEncrypts: true, receiverKey: 'mallory' })
      const noKey = await scenario({ receiverEncrypts: true, receiverKey: '' })
      const plain = await scenario({ receiverEncrypts: false, receiverKey: '' })
      console.log(engine.name(), JSON.stringify({ sameKey, wrongKey, noKey, plain }))
      assert.ok(sameKey.decoded > 0, `${engine.name()}: the same key decodes video`)
      assert.ok(sameKey.events.includes('MediaFrameKeyReady'), `${engine.name()}: keys report ready`)
      assert.equal(wrongKey.decoded, 0, `${engine.name()}: the wrong key decodes nothing`)
      assert.ok(wrongKey.events.includes('MediaFrameRejected'), `${engine.name()}: the wrong key reports rejected frames`)
      assert.equal(noKey.decoded, 0, `${engine.name()}: no key decodes nothing`)
      assert.ok(noKey.events.includes('MediaFrameKeyMissing'), `${engine.name()}: no key reports the missing key`)
      assert.equal(plain.decoded, 0, `${engine.name()}: a receiver without the transform cannot decode the sent frames`)
    } finally {
      await browser.close()
    }
  }
}

await MediaFramesBrowserCheck.run()
console.log('media frames browser checks passed')

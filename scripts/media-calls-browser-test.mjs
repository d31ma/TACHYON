#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdtemp, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class MediaCallsBrowserCheck {
  static template = (name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url)

  static files = { '/tac-peers.js': 'tac-peers.js', '/tac-media.js': 'tac-media.js', '/tac-media-view.js': 'tac-media-view.js' }

  static engines = [
    { engine: chromium, options: { args: ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream', '--autoplay-policy=no-user-gesture-required'] }, simulcast: true, toneFlag: '--use-file-for-fake-audio-capture=' },
    { engine: firefox, options: { firefoxUserPrefs: { 'media.navigator.streams.fake': true, 'media.navigator.permission.disabled': true, 'media.autoplay.default': 0 } }, simulcast: false },
    { engine: webkit, options: {}, simulcast: false, mockMicrophoneSilent: true, levelsSaturate: true },
  ]

  static waitMs = 30000

  static pollMs = 200

  static audioTracks = 50

  static videoTracks = 9

  static gains = [0.2, 0.5, 0.9]

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Calls</title>
    <meta name="tachyon-browser-apis" content="microphone camera peers"></head><body>
    <script>
      window.events = []
      window.kept = []
      const reply = (event) => window.events.push(event)
      const peers = () => globalThis[Symbol.for('tachyon.tac.peers')]
      const media = () => globalThis[Symbol.for('tachyon.tac.media')]
      const tracks = () => globalThis[Symbol.for('tachyon.tac.media.tracks')]
      const rawPeer = (id) => globalThis[Symbol.for('tachyon.tac.peers.hooks')].connection(id)
      window.drain = () => { const taken = window.events.splice(0); window.kept.push(...taken); return taken }
      window.capture = (names) => { for (const [kind, name] of names) media()[kind === 'audio' ? 'captureMicrophone' : 'captureCamera'](reply, name, '', kind === 'audio' ? true : 'low') }
      window.openPeer = () => peers().open(reply, '', '')
      window.media = (call, ...values) => media()[call](reply, ...values)
      window.peers = (call, ...values) => peers()[call](reply, ...values)
      window.state = (id) => ({ connection: rawPeer(id).connectionState, signaling: rawPeer(id).signalingState })
      window.framesDecoded = async (id) => {
        let frames = 0
        for (const item of (await rawPeer(id).getStats()).values()) if (item.type === 'inbound-rtp' && item.kind === 'video') frames += item.framesDecoded ?? 0
        return frames
      }
      window.scale = async (audioCount, videoCount, gains) => {
        const id = peers().open(reply, '', '')
        media().stats(reply, 'bind', id)
        media().speaking(reply, true)
        const receiver = rawPeer(id)
        const sender = new RTCPeerConnection()
        sender.onicecandidate = (event) => event.candidate && receiver.addIceCandidate(event.candidate)
        receiver.addEventListener('icecandidate', (event) => event.candidate && sender.addIceCandidate(event.candidate))
        const audio = new AudioContext()
        await audio.resume()
        for (let index = 0; index < audioCount; index += 1) {
          const tone = audio.createOscillator()
          const gain = audio.createGain()
          gain.gain.value = gains[index] ?? 0
          const out = audio.createMediaStreamDestination()
          tone.frequency.value = 300 + index * 20
          tone.connect(gain).connect(out)
          tone.start()
          sender.addTrack(out.stream.getAudioTracks()[0], new MediaStream())
        }
        for (let index = 0; index < videoCount; index += 1) {
          const canvas = document.createElement('canvas')
          canvas.width = 160
          canvas.height = 120
          const paint = canvas.getContext('2d')
          let tick = 0
          setInterval(() => { tick += 1; paint.fillStyle = 'hsl(' + (tick * 9 + index * 40) % 360 + ' 80% 50%)'; paint.fillRect(0, 0, 160, 120) }, 50)
          sender.addTrack(canvas.captureStream(20).getVideoTracks()[0], new MediaStream())
        }
        const offer = await sender.createOffer()
        await sender.setLocalDescription(offer)
        await receiver.setRemoteDescription(offer)
        await receiver.setLocalDescription(await receiver.createAnswer())
        await sender.setRemoteDescription(receiver.localDescription)
        window.scaleSender = sender
        return { id, offerBytes: new TextEncoder().encode(offer.sdp).length }
      }
      window.listen = async () => {
        const id = peers().open(reply, '', '')
        const sender = new RTCPeerConnection()
        const audio = new AudioContext()
        const out = audio.createMediaStreamDestination()
        audio.createOscillator().connect(out)
        const canvas = document.createElement('canvas')
        canvas.getContext('2d').fillRect(0, 0, 16, 16)
        sender.addTrack(out.stream.getAudioTracks()[0], new MediaStream())
        sender.addTrack(canvas.captureStream(5).getVideoTracks()[0], new MediaStream())
        const offer = await sender.createOffer()
        await sender.setLocalDescription(offer)
        peers().answer(reply, id, offer.sdp)
        window.listenSender = sender
        return id
      }
      window.showViews = (names) => {
        for (const name of names) {
          const view = document.createElement('media-view')
          view.setAttribute('track', name)
          view.setAttribute('label', name)
          document.body.append(view)
        }
        return document.querySelectorAll('media-view').length
      }
    </script>
    <script src="/tac-peers.js"></script>
    <script src="/tac-media.js"></script>
    <script src="/tac-media-view.js"></script>
  </body></html>`

  static async serve() {
    const scripts = {}
    for (const [path, name] of Object.entries(MediaCallsBrowserCheck.files)) scripts[path] = await readFile(MediaCallsBrowserCheck.template(name), 'utf8')
    const server = createServer((request, response) => {
      const script = scripts[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? MediaCallsBrowserCheck.page)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    return server
  }

  static async tone() {
    const rate = 48000
    const samples = rate * 2
    const wave = Buffer.alloc(44 + samples * 2)
    wave.write('RIFF', 0)
    wave.writeUInt32LE(36 + samples * 2, 4)
    wave.write('WAVEfmt ', 8)
    wave.writeUInt32LE(16, 16)
    wave.writeUInt16LE(1, 20)
    wave.writeUInt16LE(1, 22)
    wave.writeUInt32LE(rate, 24)
    wave.writeUInt32LE(rate * 2, 28)
    wave.writeUInt16LE(2, 32)
    wave.writeUInt16LE(16, 34)
    wave.write('data', 36)
    wave.writeUInt32LE(samples * 2, 40)
    for (let index = 0; index < samples; index += 1) wave.writeInt16LE(Math.round(Math.sin(2 * Math.PI * 440 * index / rate) * 16000), 44 + index * 2)
    const file = path.join(await mkdtemp(path.join(tmpdir(), 'tachyon-calls-')), 'tone.wav')
    await writeFile(file, wave)
    return file
  }

  static async run() {
    const tone = await MediaCallsBrowserCheck.tone()
    for (const setup of MediaCallsBrowserCheck.engines) if (setup.toneFlag) setup.options.args.push(setup.toneFlag + tone)
    const server = await MediaCallsBrowserCheck.serve()
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const setup of MediaCallsBrowserCheck.engines) await MediaCallsBrowserCheck.runEngine(setup, url)
    } finally {
      server.close()
    }
  }

  static async runEngine(setup, url) {
    const name = setup.engine.name()
    const browser = await setup.engine.launch(setup.options)
    try {
      const context = await browser.newContext()
      await context.grantPermissions(['microphone', 'camera']).catch(() => null)
      const listen = await MediaCallsBrowserCheck.listenOnly(context, url)
      console.log(name, 'listen only', JSON.stringify(listen))
      MediaCallsBrowserCheck.checkListenOnly(name, listen)
      const call = await MediaCallsBrowserCheck.call(context, url, setup)
      console.log(name, 'call', JSON.stringify(call))
      MediaCallsBrowserCheck.checkCall(name, call, setup)
      const scale = await MediaCallsBrowserCheck.scale(context, url, setup)
      console.log(name, 'scale', JSON.stringify(scale))
      MediaCallsBrowserCheck.checkScale(name, scale, setup)
    } finally {
      await browser.close()
    }
  }

  static async listenOnly(context, url) {
    const page = await context.newPage()
    await page.goto(url)
    const id = await page.evaluate(() => window.listen())
    const deadline = Date.now() + MediaCallsBrowserCheck.waitMs
    let arrived = []
    while (Date.now() < deadline && arrived.length < 2) {
      await page.waitForTimeout(MediaCallsBrowserCheck.pollMs)
      arrived = await page.evaluate((connection) => window.events.filter((event) => event[0] === 'MediaTrackArrived' && event[1].connection === connection).map((event) => event[1].kind), id)
    }
    const answered = await page.evaluate(() => window.events.some((event) => event[0] === 'PeerDescription'))
    await page.close()
    return { id, answered, arrived: arrived.sort() }
  }

  static checkListenOnly(name, listen) {
    assert.ok(listen.answered, `${name}: the listening page answered the offer through Peers (${JSON.stringify(listen)})`)
    assert.deepEqual(listen.arrived, ['audio', 'video'], `${name}: a page that only receives, and never calls Media, still gets MediaTrackArrived for each track (${JSON.stringify(listen)})`)
  }

  static async call(context, url, setup) {
    const pages = { a: await context.newPage(), b: await context.newPage() }
    await Promise.all([pages.a.goto(url), pages.b.goto(url)])
    const logs = { a: [], b: [] }
    await pages.a.evaluate(() => window.capture([['audio', 'mic'], ['video', 'cam'], ['video', 'cam2']]))
    await pages.b.evaluate(() => window.capture([['audio', 'mic']]))
    const captured = await MediaCallsBrowserCheck.until(pages, logs, () => MediaCallsBrowserCheck.count(logs.a, 'MediaTrackStarted') >= 3 && MediaCallsBrowserCheck.count(logs.b, 'MediaTrackStarted') >= 1)
    if (!captured) return { skipped: 'capture', failures: logs.a.concat(logs.b).filter((event) => event[0] === 'MediaFailed') }
    const ids = { a: await pages.a.evaluate(() => window.openPeer()), b: await pages.b.evaluate(() => window.openPeer()) }
    await pages.b.evaluate(() => window.media('speaking', true))
    await pages.a.evaluate(([id, simulcast]) => { window.media('send', id, 'mic', 'alice', false); window.media('send', id, 'cam', 'alice', simulcast) }, [ids.a, setup.simulcast])
    const first = await MediaCallsBrowserCheck.until(pages, logs, () => MediaCallsBrowserCheck.arrived(logs.b).length >= 2, ids)
    const arrivedAtB = MediaCallsBrowserCheck.arrived(logs.b)
    const audioName = arrivedAtB.find((event) => event.kind === 'audio')?.track ?? ''
    await MediaCallsBrowserCheck.until(pages, logs, async () => (await pages.b.evaluate((id) => window.framesDecoded(id), ids.b)) > 0, ids)
    const views = await pages.b.evaluate((names) => window.showViews(names), arrivedAtB.filter((event) => event.kind === 'video').map((event) => event.track))
    const receiving = (kind) => (rows) => rows.some((row) => row.direction === 'receive' && row.kind === kind && row.bitrate > 0)
    const receivedStats = await MediaCallsBrowserCheck.stats(pages.b, logs.b, ids.b, pages, ids, logs, (rows) => (setup.mockMicrophoneSilent || receiving('audio')(rows)) && receiving('video')(rows))
    await pages.b.evaluate((id) => window.media('send', id, 'mic', 'bob', false), ids.b)
    const back = await MediaCallsBrowserCheck.until(pages, logs, () => MediaCallsBrowserCheck.arrived(logs.a).length >= 1, ids)
    const sentStats = await MediaCallsBrowserCheck.stats(pages.a, logs.a, ids.a, pages, ids, logs, (rows) => rows.some((row) => row.direction === 'send' && row.track === 'cam' && row.bitrate > 0))
    const framesBefore = await pages.b.evaluate((id) => window.framesDecoded(id), ids.b)
    await pages.a.evaluate((id) => window.media('replace', id, 'cam', 'cam2'), ids.a)
    await MediaCallsBrowserCheck.until(pages, logs, async () => (await pages.b.evaluate((id) => window.framesDecoded(id), ids.b)) > framesBefore + 30, ids)
    const framesAfter = await pages.b.evaluate((id) => window.framesDecoded(id), ids.b)
    if (!setup.mockMicrophoneSilent) await MediaCallsBrowserCheck.until(pages, logs, () => logs.b.some((event) => event[0] === 'MediaSpeaking' && event[1].track === audioName && event[1].speaking), ids)
    await pages.a.evaluate((id) => window.media('unsend', id, 'mic'), ids.a)
    const ended = await MediaCallsBrowserCheck.until(pages, logs, () => logs.b.some((event) => event[0] === 'MediaTrackEnded' && event[1].track === audioName), ids)
    await Promise.all([pages.a.close(), pages.b.close()])
    return {
      first, back, ended, views, framesBefore, framesAfter, audioName,
      arrivedAtB, arrivedAtA: MediaCallsBrowserCheck.arrived(logs.a),
      sendingAtA: logs.a.filter((event) => event[0] === 'MediaSending').map((event) => event[1].track),
      receivedStats, sentStats,
      speaking: logs.b.filter((event) => event[0] === 'MediaSpeaking' || event[0] === 'MediaLoudest').slice(0, 6),
      endedAtB: logs.b.filter((event) => event[0] === 'MediaTrackEnded').map((event) => event[1]),
      failures: logs.a.concat(logs.b).filter((event) => event[0] === 'MediaFailed' || event[0] === 'PeerFailed'),
    }
  }

  static async stats(page, log, id, pages, ids, logs, flowing) {
    let rows = []
    let round = 0
    const deadline = Date.now() + MediaCallsBrowserCheck.waitMs
    while (Date.now() < deadline) {
      round += 1
      const request = `stats-${round}`
      await page.evaluate(([connection, name]) => window.media('stats', name, connection), [id, request])
      await MediaCallsBrowserCheck.until(pages, logs, () => log.some((event) => event[0] === 'MediaStats' && event[1].request === request), ids)
      rows = log.filter((event) => event[0] === 'MediaStats' && event[1].request === request).map((event) => event[1])
      if (round > 1 && flowing(rows)) return rows
      await page.waitForTimeout(1000)
    }
    return rows
  }

  static arrived(log) {
    return log.filter((event) => event[0] === 'MediaTrackArrived').map((event) => event[1])
  }

  static count(log, name) {
    return log.filter((event) => event[0] === name).length
  }

  static async until(pages, logs, done, ids = null) {
    const deadline = Date.now() + MediaCallsBrowserCheck.waitMs
    while (Date.now() < deadline) {
      if (ids) await MediaCallsBrowserCheck.signal(pages, logs, ids)
      else for (const side of ['a', 'b']) logs[side].push(...(await pages[side].evaluate(() => window.drain())))
      if (await done()) return true
      await new Promise((resolve) => setTimeout(resolve, MediaCallsBrowserCheck.pollMs))
    }
    return false
  }

  static async signal(pages, logs, ids) {
    for (const [side, other] of [['a', 'b'], ['b', 'a']]) {
      const taken = await pages[side].evaluate(() => window.drain())
      logs[side].push(...taken)
      for (const [name, payload] of taken) await MediaCallsBrowserCheck.route(pages, ids, side, other, name, payload)
    }
  }

  static async route(pages, ids, side, other, name, payload) {
    if (name === 'PeerNegotiationNeeded') return pages[side].evaluate((id) => window.state(id).signaling === 'stable' && window.peers('offer', id), ids[side])
    if (name === 'PeerCandidate') return pages[other].evaluate(([id, candidate]) => window.peers('addCandidate', id, candidate), [ids[other], payload.candidateJson])
    if (name !== 'PeerDescription') return null
    const call = payload.kind === 'offer' ? 'answer' : 'accept'
    return pages[other].evaluate(([id, verb, sdp]) => window.peers(verb, id, sdp), [ids[other], call, payload.sdp])
  }

  static checkCall(name, call, setup) {
    if (call.skipped) {
      assert.equal(name, 'webkit', `${name}: capture works with fake devices (${JSON.stringify(call.failures)})`)
      return
    }
    assert.deepEqual(call.failures, [], `${name}: no media or peer failure`)
    assert.deepEqual(call.sendingAtA, ['mic', 'cam'], `${name}: send answers MediaSending`)
    assert.ok(call.first, `${name}: the receiver gets the audio and the video track`)
    assert.deepEqual(call.arrivedAtB.map((event) => event.kind).sort(), ['audio', 'video'], `${name}: one audio and one video track arrive`)
    assert.ok(call.arrivedAtB.every((event) => /^peer-1\.\d+$/.test(event.track) && event.connection === 'peer-1' && event.stream !== ''), `${name}: remote tracks are named <connection>.<mid> with the sender's stream`)
    assert.equal(call.arrivedAtB[0].stream, call.arrivedAtB[1].stream, `${name}: tracks sent with one stream name arrive in one stream`)
    assert.equal(call.views, 1, `${name}: a media-view shows the remote video`)
    assert.ok(call.receivedStats.some((row) => row.direction === 'receive' && row.kind === 'video' && row.bitrate > 0), `${name}: stats show received video`)
    if (setup.mockMicrophoneSilent) console.log(`${name}: the mock microphone sends no audio, so received audio and remote talking are checked only in the 50-track test`)
    else assert.ok(call.receivedStats.some((row) => row.direction === 'receive' && row.kind === 'audio' && row.bitrate > 0), `${name}: stats show received audio`)
    assert.ok(call.back, `${name}: the other side's microphone arrives after it negotiates`)
    assert.ok(call.sentStats.some((row) => row.direction === 'send' && row.track === 'cam' && row.bitrate > 0), `${name}: stats show the sent camera`)
    const layers = call.sentStats.filter((row) => row.track === 'cam').map((row) => row.layer).sort()
    if (setup.simulcast) assert.deepEqual(layers, ['f', 'h', 'q'], `${name}: simulcast sends three layers`)
    assert.ok(call.framesAfter > call.framesBefore, `${name}: video keeps playing after replace`)
    if (!setup.mockMicrophoneSilent) assert.ok(call.speaking.some((event) => event[0] === 'MediaSpeaking' && event[1].track === call.audioName && event[1].speaking), `${name}: the receiver hears the sender talking`)
    assert.ok(call.ended, `${name}: unsend ends the track on the other side`)
    assert.ok(call.endedAtB.some((event) => event.track === call.audioName && event.reason === 'remote'), `${name}: the ended track says remote`)
  }

  static async scale(context, url, setup) {
    const page = await context.newPage()
    await page.goto(url)
    const logs = []
    const started = await page.evaluate(([audio, video, gains]) => window.scale(audio, video, gains), [MediaCallsBrowserCheck.audioTracks, MediaCallsBrowserCheck.videoTracks, MediaCallsBrowserCheck.gains])
    const total = MediaCallsBrowserCheck.audioTracks + MediaCallsBrowserCheck.videoTracks
    const pages = { a: page, b: page }
    const drainOnce = async () => logs.push(...(await page.evaluate(() => window.drain())))
    const wait = async (done) => {
      const deadline = Date.now() + MediaCallsBrowserCheck.waitMs
      while (Date.now() < deadline) {
        await drainOnce()
        if (done()) return true
        await page.waitForTimeout(MediaCallsBrowserCheck.pollMs)
      }
      return false
    }
    const allArrived = await wait(() => MediaCallsBrowserCheck.arrived(logs).length >= total)
    const arrived = MediaCallsBrowserCheck.arrived(logs)
    const videos = arrived.filter((event) => event.kind === 'video').map((event) => event.track)
    const views = await page.evaluate((names) => window.showViews(names), videos)
    const loudName = `${started.id}.${MediaCallsBrowserCheck.gains.length - 1}`
    const loudTracks = MediaCallsBrowserCheck.gains.map((gain, index) => `${started.id}.${index}`)
    const wanted = setup.levelsSaturate ? loudTracks : [loudName]
    const loudest = await wait(() => logs.some((event) => event[0] === 'MediaLoudest' && wanted.includes(event[1].track)))
    await page.evaluate((id) => window.media('stats', 'one', id), started.id)
    await page.waitForTimeout(2000)
    await page.evaluate((id) => window.media('stats', 'two', id), started.id)
    await wait(() => logs.some((event) => event[0] === 'MediaStats' && event[1].request === 'two'))
    const rows = logs.filter((event) => event[0] === 'MediaStats' && event[1].request === 'two').map((event) => event[1])
    await page.close()
    void pages
    return {
      offerBytes: started.offerBytes, allArrived, arrived: arrived.length,
      audio: arrived.filter((event) => event.kind === 'audio').length, video: videos.length, views, loudest, loudName, wanted,
      loudestTracks: [...new Set(logs.filter((event) => event[0] === 'MediaLoudest').map((event) => event[1].track))],
      speakingTracks: [...new Set(logs.filter((event) => event[0] === 'MediaSpeaking' && event[1].speaking).map((event) => event[1].track))].sort(),
      videoReceiving: rows.filter((row) => row.kind === 'video' && row.bitrate > 0).length,
      failures: logs.filter((event) => event[0] === 'MediaFailed'),
    }
  }

  static checkScale(name, scale, setup) {
    assert.ok(scale.allArrived, `${name}: all ${MediaCallsBrowserCheck.audioTracks + MediaCallsBrowserCheck.videoTracks} remote tracks arrive on one connection`)
    assert.equal(scale.audio, MediaCallsBrowserCheck.audioTracks, `${name}: 50 audio tracks`)
    assert.equal(scale.video, MediaCallsBrowserCheck.videoTracks, `${name}: 9 video tracks`)
    assert.equal(scale.views, MediaCallsBrowserCheck.videoTracks, `${name}: 9 media-view elements`)
    assert.deepEqual(scale.failures, [], `${name}: no media failure`)
    if (setup.levelsSaturate) console.log(`${name}: audio levels reach 1 for any loud track, so the loudest is one of ${scale.wanted}`)
    assert.ok(scale.loudest, `${name}: the loudest track is one of ${scale.wanted}`)
    assert.ok(scale.loudestTracks.every((track) => scale.wanted.includes(track)), `${name}: only talking tracks become the loudest (${scale.loudestTracks})`)
    assert.ok(scale.speakingTracks.every((track) => Number(track.split('.')[1]) < MediaCallsBrowserCheck.gains.length), `${name}: only the tracks with sound talk (${scale.speakingTracks})`)
    assert.equal(scale.videoReceiving, MediaCallsBrowserCheck.videoTracks, `${name}: all 9 videos are received`)
  }
}

await MediaCallsBrowserCheck.run()
console.log('media calls browser checks passed')

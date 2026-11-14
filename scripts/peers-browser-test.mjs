#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { createHmac, randomBytes } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class PeersBrowserCheck {
  static sources = ['tac-web.js', 'tac-peers.js'].map((name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url))
  static turnPort = 34780
  static turnSecret = randomBytes(24).toString('hex')
  static mostMessage = 32768
  static transferBytes = 10 * 1024 * 1024
  static requireToolchain = process.env.TAC_PEERS_REQUIRE_TOOLCHAINS === '1'
  static launchOptions = {
    chromium: { args: ['--disable-features=WebRtcHideLocalIpsWithMdns'] },
    firefox: { firefoxUserPrefs: { 'media.peerconnection.ice.obfuscate_host_addresses': false, 'media.peerconnection.ice.loopback': true } },
    webkit: {},
  }

  static page(settings) {
    return `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="tachyon-browser-apis" content="peers">
    <meta name="tachyon-peers" content='${JSON.stringify(settings)}'>
    <title>Peers</title></head><body>
    <script src="/tac-web.js"></script>
    <script src="/tac-peers.js"></script>
    <script>
      window.sent = []
      window.waiters = []
      TacWebApis.events = { publish: () => {}, send: (owner, name, value) => { window.sent.push([name, value]); for (const look of [...window.waiters]) look() } }
      const web = globalThis[Symbol.for('tachyon.tac.web')]
      window.ask = (call, args = []) => JSON.parse(web.calling('app', () => web.host(JSON.stringify({ api: 'peers', call, args }))))
      window.value = (call, args = []) => { const answer = window.ask(call, args); if ('error' in answer) throw new Error(answer.error); return answer.value }
      window.drop = (look) => { window.waiters = window.waiters.filter((waiter) => waiter !== look) }
      window.next = (name, test = () => true, wait = 0) => new Promise((resolve) => {
        const look = () => {
          const index = window.sent.findIndex(([event, value]) => event === name && test(value))
          if (index < 0) return false
          const [[, value]] = window.sent.splice(index, 1)
          window.drop(look)
          resolve(value)
          return true
        }
        if (look()) return
        window.waiters.push(look)
        if (wait > 0) setTimeout(() => { window.drop(look); resolve(null) }, wait)
      })
      window.encode = (bytes) => btoa(Array.from(bytes, (byte) => String.fromCharCode(byte)).join('')).replace(/\\+/g, '-').replace(/\\//g, '_').replace(/=+$/, '')
      window.decode = (text) => Uint8Array.from(atob(text.replace(/-/g, '+').replace(/_/g, '/')), (letter) => letter.charCodeAt(0))
      window.digest = async (chunks) => {
        const whole = new Uint8Array(chunks.reduce((total, chunk) => total + chunk.length, 0))
        let offset = 0
        for (const chunk of chunks) { whole.set(chunk, offset); offset += chunk.length }
        return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', whole)), (byte) => byte.toString(16).padStart(2, '0')).join('')
      }
      window.transfer = async (connection, channel, total, most) => {
        const chunks = []
        let busy = 0
        for (let offset = 0; offset < total; offset += most) {
          const chunk = new Uint8Array(Math.min(most, total - offset))
          for (let index = 0; index < chunk.length; index += 1) chunk[index] = (offset + index * 7) % 251
          chunks.push(chunk)
          while (!window.value('send', [connection, channel, window.encode(chunk)])) {
            busy += 1
            await window.next('PeerChannelReady', (value) => value.channel === channel)
          }
        }
        return { busy, digest: await window.digest(chunks) }
      }
      window.collect = async (channel, total) => {
        const chunks = []
        let size = 0
        while (size < total) {
          const message = await window.next('PeerMessage', (value) => value.channel === channel)
          const bytes = window.decode(message.data)
          chunks.push(bytes)
          size += bytes.length
        }
        return { size, digest: await window.digest(chunks) }
      }
    </script>
  </body></html>`
  }

  static async run() {
    const files = Object.fromEntries(await Promise.all(PeersBrowserCheck.sources.map(async (source) => [`/${source.pathname.split('/').pop()}`, await readFile(source, 'utf8')])))
    const turn = PeersBrowserCheck.startTurn()
    const direct = PeersBrowserCheck.page({ iceServers: [], relayOnly: false })
    const relayed = PeersBrowserCheck.page({ iceServers: [`turn:127.0.0.1:${PeersBrowserCheck.turnPort}?transport=udp`, `turn:127.0.0.1:${PeersBrowserCheck.turnPort}?transport=tcp`], relayOnly: true })
    const server = createServer((request, response) => {
      const script = files[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? (request.url === '/relay' ? relayed : direct))
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const origin = `http://localhost:${server.address().port}`
    const browsers = {}
    try {
      for (const engine of [chromium, firefox, webkit]) browsers[engine.name()] = await engine.launch(PeersBrowserCheck.launchOptions[engine.name()])
      await PeersBrowserCheck.directPairs(browsers, origin)
      await PeersBrowserCheck.relayPairs(browsers, origin, turn)
    } finally {
      for (const browser of Object.values(browsers)) await browser.close()
      server.close()
      turn?.kill('SIGTERM')
    }
  }

  static startTurn() {
    const found = spawnSync('turnserver', ['--version'], { encoding: 'utf8' })
    if (found.status === 0) return spawn('turnserver', ['-n', '--listening-ip=127.0.0.1', '--relay-ip=127.0.0.1', `--listening-port=${PeersBrowserCheck.turnPort}`, '--min-port=49400', '--max-port=49500', '--use-auth-secret', `--static-auth-secret=${PeersBrowserCheck.turnSecret}`, '--realm=tachyon.test', '--no-tls', '--allow-loopback-peers', '--no-multicast-peers', '--log-file=stdout'], { stdio: 'ignore' })
    assert.ok(!PeersBrowserCheck.requireToolchain, 'TAC_PEERS_REQUIRE_TOOLCHAINS=1 needs coturn (turnserver) on PATH')
    console.log('peers: skipping the relay checks: coturn (turnserver) is not on PATH')
    return null
  }

  static credentials() {
    const username = `${Math.floor(Date.now() / 1000) + 600}:tachyon-test`
    const credential = createHmac('sha1', PeersBrowserCheck.turnSecret).update(username).digest('base64')
    return [username, credential]
  }

  static async directPairs(browsers, origin) {
    for (const [first, second] of [['chromium', 'chromium'], ['firefox', 'firefox'], ['chromium', 'firefox']]) {
      await PeersBrowserCheck.pair(browsers[first], browsers[second], `${origin}/`, ['', ''], `${first} with ${second}, direct`)
    }
  }

  static async relayPairs(browsers, origin, turn) {
    if (!turn) return
    await new Promise((resolve) => setTimeout(resolve, 1000))
    for (const [first, second] of [['chromium', 'chromium'], ['firefox', 'firefox'], ['webkit', 'webkit'], ['chromium', 'webkit']]) {
      await PeersBrowserCheck.pair(browsers[first], browsers[second], `${origin}/relay`, PeersBrowserCheck.credentials(), `${first} with ${second}, relay only through coturn`)
    }
  }

  static async pair(firstBrowser, secondBrowser, url, credentials, title) {
    const first = await (await firstBrowser.newContext()).newPage()
    const second = await (await secondBrowser.newContext()).newPage()
    try {
      await first.goto(url)
      await second.goto(url)
      await PeersBrowserCheck.connectAndSend(first, second, credentials, title)
    } finally {
      await first.context().close()
      await second.context().close()
    }
  }

  static async connectAndSend(first, second, credentials, title) {
    assert.equal(await first.evaluate(() => window.value('supported')), true, title)
    const left = await first.evaluate((login) => window.value('open', login), credentials)
    const right = await second.evaluate((login) => window.value('open', login), credentials)
    const reliable = await first.evaluate((connection) => window.value('openChannel', [connection, 'chat', true]), left)
    const fast = await first.evaluate((connection) => window.value('openChannel', [connection, 'typing', false]), left)
    await PeersBrowserCheck.negotiate(first, left, second, right)
    const relays = PeersBrowserCheck.forwardCandidates(first, left, second, right)
    await PeersBrowserCheck.connected(first, left, title)
    await PeersBrowserCheck.connected(second, right, title)
    const opened = await second.evaluate(() => Promise.all([window.next('PeerChannelOpened'), window.next('PeerChannelOpened')]))
    assert.deepEqual(opened.map((value) => [value.label, value.ordered, value.remote]).sort(), [['chat', true, true], ['typing', false, true]], title)
    const own = await first.evaluate(() => Promise.all([window.next('PeerChannelOpened'), window.next('PeerChannelOpened')]))
    assert.deepEqual(own.map((value) => value.channel).sort(), [reliable, fast].sort(), title)
    const theirs = Object.fromEntries(opened.map((value) => [value.label, value.channel]))
    await PeersBrowserCheck.transfer(first, left, reliable, second, theirs.chat, title)
    await PeersBrowserCheck.unordered(first, left, fast, second, theirs.typing, title)
    await PeersBrowserCheck.limits(first, left, reliable, title)
    await PeersBrowserCheck.restart(first, left, second, right, reliable, theirs.chat, title)
    await second.evaluate(([connection, channel]) => window.value('closeChannel', [connection, channel]), [right, theirs.chat])
    assert.equal((await first.evaluate((channel) => window.next('PeerChannelClosed', (value) => value.channel === channel), reliable)).channel, reliable, title)
    await first.evaluate((connection) => window.value('close', [connection]), left)
    assert.equal((await first.evaluate(() => window.next('PeerState', (value) => value.state === 'closed'))).state, 'closed', title)
    relays.stop()
    console.log(`peers (${title}): connected, 10 MiB on an ordered channel with back-pressure, an unordered channel, size limits, an ICE restart and close`)
  }

  static async negotiate(first, left, second, right) {
    await first.evaluate((connection) => window.value('offer', [connection]), left)
    const offer = await first.evaluate(() => window.next('PeerDescription'))
    assert.equal(offer.kind, 'offer')
    await second.evaluate(([connection, sdp]) => window.value('answer', [connection, sdp]), [right, offer.sdp])
    const answer = await second.evaluate(() => window.next('PeerDescription'))
    assert.equal(answer.kind, 'answer')
    await first.evaluate(([connection, sdp]) => window.value('accept', [connection, sdp]), [left, answer.sdp])
  }

  static forwardCandidates(first, left, second, right) {
    const state = { running: true }
    const forward = async (from, to, connection) => {
      while (state.running) {
        const candidate = await from.evaluate(() => window.next('PeerCandidate', () => true, 200)).catch(() => null)
        if (candidate && state.running) await to.evaluate(([id, json]) => window.value('addCandidate', [id, json]), [connection, candidate.candidateJson]).catch(() => null)
      }
    }
    forward(first, second, right)
    forward(second, first, left)
    return { stop: () => { state.running = false } }
  }

  static async connected(page, connection, title) {
    const state = await page.evaluate((id) => window.next('PeerState', (value) => value.connection === id && ['connected', 'failed'].includes(value.state), 30000), connection)
    assert.equal(state?.state, 'connected', title)
  }

  static async transfer(first, left, reliable, second, theirs, title) {
    const received = second.evaluate(([channel, total]) => window.collect(channel, total), [theirs, PeersBrowserCheck.transferBytes])
    const sent = await first.evaluate(([connection, channel, total, most]) => window.transfer(connection, channel, total, most), [left, reliable, PeersBrowserCheck.transferBytes, PeersBrowserCheck.mostMessage])
    const got = await received
    assert.equal(got.size, PeersBrowserCheck.transferBytes, title)
    assert.equal(got.digest, sent.digest, `${title}: the bytes arrive whole and in order`)
    assert.ok(sent.busy > 0, `${title}: send answered false at least once and the sender waited for PeerChannelReady`)
  }

  static async unordered(first, left, fast, second, theirs, title) {
    await first.evaluate(([connection, channel]) => { for (let index = 0; index < 10; index += 1) window.value('send', [connection, channel, window.encode([index])]) }, [left, fast])
    const delivered = await second.evaluate((channel) => window.next('PeerMessage', (value) => value.channel === channel, 10000), theirs)
    assert.ok(delivered, `${title}: the unordered channel delivers`)
  }

  static async limits(first, left, reliable, title) {
    const answer = await first.evaluate(([connection, channel, size]) => window.ask('send', [connection, channel, window.encode(new Uint8Array(size))]), [left, reliable, PeersBrowserCheck.mostMessage + 1])
    assert.deepEqual(answer, { error: 'A peer message is at most 32768 bytes.' }, title)
  }

  static async restart(first, left, second, right, reliable, theirs, title) {
    await first.evaluate((connection) => window.value('restart', [connection]), left)
    const offer = await first.evaluate(() => window.next('PeerDescription'))
    await second.evaluate(([connection, sdp]) => window.value('answer', [connection, sdp]), [right, offer.sdp])
    const answer = await second.evaluate(() => window.next('PeerDescription'))
    await first.evaluate(([connection, sdp]) => window.value('accept', [connection, sdp]), [left, answer.sdp])
    await first.evaluate(([connection, channel]) => window.value('send', [connection, channel, window.encode([42])]), [left, reliable])
    const message = await second.evaluate((channel) => window.next('PeerMessage', (value) => value.channel === channel), theirs)
    assert.equal(message.data, 'Kg', `${title}: the channel still works after an ICE restart`)
  }
}

await PeersBrowserCheck.run()
console.log('peers: all pairs passed')

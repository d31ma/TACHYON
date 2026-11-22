#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawn, spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { createServer as createNetServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { chromium, firefox, webkit } from 'playwright'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { withServer } from './release/server-probe.mjs'

class PeersLargeCallCheck {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static audioTracks = 50
  static videoTracks = 9
  static descriptionLimit = 262144
  static eventLimit = 65536
  static waitMs = 120000
  static buildTimeout = 900000
  static turnUser = 'probe'
  static turnSecret = 'probe-relay-secret'
  static transports = ['socket', 'http']
  static engines = [
    { engine: chromium, options: { args: ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream', '--autoplay-policy=no-user-gesture-required'] } },
    { engine: firefox, options: { firefoxUserPrefs: { 'media.navigator.streams.fake': true, 'media.navigator.permission.disabled': true, 'media.autoplay.default': 0, 'media.peerconnection.ice.loopback': true } } },
    { engine: webkit, options: {} },
  ]

  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createNetServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address()
        server.close(() => resolve(port))
      })
    })
  }

  static startTurn(port) {
    const found = spawnSync('turnserver', ['--version'], { encoding: 'utf8' })
    assert.equal(found.status, 0, 'coturn (turnserver) is required on PATH: WebKit hides host candidates, so every engine connects through a local relay')
    return spawn('turnserver', ['-n', '--listening-ip=127.0.0.1', '--relay-ip=127.0.0.1', `--listening-port=${port}`, '--min-port=49600', '--max-port=49700', '--lt-cred-mech', `--user=${PeersLargeCallCheck.turnUser}:${PeersLargeCallCheck.turnSecret}`, '--realm=tachyon.test', '--no-tls', '--allow-loopback-peers', '--no-multicast-peers'], { stdio: 'ignore' })
  }

  static async startRelay() {
    const signals = []
    const server = createServer((request, response) => PeersLargeCallCheck.relayRequest(signals, request, response))
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    return server
  }

  static relayRequest(signals, request, response) {
    const headers = { 'access-control-allow-origin': request.headers.origin ?? '*', 'access-control-allow-headers': request.headers['access-control-request-headers'] ?? 'content-type', 'access-control-allow-methods': 'POST, OPTIONS', 'content-type': 'application/json' }
    if (request.method === 'OPTIONS') return PeersLargeCallCheck.answer(response, 204, headers, '')
    const chunks = []
    request.on('data', (chunk) => chunks.push(chunk))
    request.on('end', () => {
      const body = JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}')
      if (request.url === '/reset') signals.length = 0
      if (request.url === '/post') signals.push(body)
      const items = request.url === '/take' ? signals.slice(body.after).filter((signal) => signal.from !== body.from) : []
      PeersLargeCallCheck.answer(response, 200, headers, JSON.stringify({ items, next: signals.length, count: signals.length }))
    })
  }

  static answer(response, status, headers, text) {
    response.writeHead(status, headers)
    response.end(text)
  }

  static manifest(turnPort, relayPort) {
    return {
      name: 'Large Call',
      start_url: '/',
      tac: {
        browserApis: ['microphone', 'camera', 'peers'],
        peers: { iceServers: [`turn:127.0.0.1:${turnPort}?transport=udp`], relayOnly: true },
        yon: { origins: { relay: `http://127.0.0.1:${relayPort}` } },
        visibility: { check: false },
        duplication: { check: false },
      },
    }
  }

  static files(turnPort, relayPort) {
    return {
      'client/manifest.json': `${JSON.stringify(PeersLargeCallCheck.manifest(turnPort, relayPort))}\n`,
      'client/pages/tac.html': '<call-room hydrate="load" />\n',
      'client/components/call/room/tac.html': '<main aria-label="Call"><button on:click="call.answer(\'socket\')">Answer over socket</button><button on:click="call.answer(\'http\')">Answer over Http</button><p>{call.report}</p></main>\n',
      'client/models/Signal.ts': 'export type Signal = { from: string; kind: string; text: string }\n',
      'client/models/TakeRequest.ts': 'export type TakeRequest = { after: number; from: string }\n',
      'client/models/Taken.ts': "import type { Signal } from './Signal.ts'\nexport type Taken = { items: Signal[]; next: number }\n",
      'server/models/Signal.ts': 'export interface Signal {\n  from: string\n  kind: string\n  text: string\n}\n',
      'server/controllers/SignalController.js': PeersLargeCallCheck.controller(),
      'client/remotes/SignalClient.js': "/** @import { Signal } from '/models/Signal.ts' */\n@HTTPClient('/signal')\nexport default class SignalClient {\n  /** @param {Signal} signal */\n  @SocketRequest\n  receive(signal) {}\n}\n",
      'client/remotes/RelayClient.js': PeersLargeCallCheck.relayClient(),
      'client/services/CallService.js': PeersLargeCallCheck.service(),
    }
  }

  static controller() {
    return `/** @import { Signal } from '/models/Signal.ts' */
@SocketRoute('/signal')
@Public
class SignalController {
  /**
   * @param {SocketSession} session
   * @param {Signal} signal
   */
  @OnMessage
  async onMessage(session, signal) {
    await this._sendToAll(await session.openSessions(), signal);
  }

  /**
   * @param {SocketSession[]} sessions
   * @param {Signal} signal
   */
  async _sendToAll(sessions, signal) {
    for (const peer of sessions) await peer.send(signal);
  }
}
`
  }

  static relayClient() {
    return `/** @import { Signal } from '/models/Signal.ts' */
/** @import { TakeRequest } from '/models/TakeRequest.ts' */
/** @import { Taken } from '/models/Taken.ts' */
@HTTPClient({"origin":"relay"})
export default class RelayClient {
  /**
   * @param {Signal} signal
   * @returns {Promise<{ count: number } | void>}
   */
  @PostRequest("/post")
  async post(@RequestBody signal) {}

  /**
   * @param {TakeRequest} request
   * @returns {Promise<Taken | void>}
   */
  @PostRequest("/take")
  async take(@RequestBody request) {}
}
`
  }

  static service() {
    const { turnUser, turnSecret } = PeersLargeCallCheck
    return `/** @import { Signal } from '/models/Signal.ts' */
@Service
export default class CallService {
  static _POLL_MS = 300;
  static _USER = ${JSON.stringify(turnUser)};
  static _SECRET = ${JSON.stringify(turnSecret)};
  static _ROLE = 'answerer';
  static _SOCKET = 'socket';
  static _HTTP = 'http';
  static _SPACE = ' ';
  static _STEPS = {
    offer: (peers, connection, text) => peers.answer(connection, text),
    candidate: (peers, connection, text) => peers.addCandidate(connection, text),
  };
  /** @type {TacPeers} */
  @Needs
  peers;
  /** @type {TacMedia} */
  @Needs
  media;
  /** @type {SignalClient & { send: SocketSend<Signal> }} */
  @Needs
  signalClient;
  /** @type {RelayClient} */
  @Needs
  relayClient;
  @Needs
  taskScheduler;
  report = 'idle';
  _transport = '';
  _connection = '';
  _arrived = 0;
  _answerBytes = 0;
  _taken = 0;

  /**
   * @param {Event} _event
   * @param {string} transport
   */
  answer(_event, transport) {
    this._transport = transport;
    this.media.resumeAudio();
    this._connection = this.peers.open(CallService._USER, CallService._SECRET);
    this._show();
    this._startPolling();
  }

  _startPolling() {
    if (this._transport !== CallService._HTTP) return;
    void this._poll();
  }

  async _poll() {
    const taken = await this.relayClient.take({ after: this._taken, from: CallService._ROLE });
    this._taken = taken.next;
    for (const signal of taken.items) this._step(signal);
    setTimeout(() => void this._poll(), CallService._POLL_MS);
  }

  /** @param {Signal} signal */
  @OnEvent
  hearSignal(signal) {
    this._stepFromSocket(signal);
  }

  /** @param {Signal} signal */
  _stepFromSocket(signal) {
    if (this._transport !== CallService._SOCKET || signal.from === CallService._ROLE) return;
    this._step(signal);
  }

  /** @param {Signal} signal */
  _step(signal) {
    CallService._STEPS[signal.kind](this.peers, this._connection, signal.text);
  }

  /** @param {PeerDescription} described */
  @OnEvent
  hearDescription(described) {
    void this._relay(described.kind, described.sdp);
    this._answerBytes = new TextEncoder().encode(described.sdp).length;
    this._show();
  }

  /** @param {PeerCandidate} candidate */
  @OnEvent
  hearCandidate(candidate) {
    void this._relay('candidate', candidate.candidateJson);
  }

  /**
   * @param {string} kind
   * @param {string} text
   */
  async _relay(kind, text) {
    const signal = { from: CallService._ROLE, kind, text };
    if (this._transport === CallService._SOCKET) return this.signalClient.send(signal);
    return this.relayClient.post(signal);
  }

  /** @param {MediaTrackArrived} arrived */
  @OnEvent
  hearArrived(arrived) {
    this._arrived += 1;
    this._show();
  }

  /** @param {PeerFailed} failed */
  @OnEvent
  hearFailed(failed) {
    this._showProblem('failed ' + failed.call + CallService._SPACE + failed.reason + CallService._SPACE + failed.message);
  }

  /** @param {MediaFailed} failed */
  @OnEvent
  hearMediaFailed(failed) {
    this._showProblem('media failed ' + JSON.stringify(failed));
  }

  _show() {
    this._showProblem('arrived ' + this._arrived + ' tracks, answered ' + this._answerBytes + ' bytes');
  }

  /** @param {string} text */
  _showProblem(text) {
    this.report = CallService._ROLE + CallService._SPACE + this._transport + ': ' + text;
    void this.taskScheduler.render();
  }
}
`
  }

  static stage(project, turnPort, relayPort) {
    for (const [name, text] of Object.entries(PeersLargeCallCheck.files(turnPort, relayPort))) {
      const file = path.join(project, name)
      mkdirSync(path.dirname(file), { recursive: true })
      writeFileSync(file, text)
    }
  }

  static build(project) {
    const built = spawnSync(PeersLargeCallCheck.binary, ['build', project], { encoding: 'utf8', timeout: PeersLargeCallCheck.buildTimeout, killSignal: 'SIGKILL', maxBuffer: 16 * 1024 * 1024 })
    assert.equal(built.status, 0, `the large-call app did not build: ${built.error?.message || `${built.stdout}${built.stderr}`}`)
  }

  static async report(page) {
    return (await page.locator('main[aria-label="Call"] p').textContent()) ?? ''
  }

  static async waitFor(page, done) {
    const deadline = Date.now() + PeersLargeCallCheck.waitMs
    let text = ''
    while (Date.now() < deadline) {
      text = await PeersLargeCallCheck.report(page)
      if (done(text) || text.includes('failed')) return text
      await page.waitForTimeout(500)
    }
    return text
  }

  static async call(context, origin, ports, transport) {
    await fetch(`http://127.0.0.1:${ports.relay}/reset`, { method: 'POST', body: '{}' })
    const [server, answerer] = [await context.newPage(), await context.newPage()]
    const errors = []
    const sockets = []
    for (const page of [server, answerer]) page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
    answerer.on('websocket', (socket) => sockets.push(socket.url()))
    await Promise.all([server.goto(origin), answerer.goto(origin)])
    const label = transport === 'socket' ? 'socket' : 'Http'
    await answerer.getByRole('button', { name: `Answer over ${label}` }).click()
    await PeersLargeCallCheck.waitFor(answerer, (text) => text.startsWith('answerer'))
    await server.evaluate(PeersLargeCallCheck.mediaServerStandIn, PeersLargeCallCheck.standInSetup(transport, sockets, ports))
    const total = PeersLargeCallCheck.audioTracks + PeersLargeCallCheck.videoTracks
    const arrived = await PeersLargeCallCheck.waitFor(answerer, (text) => new RegExp(`arrived ${total} tracks, answered [1-9]`).test(text))
    const standIn = await PeersLargeCallCheck.waitForStandIn(server)
    await Promise.all([server.close(), answerer.close()])
    return { arrived, standIn, errors: errors.slice(0, 5) }
  }

  static standInSetup(transport, sockets, ports) {
    return {
      transport,
      socketUrl: sockets.find((url) => url.endsWith('/signal')) ?? '',
      relayUrl: `http://127.0.0.1:${ports.relay}`,
      turnUrl: `turn:127.0.0.1:${ports.turn}`,
      username: PeersLargeCallCheck.turnUser,
      credential: PeersLargeCallCheck.turnSecret,
      audio: PeersLargeCallCheck.audioTracks,
      video: PeersLargeCallCheck.videoTracks,
    }
  }

  static async mediaServerStandIn(setup) {
    const name = 'media-server'
    const peer = new RTCPeerConnection({ iceServers: [{ urls: setup.turnUrl, username: setup.username, credential: setup.credential }], iceTransportPolicy: 'relay', bundlePolicy: 'max-bundle' })
    const standIn = { state: 'new', offerBytes: 0, errors: [] }
    window.mediaServerStandIn = standIn
    peer.addEventListener('connectionstatechange', () => { standIn.state = peer.connectionState })
    const audioContext = new AudioContext()
    const tone = audioContext.createOscillator()
    const toneOut = audioContext.createMediaStreamDestination()
    tone.connect(toneOut)
    tone.start()
    const canvas = Object.assign(document.createElement('canvas'), { width: 160, height: 120 })
    const paint = canvas.getContext('2d')
    setInterval(() => { paint.fillStyle = `hsl(${Math.floor(Date.now() / 10) % 360} 60% 50%)`; paint.fillRect(0, 0, 160, 120) }, 50)
    const [audioSource, videoSource] = [toneOut.stream.getAudioTracks()[0], canvas.captureStream(15).getVideoTracks()[0]]
    for (let index = 0; index < setup.audio; index += 1) peer.addTrack(audioSource.clone(), new MediaStream())
    for (let index = 0; index < setup.video; index += 1) peer.addTrack(videoSource.clone(), new MediaStream())
    let socket = null
    const send = (kind, text) => {
      const signal = JSON.stringify({ from: name, kind, text })
      if (socket) return socket.send(signal)
      return fetch(`${setup.relayUrl}/post`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: signal })
    }
    const pending = []
    const apply = async (signal) => {
      if (signal.from === name) return
      if (signal.kind === 'answer') {
        await peer.setRemoteDescription({ type: 'answer', sdp: signal.text })
        for (const candidate of pending.splice(0)) await peer.addIceCandidate(candidate)
      }
      if (signal.kind === 'candidate' && signal.text !== '') {
        const candidate = JSON.parse(signal.text)
        if (peer.remoteDescription) await peer.addIceCandidate(candidate)
        else pending.push(candidate)
      }
    }
    let queue = Promise.resolve()
    const heard = (signal) => { queue = queue.then(() => apply(signal)).catch((error) => standIn.errors.push(String(error?.message ?? error))) }
    if (setup.transport === 'socket') {
      socket = new WebSocket(setup.socketUrl)
      await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = () => reject(new Error(`the stand-in could not open ${setup.socketUrl}`)) })
      socket.onmessage = (event) => heard(JSON.parse(event.data))
    } else {
      let after = 0
      const poll = async () => {
        const response = await fetch(`${setup.relayUrl}/take`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ after, from: name }) })
        const taken = await response.json()
        after = taken.next
        for (const signal of taken.items) heard(signal)
        setTimeout(poll, 300)
      }
      void poll()
    }
    peer.addEventListener('icecandidate', (event) => { if (event.candidate) void send('candidate', JSON.stringify(event.candidate.toJSON())) })
    await peer.setLocalDescription(await peer.createOffer())
    standIn.offerBytes = new TextEncoder().encode(peer.localDescription.sdp).length
    await send('offer', peer.localDescription.sdp)
  }

  static async waitForStandIn(page) {
    const deadline = Date.now() + PeersLargeCallCheck.waitMs
    let standIn = {}
    while (Date.now() < deadline) {
      standIn = await page.evaluate(() => ({ ...window.mediaServerStandIn }))
      if (['connected', 'failed'].includes(standIn.state)) return standIn
      await page.waitForTimeout(500)
    }
    return standIn
  }

  static answerBytes(text) {
    return Number(/answered (\d+) bytes/.exec(text)?.[1] ?? 0)
  }

  static check(name, transport, result) {
    const total = PeersLargeCallCheck.audioTracks + PeersLargeCallCheck.videoTracks
    const limit = PeersLargeCallCheck.descriptionLimit
    assert.match(result.arrived, new RegExp(`^answerer ${transport}: arrived ${total} tracks, answered \\d+ bytes$`), `${name} ${transport}: the answering Tac page receives all ${total} tracks (${JSON.stringify(result)})`)
    assert.equal(result.standIn.state, 'connected', `${name} ${transport}: the media server stand-in connects (${JSON.stringify(result.standIn)})`)
    assert.deepEqual(result.standIn.errors, [], `${name} ${transport}: the stand-in applied every answer and candidate`)
    const answer = PeersLargeCallCheck.answerBytes(result.arrived)
    assert.ok(result.standIn.offerBytes <= limit, `${name}: the ${total}-track offer fits the incoming description limit (${result.standIn.offerBytes} bytes)`)
    assert.ok(answer > 0 && answer <= limit, `${name}: the Tac page's ${total}-track answer passed the outgoing description limit (${answer} bytes)`)
    if (name !== 'firefox') assert.ok(result.standIn.offerBytes > PeersLargeCallCheck.eventLimit, `${name}: a ${total}-track offer is larger than the old 64 KiB limit, so the raised limits carried it (${result.standIn.offerBytes} bytes)`)
  }

  static async runEngine(setup, origin, ports) {
    const name = setup.engine.name()
    const browser = await setup.engine.launch(setup.options)
    try {
      const context = await browser.newContext()
      await context.grantPermissions(['microphone', 'camera']).catch(() => null)
      for (const transport of PeersLargeCallCheck.transports) {
        const result = await PeersLargeCallCheck.call(context, origin, ports, transport)
        console.log(name, transport, JSON.stringify(result))
        PeersLargeCallCheck.check(name, transport, result)
        console.log(`ok - ${name} ${transport}: ${PeersLargeCallCheck.audioTracks + PeersLargeCallCheck.videoTracks} tracks arrived after a ${result.standIn.offerBytes}-byte offer and a ${PeersLargeCallCheck.answerBytes(result.arrived)}-byte answer went through Peers.answer and addCandidate over ${transport}`)
      }
    } finally {
      await browser.close()
    }
  }

  static async main() {
    const turnPort = await PeersLargeCallCheck.freePort()
    const turn = PeersLargeCallCheck.startTurn(turnPort)
    const relay = await PeersLargeCallCheck.startRelay()
    const project = mkdtempSync(path.join(tmpdir(), 'tachyon-peers-large-call-'))
    try {
      PeersLargeCallCheck.stage(project, turnPort, relay.address().port)
      PeersLargeCallCheck.build(project)
      const serve = ['serve', project, '--host', '127.0.0.1', '--port', '0', '--out-dir', 'dist', '--no-watch', '--no-self-restart', '--no-bundle']
      await withServer(PeersLargeCallCheck.binary, serve, async (request) => {
        const response = await request('/')
        assert.equal(response.status, 200)
        const origin = new URL(response.url).origin
        const ports = { relay: relay.address().port, turn: turnPort }
        for (const setup of PeersLargeCallCheck.engines) await PeersLargeCallCheck.runEngine(setup, origin, ports)
      }, { readinessTimeoutMilliseconds: 120000 })
    } finally {
      relay.close()
      turn.kill('SIGTERM')
      rmSync(project, { recursive: true, force: true })
    }
  }
}

await PeersLargeCallCheck.main()

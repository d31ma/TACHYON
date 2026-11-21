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
      'client/components/call/room/tac.html': '<main aria-label="Call"><button on:click="call.start(\'caller\', \'socket\')">Call over socket</button><button on:click="call.start(\'answerer\', \'socket\')">Answer over socket</button><button on:click="call.start(\'caller\', \'http\')">Call over Http</button><button on:click="call.start(\'answerer\', \'http\')">Answer over Http</button><p>{call.report}</p></main>\n',
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
    const { audioTracks, videoTracks, turnUser, turnSecret } = PeersLargeCallCheck
    return `/** @import { Signal } from '/models/Signal.ts' */
@Service
export default class CallService {
  static _AUDIO = ${audioTracks};
  static _VIDEO = ${videoTracks};
  static _TRACKS = ${audioTracks + videoTracks};
  static _POLL_MS = 300;
  static _USER = ${JSON.stringify(turnUser)};
  static _SECRET = ${JSON.stringify(turnSecret)};
  static _CALLER = 'caller';
  static _SOCKET = 'socket';
  static _HTTP = 'http';
  static _OFFER = 'offer';
  static _CANDIDATE = 'candidate';
  static _CONNECTED = 'connected';
  static _MICROPHONE = 'mic';
  static _CAMERA = 'cam';
  static _STREAM = 'call';
  static _SPACE = ' ';
  static _BYTES = ' bytes';
  static _STEPS = {
    offer: (peers, connection, text) => peers.answer(connection, text),
    answer: (peers, connection, text) => peers.accept(connection, text),
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
  _role = '';
  _transport = '';
  _connection = '';
  _started = 0;
  _arrived = 0;
  _offerBytes = 0;
  _taken = 0;

  /**
   * @param {string} role
   * @param {string} transport
   */
  start(role, transport) {
    this._role = role;
    this._transport = transport;
    this._connection = this.peers.open(CallService._USER, CallService._SECRET);
    this._show('waiting');
    this._startPolling();
    this._startCapture();
  }

  _startPolling() {
    if (this._transport !== CallService._HTTP) return;
    void this._poll();
  }

  async _poll() {
    const taken = await this.relayClient.take({ after: this._taken, from: this._role });
    this._taken = taken.next;
    for (const signal of taken.items) this._step(signal);
    setTimeout(() => void this._poll(), CallService._POLL_MS);
  }

  _startCapture() {
    if (this._role !== CallService._CALLER) return;
    this._captureMicrophones();
    this._captureCameras();
  }

  _captureMicrophones() {
    for (let index = 0; index < CallService._AUDIO; index += 1) this.media.captureMicrophone(CallService._MICROPHONE + index, '', false);
  }

  _captureCameras() {
    for (let index = 0; index < CallService._VIDEO; index += 1) this.media.captureCamera(CallService._CAMERA + index, '', 'low');
  }

  /** @param {MediaTrackStarted} started */
  @OnEvent
  hearStarted(started) {
    this._started += 1;
    this._sendWhenAllStarted();
  }

  _sendWhenAllStarted() {
    if (this._started !== CallService._TRACKS) return;
    this._sendMicrophones();
    this._sendCameras();
    this.peers.offer(this._connection);
  }

  _sendMicrophones() {
    for (let index = 0; index < CallService._AUDIO; index += 1) this.media.send(this._connection, CallService._MICROPHONE + index, CallService._STREAM, false);
  }

  _sendCameras() {
    for (let index = 0; index < CallService._VIDEO; index += 1) this.media.send(this._connection, CallService._CAMERA + index, CallService._STREAM, false);
  }

  /** @param {PeerDescription} described */
  @OnEvent
  hearDescription(described) {
    void this._relay(described.kind, described.sdp);
    this._noteOffer(described);
  }

  /** @param {PeerDescription} described */
  _noteOffer(described) {
    if (described.kind !== CallService._OFFER) return;
    this._offerBytes = new TextEncoder().encode(described.sdp).length;
    this._show('offered ' + this._offerBytes + CallService._BYTES);
  }

  /** @param {PeerCandidate} candidate */
  @OnEvent
  hearCandidate(candidate) {
    void this._relay(CallService._CANDIDATE, candidate.candidateJson);
  }

  /** @param {Signal} signal */
  @OnEvent
  hearSignal(signal) {
    this._stepFromSocket(signal);
  }

  /** @param {Signal} signal */
  _stepFromSocket(signal) {
    if (this._transport !== CallService._SOCKET || signal.from === this._role) return;
    this._step(signal);
  }

  /** @param {Signal} signal */
  _step(signal) {
    CallService._STEPS[signal.kind](this.peers, this._connection, signal.text);
  }

  /**
   * @param {string} kind
   * @param {string} text
   */
  async _relay(kind, text) {
    const signal = { from: this._role, kind, text };
    if (this._transport === CallService._SOCKET) return this.signalClient.send(signal);
    return this.relayClient.post(signal);
  }

  /** @param {MediaTrackArrived} arrived */
  @OnEvent
  hearArrived(arrived) {
    this._arrived += 1;
    this._showArrived();
  }

  _showArrived() {
    if (this._arrived !== CallService._TRACKS) return;
    this._show('arrived ' + this._arrived + ' tracks');
  }

  /** @param {PeerState} state */
  @OnEvent
  hearState(state) {
    this._showConnected(state);
  }

  /** @param {PeerState} state */
  _showConnected(state) {
    if (this._role !== CallService._CALLER || state.state !== CallService._CONNECTED) return;
    this._show('connected, offered ' + this._offerBytes + CallService._BYTES);
  }

  /** @param {PeerFailed} failed */
  @OnEvent
  hearFailed(failed) {
    this._show('failed ' + failed.call + CallService._SPACE + failed.reason + CallService._SPACE + failed.message);
  }

  /** @param {MediaFailed} failed */
  @OnEvent
  hearMediaFailed(failed) {
    this._show('media failed ' + JSON.stringify(failed));
  }

  /** @param {string} text */
  _show(text) {
    this.report = this._role + CallService._SPACE + this._transport + ': ' + text;
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

  static async call(context, origin, relayPort, transport) {
    await fetch(`http://127.0.0.1:${relayPort}/reset`, { method: 'POST', body: '{}' })
    const [caller, answerer] = [await context.newPage(), await context.newPage()]
    const errors = []
    for (const page of [caller, answerer]) page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
    await Promise.all([caller.goto(origin), answerer.goto(origin)])
    const label = transport === 'socket' ? 'socket' : 'Http'
    await answerer.getByRole('button', { name: `Answer over ${label}` }).click()
    await caller.getByRole('button', { name: `Call over ${label}` }).click()
    const arrived = await PeersLargeCallCheck.waitFor(answerer, (text) => text.includes('arrived'))
    const offered = await PeersLargeCallCheck.waitFor(caller, (text) => text.includes('connected'))
    await Promise.all([caller.close(), answerer.close()])
    return { arrived, offered, errors: errors.slice(0, 5) }
  }

  static offerBytes(text) {
    return Number(/offered (\d+) bytes/.exec(text)?.[1] ?? 0)
  }

  static check(name, transport, result) {
    const total = PeersLargeCallCheck.audioTracks + PeersLargeCallCheck.videoTracks
    assert.equal(result.arrived, `answerer ${transport}: arrived ${total} tracks`, `${name} ${transport}: the answering page receives all ${total} tracks (${JSON.stringify(result)})`)
    assert.match(result.offered, /^caller [a-z]+: connected, offered \d+ bytes$/, `${name} ${transport}: the calling page connects (${JSON.stringify(result)})`)
    const bytes = PeersLargeCallCheck.offerBytes(result.offered)
    assert.ok(bytes <= PeersLargeCallCheck.descriptionLimit, `${name}: the offer fits the description limit`)
    if (name !== 'firefox') assert.ok(bytes > PeersLargeCallCheck.eventLimit, `${name}: a ${total}-track offer is larger than the old 64 KiB limit, so the raised limits carried it (${bytes} bytes)`)
  }

  static async runEngine(setup, origin, relayPort) {
    const name = setup.engine.name()
    const browser = await setup.engine.launch(setup.options)
    try {
      const context = await browser.newContext()
      await context.grantPermissions(['microphone', 'camera']).catch(() => null)
      for (const transport of PeersLargeCallCheck.transports) {
        const result = await PeersLargeCallCheck.call(context, origin, relayPort, transport)
        console.log(name, transport, JSON.stringify(result))
        PeersLargeCallCheck.check(name, transport, result)
        console.log(`ok - ${name} ${transport}: ${PeersLargeCallCheck.audioTracks + PeersLargeCallCheck.videoTracks} tracks arrived after a ${PeersLargeCallCheck.offerBytes(result.offered)}-byte offer went through Peers.answer, accept and addCandidate over ${transport}`)
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
        for (const setup of PeersLargeCallCheck.engines) await PeersLargeCallCheck.runEngine(setup, origin, relay.address().port)
      }, { readinessTimeoutMilliseconds: 120000 })
    } finally {
      relay.close()
      turn.kill('SIGTERM')
      rmSync(project, { recursive: true, force: true })
    }
  }
}

await PeersLargeCallCheck.main()

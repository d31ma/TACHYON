import { PeersNativeProbe } from './peers-native-probe.mjs'

export class SframeCallProbe {
  static PREFIX = 'Call: '
  static CHECKING = 'checking'
  static LABEL = 'Call'
  static KEY_ID = 7
  static KEY_NAME = 'call-key'
  static SAME_SECRET = SframeCallProbe.secret(1)
  static WRONG_SECRET = SframeCallProbe.secret(101)
  static ENOUGH_FRAMES = 30
  static WATCH_MS = 20000
  static WAIT_MS = 90000
  static POLL_MS = 250
  static MICROPHONE_REASON = 'The call test sends a drawn test pattern and never opens the microphone.'

  /** @param {number} first */
  static secret(first) {
    const bytes = Array.from({ length: 32 }, (_, index) => (first + index) % 256)
    return Buffer.from(bytes).toString('base64url')
  }

  /** @param {{ id: string, name: string, turnPort: number, mailbox: string }} settings */
  static manifest(settings) {
    const relay = PeersNativeProbe.relayed(settings.turnPort)
    const manifest = PeersNativeProbe.manifest({ id: settings.id, name: settings.name, iceServers: relay.iceServers, relayOnly: relay.relayOnly, permissions: { microphone: SframeCallProbe.MICROPHONE_REASON } })
    manifest.tac.browserApis = ['peers', 'crypto', 'microphone']
    manifest.tac.yon = { origin: settings.mailbox }
    return manifest
  }

  /** @param {{ id: string, name: string, turnPort: number, mailbox: string }} settings */
  static files(settings) {
    return {
      'client/manifest.json': JSON.stringify(SframeCallProbe.manifest(settings)),
      'client/pages/tac.html': '<sframe-call hydrate="load" />\n',
      'client/components/sframe/call/tac.html': `<main aria-label="${SframeCallProbe.LABEL}"><p>{sframeCall.report}</p></main>\n`,
      'client/shared/sframe-call.js': SframeCallProbe.pageModule(),
      'client/services/SframeCallService.js': SframeCallProbe.service(),
    }
  }

  static service() {
    return `import { SframeCall } from '/shared/sframe-call.js';
@Service()
export default class SframeCallService {
  static _PREFIX = ${JSON.stringify(SframeCallProbe.PREFIX)};
  static _USERNAME = ${JSON.stringify(PeersNativeProbe.TURN_USER)};
  static _CREDENTIAL = ${JSON.stringify(PeersNativeProbe.TURN_SECRET)};
  /** @type {TacPeers} */
  @Needs
  peers;
  /** @type {TacMedia} */
  @Needs
  media;
  /** @type {TacCrypto} */
  @Needs
  crypto;
  @Needs
  taskScheduler;
  report = SframeCallService._PREFIX + ${JSON.stringify(SframeCallProbe.CHECKING)};
  _call = null;

  @OnStart()
  initialize() {
    this._call = new SframeCall({ peers: this.peers, media: this.media, crypto: this.crypto }, (text) => this._show(text));
    this._call.start(SframeCallService._USERNAME, SframeCallService._CREDENTIAL);
  }

  /** @param {CryptoKeyMade} made */
  @OnEvent
  hearKeyMade(made) {
    this._call.keyMade(made);
  }

  /** @param {CryptoFailed} failed */
  @OnEvent
  hearCryptoFailed(failed) {
    this._call.failed('crypto ' + failed.call + ' ' + failed.reason);
  }

  /** @param {MediaFrameKeyReady} ready */
  @OnEvent
  hearKeyReady(ready) {
    this._call.keyReady(ready);
  }

  /** @param {MediaFrameKeyMissing} missing */
  @OnEvent
  hearKeyMissing(missing) {
    this._call.note('MediaFrameKeyMissing');
  }

  /** @param {MediaFrameRejected} rejected */
  @OnEvent
  hearRejected(rejected) {
    this._call.note('MediaFrameRejected');
  }

  /** @param {MediaFailed} failed */
  @OnEvent
  hearMediaFailed(failed) {
    this._call.failed('media ' + failed.call + ' ' + failed.reason);
  }

  /** @param {PeerDescription} described */
  @OnEvent
  hearDescription(described) {
    this._call.described(described);
  }

  /** @param {PeerCandidate} candidate */
  @OnEvent
  hearCandidate(candidate) {
    this._call.candidate(candidate);
  }

  /** @param {PeerState} state */
  @OnEvent
  hearState(state) {
    this._call.state(state);
  }

  /** @param {PeerFailed} failed */
  @OnEvent
  hearPeerFailed(failed) {
    this._call.failed('peers ' + failed.call + ' ' + failed.reason);
  }

  /** @param {string} text */
  _show(text) {
    this.report = SframeCallService._PREFIX + text;
    void this.taskScheduler.render();
  }
}
`
  }

  static pageModule() {
    return `export class SframeCall {
  static KEY_ID = ${SframeCallProbe.KEY_ID};
  static KEY_NAME = ${JSON.stringify(SframeCallProbe.KEY_NAME)};
  static SECRET = ${JSON.stringify(SframeCallProbe.SAME_SECRET)};
  static ENOUGH = ${SframeCallProbe.ENOUGH_FRAMES};
  static WATCH = ${SframeCallProbe.WATCH_MS};
  static WAIT = ${SframeCallProbe.WAIT_MS};
  static POLL = ${SframeCallProbe.POLL_MS};
  static SECOND = 1000;

  constructor(services, show) {
    this.peers = services.peers;
    this.media = services.media;
    this.crypto = services.crypto;
    this.show = show;
    this.origin = document.querySelector('meta[name="tachyon-yon-origin"]').content;
    this.connection = '';
    this.offered = false;
    this.watching = false;
    this.done = false;
    this.states = [];
    this.events = new Set();
  }

  start(username, credential) {
    this.username = username;
    this.credential = credential;
    setTimeout(() => this.finish('timeout after ' + (this.states.join(' ') || 'no state change')), SframeCall.WAIT);
    try {
      this.crypto.importKey(SframeCall.KEY_NAME, SframeCall.SECRET, 'derive', false);
    } catch (error) {
      this.finish('error ' + String(error && error.message || error));
    }
  }

  keyMade(made) {
    if (made.key !== SframeCall.KEY_NAME || this.connection) return;
    this.connection = this.peers.open(this.username, this.credential);
    const stream = SframeCall.pattern();
    SframeCall.raw(this.connection).addTrack(stream.getVideoTracks()[0], stream);
    this.media.encryptFrames(this.connection, true);
    this.media.useFrameKey(SframeCall.KEY_ID, SframeCall.KEY_NAME);
  }

  keyReady(ready) {
    this.note('MediaFrameKeyReady');
    if (this.offered) return;
    this.offered = true;
    this.media.sendFrameKey(ready.keyId);
    this.peers.offer(this.connection);
    void this.listen();
  }

  described(description) {
    if (description.connection !== this.connection || description.kind !== 'offer') return;
    void this.post({ type: 'offer', sdp: description.sdp });
  }

  candidate(candidate) {
    if (candidate.connection !== this.connection || !candidate.candidateJson) return;
    void this.post({ type: 'candidate', candidateJson: candidate.candidateJson });
  }

  state(state) {
    this.states.push(state.state);
    if (state.state === 'connected') void this.watch();
  }

  note(name) {
    this.events.add(name);
  }

  failed(text) {
    this.finish('failed ' + text);
  }

  async listen() {
    while (!this.done) {
      const answer = await fetch(this.origin + '/to-app');
      for (const message of await answer.json()) this.take(message);
      await SframeCall.wait(SframeCall.POLL);
    }
  }

  take(message) {
    if (message.type === 'answer') this.peers.accept(this.connection, message.sdp);
    if (message.type === 'candidate') this.peers.addCandidate(this.connection, message.candidateJson);
  }

  post(message) {
    return fetch(this.origin + '/to-browser', { method: 'POST', body: JSON.stringify(message) });
  }

  async watch() {
    if (this.watching) return;
    this.watching = true;
    const end = Date.now() + SframeCall.WATCH;
    let frames = 0;
    while (Date.now() < end && frames < SframeCall.ENOUGH) {
      await SframeCall.wait(SframeCall.SECOND);
      frames = await SframeCall.decoded(SframeCall.raw(this.connection));
    }
    this.finish('done decoded=' + frames + ' events=' + Array.from(this.events).sort().join(','));
  }

  finish(text) {
    if (this.done) return;
    this.done = true;
    this.show(text);
  }

  static raw(connection) {
    return globalThis[Symbol.for('tachyon.tac.peers.hooks')].connection(connection);
  }

  static pattern() {
    const canvas = document.createElement('canvas');
    canvas.width = 320;
    canvas.height = 240;
    document.body.appendChild(canvas);
    const paint = canvas.getContext('2d');
    let tick = 0;
    setInterval(() => {
      tick += 1;
      paint.fillStyle = 'hsl(' + (tick * 7 % 360) + ' 80% 50%)';
      paint.fillRect(0, 0, canvas.width, canvas.height);
      paint.fillStyle = '#fff';
      paint.fillText(String(tick), 20 + tick % 200, 120);
    }, 33);
    return canvas.captureStream(30);
  }

  static async decoded(peer) {
    let frames = 0;
    for (const report of (await peer.getStats()).values()) if (report.type === 'inbound-rtp' && report.kind === 'video') frames += report.framesDecoded || 0;
    return frames;
  }

  static wait(milliseconds) {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
  }
}
`
  }

  /** @param {number} turnPort */
  static browserPage(turnPort) {
    const relay = PeersNativeProbe.relayed(turnPort)
    const server = { urls: relay.iceServers, username: relay.username, credential: relay.credential }
    return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Call</title></head><body>
    <video id="seen" autoplay muted playsinline></video>
    <script>
      const secrets = { same: ${JSON.stringify(SframeCallProbe.SAME_SECRET)}, wrong: ${JSON.stringify(SframeCallProbe.WRONG_SECRET)} }
      const chosen = new URLSearchParams(location.search).get('key')
      const bytes = (text) => Uint8Array.from(atob(text.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((text.length + 3) % 4)), (c) => c.charCodeAt(0))
      globalThis[Symbol.for('tachyon.tac.crypto.hooks')] = Object.freeze({
        key: (name) => name === ${JSON.stringify(SframeCallProbe.KEY_NAME)} ? crypto.subtle.importKey('raw', bytes(secrets[chosen]), 'HKDF', false, ['deriveKey', 'deriveBits']) : Promise.resolve(null),
      })
      const peers = new Map()
      globalThis[Symbol.for('tachyon.tac.peers.hooks')] = Object.freeze({ connection: (id) => peers.get(id) ?? null, opened: () => {}, closed: () => {} })
      const canvas = document.createElement('canvas')
      canvas.width = 320
      canvas.height = 240
      document.body.appendChild(canvas)
      const paint = canvas.getContext('2d')
      let tick = 0
      setInterval(() => { tick += 1; paint.fillStyle = 'hsl(' + (tick * 11 % 360) + ' 70% 45%)'; paint.fillRect(0, 0, 320, 240); paint.fillStyle = '#000'; paint.fillText(String(tick), 20 + tick % 200, 120) }, 33)
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
      const media = () => globalThis[Symbol.for('tachyon.tac.media')]
      window.events = []
      const reply = (event) => window.events.push(event[0])
      const peer = new RTCPeerConnection({ iceServers: [${JSON.stringify(server)}], iceTransportPolicy: 'relay' })
      peers.set('browser', peer)
      peer.ontrack = (event) => { document.getElementById('seen').srcObject = new MediaStream([event.track]) }
      const own = []
      let answered = false
      const post = (message) => fetch('/to-app', { method: 'POST', body: JSON.stringify(message) })
      const share = (message) => answered ? post(message) : own.push(message)
      peer.onicecandidate = (event) => { if (event.candidate) share({ type: 'candidate', candidateJson: JSON.stringify(event.candidate.toJSON()) }) }
      const waiting = []
      const answer = async (sdp) => {
        await peer.setRemoteDescription({ type: 'offer', sdp })
        const stream = canvas.captureStream(30)
        peer.addTrack(stream.getVideoTracks()[0], stream)
        media().encryptFrames(reply, 'browser', true)
        media().useFrameKey(reply, ${SframeCallProbe.KEY_ID}, ${JSON.stringify(SframeCallProbe.KEY_NAME)})
        await wait(500)
        media().sendFrameKey(reply, ${SframeCallProbe.KEY_ID})
        await peer.setLocalDescription(await peer.createAnswer())
        await post({ type: 'answer', sdp: peer.localDescription.sdp })
        answered = true
        for (const message of own.splice(0)) await post(message)
        for (const candidate of waiting.splice(0)) await peer.addIceCandidate(candidate)
      }
      const take = async (message) => {
        if (message.type === 'offer') return answer(message.sdp)
        const candidate = JSON.parse(message.candidateJson)
        if (peer.remoteDescription) return peer.addIceCandidate(candidate)
        waiting.push(candidate)
      }
      const listen = async () => {
        for (;;) {
          const list = await (await fetch('/to-browser')).json()
          for (const message of list) await take(message)
          await wait(${SframeCallProbe.POLL_MS})
        }
      }
      listen()
      window.decoded = async () => {
        let frames = 0
        for (const report of (await peer.getStats()).values()) if (report.type === 'inbound-rtp' && report.kind === 'video') frames += report.framesDecoded ?? 0
        return frames
      }
      window.result = async () => ({ decoded: await window.decoded(), state: peer.connectionState, events: window.events })
    </script>
    <script src="/tac-media.js"></script>
  </body></html>`
  }
}

import { spawn, spawnSync } from 'node:child_process'
import { createServer } from 'node:net'

export class PeersNativeProbe {
  static PREFIX = 'Peers: '
  static TURN_USER = 'probe'
  static TURN_SECRET = 'probe-relay-secret'
  static CHECKING = 'checking'
  static TOTAL = 65536
  static PASSED = `connected, ${PeersNativeProbe.TOTAL} bytes`
  static LABEL = 'Peers'

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

  /** @param {number} port */
  static startTurn(port) {
    const found = spawnSync('turnserver', ['--version'], { encoding: 'utf8' })
    if (found.status !== 0) throw new Error('coturn (turnserver) is required on PATH for the Apple peers checks')
    return spawn('turnserver', ['-n', '--listening-ip=127.0.0.1', '--relay-ip=127.0.0.1', `--listening-port=${port}`, '--min-port=49600', '--max-port=49700', '--lt-cred-mech', `--user=${PeersNativeProbe.TURN_USER}:${PeersNativeProbe.TURN_SECRET}`, '--realm=tachyon.test', '--no-tls', '--allow-loopback-peers', '--no-multicast-peers'], { stdio: 'ignore' })
  }

  /** @param {number} port */
  static relayed(port) {
    return { iceServers: [`turn:127.0.0.1:${port}?transport=udp`], relayOnly: true, username: PeersNativeProbe.TURN_USER, credential: PeersNativeProbe.TURN_SECRET }
  }

  /** @param {{ id: string, name: string, iceServers: string[], relayOnly: boolean, outgoingHosts?: string[], permissions?: Record<string, string> }} settings */
  static manifest(settings) {
    const application = { id: settings.id, version: '0.0.1', build: 1, permissions: settings.permissions ?? {} }
    return {
      name: settings.name,
      start_url: '/',
      tac: {
        application,
        browserApis: ['peers'],
        outgoing: { hosts: settings.outgoingHosts ?? [] },
        peers: { iceServers: settings.iceServers, relayOnly: settings.relayOnly },
      },
    }
  }

  /** @param {{ id: string, name: string, iceServers: string[], relayOnly: boolean, outgoingHosts?: string[], permissions?: Record<string, string>, username?: string, credential?: string }} settings */
  static files(settings) {
    return {
      'client/manifest.json': JSON.stringify(PeersNativeProbe.manifest(settings)),
      'client/pages/tac.html': '<peer-probe hydrate="load" />\n',
      'client/components/peer/probe/tac.html': `<main aria-label="${PeersNativeProbe.LABEL}"><p>{peerProbe.report}</p></main>\n`,
      'client/shared/peer-probe.js': PeersNativeProbe.pageModule(),
      'client/services/PeerProbeService.js': PeersNativeProbe.service(settings.username ?? '', settings.credential ?? ''),
    }
  }

  /** @param {string} username @param {string} credential */
  static service(username, credential) {
    return `import { PeerProbe } from '/shared/peer-probe.js';
@Service()
export default class PeerProbeService {
  static _PREFIX = ${JSON.stringify(PeersNativeProbe.PREFIX)};
  static _USERNAME = ${JSON.stringify(username)};
  static _CREDENTIAL = ${JSON.stringify(credential)};
  /** @type {TacPeers} */
  @Needs
  peers;
  @Needs
  taskScheduler;
  report = PeerProbeService._PREFIX + ${JSON.stringify(PeersNativeProbe.CHECKING)};
  _probe = null;

  @OnStart()
  initialize() {
    this._probe = new PeerProbe(this.peers, (text) => this._show(text));
    this._probe.start(PeerProbeService._USERNAME, PeerProbeService._CREDENTIAL);
  }

  /** @param {PeerDescription} described */
  @OnEvent
  hearDescription(described) {
    this._probe.described(described);
  }

  /** @param {PeerCandidate} candidate */
  @OnEvent
  hearCandidate(candidate) {
    this._probe.candidate(candidate);
  }

  /** @param {PeerState} state */
  @OnEvent
  hearState(state) {
    this._probe.state(state);
  }

  /** @param {PeerChannelOpened} opened */
  @OnEvent
  hearOpened(opened) {
    this._probe.opened(opened);
  }

  /** @param {PeerMessage} message */
  @OnEvent
  hearMessage(message) {
    this._probe.message(message);
  }

  /** @param {PeerFailed} failed */
  @OnEvent
  hearFailed(failed) {
    this._probe.failed(failed);
  }

  /** @param {string} text */
  _show(text) {
    this.report = PeerProbeService._PREFIX + text;
    void this.taskScheduler.render();
  }
}
`
  }

  static pageModule() {
    return `export class PeerProbe {
  static TOTAL = ${PeersNativeProbe.TOTAL};
  static CHUNK = 32768;
  static WAIT = 30000;
  static LABEL = 'probe';

  constructor(peers, show) {
    this.peers = peers;
    this.show = show;
    this.left = '';
    this.right = '';
    this.received = 0;
    this.done = false;
    this.states = [];
  }

  start(username, credential) {
    try {
      this.open(username, credential);
    } catch (error) {
      this.finish('error ' + String(error && error.message || error));
    }
  }

  open(username, credential) {
    if (!this.peers.supported()) return this.finish('unsupported');
    this.left = this.peers.open(username, credential);
    this.right = this.peers.open(username, credential);
    this.peers.openChannel(this.left, PeerProbe.LABEL, true);
    this.peers.offer(this.left);
    setTimeout(() => this.finish('timeout after ' + (this.states.join(' ') || 'no state change')), PeerProbe.WAIT);
    return null;
  }

  described(description) {
    if (description.connection === this.left && description.kind === 'offer') this.peers.answer(this.right, description.sdp);
    if (description.connection === this.right) this.peers.accept(this.left, description.sdp);
  }

  candidate(candidate) {
    const other = candidate.connection === this.left ? this.right : this.left;
    this.peers.addCandidate(other, candidate.candidateJson);
  }

  state(state) {
    this.states.push(state.connection + '=' + state.state);
  }

  opened(opened) {
    if (opened.connection !== this.left) return;
    for (let offset = 0; offset < PeerProbe.TOTAL; offset += PeerProbe.CHUNK) this.peers.send(this.left, opened.channel, PeerProbe.chunk(offset));
  }

  message(message) {
    if (message.connection !== this.right) return;
    this.received += PeerProbe.decoded(message.data).length;
    if (this.received >= PeerProbe.TOTAL) this.finish('connected, ' + this.received + ' bytes');
  }

  failed(failed) {
    this.finish('failed ' + failed.call + ' ' + failed.reason);
  }

  finish(text) {
    if (this.done) return null;
    this.done = true;
    this.show(text);
    return null;
  }

  static chunk(offset) {
    const bytes = new Uint8Array(PeerProbe.CHUNK);
    for (let index = 0; index < bytes.length; index += 1) bytes[index] = (offset + index) % 251;
    return btoa(Array.from(bytes, (byte) => String.fromCharCode(byte)).join('')).replace(/\\+/g, '-').replace(/\\//g, '_').replace(/=+$/, '');
  }

  static decoded(text) {
    return atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  }
}
`
  }
}

#![allow(clippy::expect_used)]

use super::page_peers::PagePeers;
use crate::node_driver::NodeDriver;
use serde_json::{Value, json};
use std::path::Path;

struct PeersDriver;

impl PeersDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const grants = { content: 'peers' }
const settings = { content: JSON.stringify({ iceServers: ['stun:stun.example.com:3478', 'turn:turn.example.com:3478?transport=udp', 'turns:turn.example.com:5349?transport=tcp'], relayOnly: true }) }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => ({ 'meta[name="tachyon-browser-apis"]': grants, 'meta[name="tachyon-peers"]': settings })[selector] ?? null }
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: true, language: 'en', userActivation: { isActive: false } } })
const behaviour = { offer: null, remote: null, strict: false }
const peers = []
class FakeChannel extends EventTarget {
  constructor(label, options) { super(); this.label = label; this.options = options ?? {}; this.ordered = this.options.ordered ?? true; this.readyState = 'connecting'; this.bufferedAmount = 0; this.sent = [] }
  send(bytes) { this.sent.push([...bytes]) }
  close() { this.readyState = 'closed'; this.dispatchEvent(new Event('close')) }
  fire(name, values = {}) { const event = new Event(name); Object.assign(event, values); this.dispatchEvent(event) }
  opened() { this.readyState = 'open'; this.fire('open') }
}
class FakePeer extends EventTarget {
  constructor(config) { super(); if (behaviour.strict && config.iceServers.some((server) => server.urls.includes('?'))) throw new DOMException('Invalid TURN URL query string', 'SyntaxError'); this.config = config; this.connectionState = 'new'; this.localDescription = null; this.remote = null; this.channels = []; this.candidates = []; peers.push(this) }
  async createOffer(options) { if (behaviour.offer) throw new DOMException('rp https://secret.example.com 9f8e', behaviour.offer); return { type: 'offer', sdp: options?.iceRestart ? 'offer restart' : 'offer first' } }
  async createAnswer() { return { type: 'answer', sdp: 'answer sdp' } }
  async setLocalDescription(description) { this.localDescription = description }
  async setRemoteDescription(description) { if (behaviour.remote) throw new DOMException('rp https://secret.example.com 9f8e', behaviour.remote); this.remote = description }
  async addIceCandidate(candidate) { this.candidates.push(candidate) }
  createDataChannel(label, options) { const channel = new FakeChannel(label, options); this.channels.push(channel); return channel }
  close() { this.connectionState = 'closed' }
  fire(name, values = {}) { const event = new Event(name); Object.assign(event, values); this.dispatchEvent(event) }
}
globalThis.RTCPeerConnection = FakePeer
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { publish: () => {}, send: (owner, name, value) => sent.push([owner, name, value]) }
const ask = (owner, call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'peers', call, args }))))
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const take = () => sent.splice(0)
const b64 = (...values) => Buffer.from(values).toString('base64url')
const report = {}
report.noModule = ask('early', 'open', ['', ''])
;(0, eval)(await readFile('./tac-peers.js', 'utf8'))
const hooks = globalThis[Symbol.for('tachyon.tac.peers.hooks')]
const announced = []
hooks.opened((connection, peer) => announced.push(['opened', connection, peer instanceof FakePeer && peer.localDescription === null]))
hooks.closed((connection) => announced.push(['closed', connection]))
report.supported = ask('app', 'supported')
report.open = ask('app', 'open', ['user-1', 'secret-credential'])
report.config = peers[0].config
report.noCredentials = ask('app', 'open', ['', ''])
report.plainConfig = peers[1].config
behaviour.strict = true
report.strict = ask('app', 'open', ['user-1', 'secret-credential'])
report.strictConfig = peers[2].config
behaviour.strict = false
ask('app', 'close', ['peer-3'])
take()
ask('app', 'offer', ['peer-1'])
await settle()
ask('app', 'answer', ['peer-2', 'offer from far'])
await settle()
report.remote = peers[1].remote
ask('app', 'accept', ['peer-1', 'answer from far'])
ask('app', 'addCandidate', ['peer-1', JSON.stringify({ candidate: 'candidate:1 1 udp 1 192.0.2.1 9 typ host', sdpMid: '0' })])
ask('app', 'addCandidate', ['peer-1', ''])
ask('app', 'addCandidate', ['peer-1', '{not json'])
ask('app', 'addCandidate', ['peer-1', '[]'])
report.tooLongCandidate = ask('app', 'addCandidate', ['peer-1', 'x'.repeat(65537)])
ask('app', 'restart', ['peer-1'])
await settle()
report.candidates = peers[0].candidates
peers[0].fire('icecandidate', { candidate: { toJSON: () => ({ candidate: 'candidate:2 1 udp 1 198.51.100.7 9 typ srflx', sdpMid: '0' }) } })
peers[0].fire('icecandidate', { candidate: null })
peers[0].connectionState = 'connected'
peers[0].fire('connectionstatechange')
peers[0].fire('negotiationneeded')
peers[0].connectionState = 'odd'
peers[0].fire('connectionstatechange')
report.signalling = take()
report.reliable = ask('app', 'openChannel', ['peer-1', 'chat', true])
report.unordered = ask('app', 'openChannel', ['peer-1', 'typing', false])
report.badLabel = ask('app', 'openChannel', ['peer-1', 'no spaces', true])
report.options = peers[0].channels.map((channel) => channel.options)
report.notOpen = ask('app', 'send', ['peer-1', 'channel-1', b64(1)])
peers[0].channels[0].opened()
peers[0].channels[1].opened()
const remote = new FakeChannel('files', { ordered: true })
peers[0].fire('datachannel', { channel: remote })
remote.opened()
report.opened = take()
report.queued = ask('app', 'send', ['peer-1', 'channel-1', b64(1, 2, 255)])
report.sentBytes = [...peers[0].channels[0].sent]
peers[0].channels[0].bufferedAmount = 1048576
report.busy = ask('app', 'send', ['peer-1', 'channel-1', b64(3)])
peers[0].channels[0].fire('bufferedamountlow')
peers[0].channels[0].fire('bufferedamountlow')
peers[0].channels[0].bufferedAmount = 0
report.badData = ask('app', 'send', ['peer-1', 'channel-1', 'not base64!'])
report.paddedData = ask('app', 'send', ['peer-1', 'channel-1', 'AQ=='])
report.largeData = ask('app', 'send', ['peer-1', 'channel-1', Buffer.alloc(32769).toString('base64url')])
report.largestData = ask('app', 'send', ['peer-1', 'channel-1', Buffer.alloc(32768).toString('base64url')])
report.unknownChannel = ask('app', 'send', ['peer-1', 'channel-9', b64(1)])
peers[0].channels[0].fire('message', { data: new Uint8Array([251, 255]).buffer })
peers[0].channels[0].fire('message', { data: 'hé' })
peers[0].channels[1].fire('message', { data: new Uint8Array(32769).buffer })
report.received = take()
behaviour.offer = 'UnknownError'
ask('app', 'offer', ['peer-2'])
await settle()
behaviour.offer = null
behaviour.remote = 'OperationError'
ask('app', 'accept', ['peer-2', 'answer'])
behaviour.remote = null
await settle()
report.failures = take()
report.unknownConnection = ask('app', 'offer', ['peer-9'])
report.closeChannel = ask('app', 'closeChannel', ['peer-1', 'channel-1'])
report.close = ask('app', 'close', ['peer-1'])
peers[0].fire('icecandidate', { candidate: null })
report.closed = take()
report.afterClose = ask('app', 'offer', ['peer-1'])
report.lookup = [hooks.connection('peer-2') === peers[1], hooks.connection('peer-1')]
report.announced = announced.slice(0, 5)
for (let index = 0; index < 31; index += 1) ask('app', 'open', ['', ''])
report.tooMany = ask('app', 'open', ['', ''])
for (let index = 0; index < 16; index += 1) ask('app', 'openChannel', ['peer-2', `c${index}`, true])
report.tooManyChannels = ask('app', 'openChannel', ['peer-2', 'one-more', true])
grants.content = 'share'
report.notGranted = [ask('x', 'supported'), ask('x', 'open', ['', ''])]
grants.content = 'peers'
delete globalThis.RTCPeerConnection
report.unsupported = [ask('x', 'supported'), ask('x', 'open', ['', ''])]
take()
console.log(JSON.stringify(report))
"#;

    fn run() -> Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        Self::write(root.path(), "tac-peers.js", &PagePeers::script());
        Self::write(root.path(), "drive.mjs", Self::DRIVER.as_bytes());
        let mut command = NodeDriver::command();
        command.arg("drive.mjs").current_dir(root.path());
        let output = NodeDriver::output(&mut command).expect("node drive.mjs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    fn write(root: &Path, name: &str, bytes: &[u8]) {
        std::fs::write(root.join(name), bytes).expect("write");
    }

    fn event(name: &str, value: Value) -> Value {
        Value::Array(vec![json!("app"), json!(name), value])
    }

    fn failed(connection: &str, call: &str, reason: &str) -> Value {
        let message = match reason {
            "invalid" => "The description, candidate or message is not valid for this connection.",
            "tooLarge" => {
                "A message from the other side is larger than 32768 bytes; the channel was closed."
            }
            _ => "The peer connection call failed.",
        };
        Self::event(
            "PeerFailed",
            json!({"connection": connection, "call": call, "reason": reason, "message": message}),
        )
    }

    fn error(text: &str) -> Value {
        json!({ "error": text })
    }
}

#[test]
fn the_page_opens_connections_only_with_the_declared_servers() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PeersDriver::run();
    assert_eq!(report["supported"], json!({"value": true}));
    assert_eq!(report["open"], json!({"value": "peer-1"}));
    assert_eq!(
        report["config"],
        json!({"iceServers": [
            {"urls": "stun:stun.example.com:3478"},
            {"urls": "turn:turn.example.com:3478?transport=udp", "username": "user-1", "credential": "secret-credential"},
            {"urls": "turns:turn.example.com:5349?transport=tcp", "username": "user-1", "credential": "secret-credential"},
        ], "iceTransportPolicy": "relay"}),
        "only TURN servers get the credentials, and relayOnly sets the relay policy"
    );
    assert_eq!(report["noCredentials"], json!({"value": "peer-2"}));
    assert_eq!(
        report["plainConfig"]["iceServers"][1],
        json!({"urls": "turn:turn.example.com:3478?transport=udp"})
    );
    assert_eq!(report["strict"], json!({"value": "peer-3"}));
    assert_eq!(
        report["strictConfig"]["iceServers"],
        json!([
            {"urls": "stun:stun.example.com:3478"},
            {"urls": "turn:turn.example.com:3478", "username": "user-1", "credential": "secret-credential"},
            {"urls": "turns:turn.example.com:5349", "username": "user-1", "credential": "secret-credential"},
        ]),
        "a browser that refuses ?transport= (WebKit) gets the servers without it"
    );
    assert!(
        !report["signalling"]
            .to_string()
            .contains("secret-credential"),
        "no event repeats the TURN credential: {report}"
    );
}

#[test]
fn the_page_carries_descriptions_candidates_and_state_to_the_owner() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PeersDriver::run();
    let event = PeersDriver::event;
    assert_eq!(
        report["remote"],
        json!({"type": "offer", "sdp": "offer from far"})
    );
    assert_eq!(
        report["candidates"],
        json!([
            {"candidate": "candidate:1 1 udp 1 192.0.2.1 9 typ host", "sdpMid": "0"},
            {"candidate": ""},
        ])
    );
    assert_eq!(
        report["tooLongCandidate"],
        PeersDriver::error("A description or candidate is at most 65536 bytes.")
    );
    assert_eq!(
        report["signalling"],
        json!([
            event(
                "PeerDescription",
                json!({"connection": "peer-1", "kind": "offer", "sdp": "offer first"})
            ),
            event(
                "PeerDescription",
                json!({"connection": "peer-2", "kind": "answer", "sdp": "answer sdp"})
            ),
            PeersDriver::failed("peer-1", "addCandidate", "invalid"),
            PeersDriver::failed("peer-1", "addCandidate", "invalid"),
            event(
                "PeerDescription",
                json!({"connection": "peer-1", "kind": "offer", "sdp": "offer restart"})
            ),
            event(
                "PeerCandidate",
                json!({"connection": "peer-1", "candidateJson": "{\"candidate\":\"candidate:2 1 udp 1 198.51.100.7 9 typ srflx\",\"sdpMid\":\"0\"}"})
            ),
            event(
                "PeerCandidate",
                json!({"connection": "peer-1", "candidateJson": ""})
            ),
            event(
                "PeerState",
                json!({"connection": "peer-1", "state": "connected"})
            ),
            event("PeerNegotiationNeeded", json!({"connection": "peer-1"})),
        ]),
        "an unknown state sends nothing: {report}"
    );
}

#[test]
fn channels_send_bytes_with_back_pressure_and_bound_every_message() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PeersDriver::run();
    let event = PeersDriver::event;
    assert_eq!(report["reliable"], json!({"value": "channel-1"}));
    assert_eq!(report["unordered"], json!({"value": "channel-2"}));
    assert_eq!(
        report["badLabel"],
        PeersDriver::error(
            "A channel label is 1 to 64 letters, digits, dots, dashes or underscores."
        )
    );
    assert_eq!(
        report["options"],
        json!([{"ordered": true}, {"ordered": false, "maxRetransmits": 0}])
    );
    assert_eq!(
        report["notOpen"],
        PeersDriver::error("The channel is not open yet. Wait for PeerChannelOpened.")
    );
    assert_eq!(
        report["opened"],
        json!([
            event(
                "PeerChannelOpened",
                json!({"connection": "peer-1", "channel": "channel-1", "label": "chat", "ordered": true, "remote": false})
            ),
            event(
                "PeerChannelOpened",
                json!({"connection": "peer-1", "channel": "channel-2", "label": "typing", "ordered": false, "remote": false})
            ),
            event(
                "PeerChannelOpened",
                json!({"connection": "peer-1", "channel": "channel-3", "label": "files", "ordered": true, "remote": true})
            ),
        ])
    );
    assert_eq!(report["queued"], json!({"value": true}));
    assert_eq!(report["sentBytes"], json!([[1, 2, 255]]));
    assert_eq!(report["busy"], json!({"value": false}));
    let base64 = PeersDriver::error("Peer data is base64url text without padding.");
    assert_eq!(report["badData"], base64);
    assert_eq!(report["paddedData"], base64);
    assert_eq!(
        report["largeData"],
        PeersDriver::error("A peer message is at most 32768 bytes.")
    );
    assert_eq!(report["largestData"], json!({"value": true}));
    assert_eq!(
        report["unknownChannel"],
        PeersDriver::error("No open channel is named \"channel-9\" on this connection.")
    );
    assert_eq!(
        report["received"],
        json!([
            event(
                "PeerChannelReady",
                json!({"connection": "peer-1", "channel": "channel-1"})
            ),
            event(
                "PeerMessage",
                json!({"connection": "peer-1", "channel": "channel-1", "data": "-_8"})
            ),
            event(
                "PeerMessage",
                json!({"connection": "peer-1", "channel": "channel-1", "data": "aMOp"})
            ),
            PeersDriver::failed("peer-1", "receive", "tooLarge"),
            event(
                "PeerChannelClosed",
                json!({"connection": "peer-1", "channel": "channel-2"})
            ),
        ]),
        "a drained channel says ready once, text arrives as UTF-8 bytes, and a large message closes its channel: {report}"
    );
}

#[test]
fn failures_close_and_limits_never_show_browser_text() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PeersDriver::run();
    let event = PeersDriver::event;
    assert_eq!(
        report["failures"],
        json!([
            PeersDriver::failed("peer-2", "offer", "failed"),
            PeersDriver::failed("peer-2", "accept", "invalid"),
        ])
    );
    for secret in ["secret.example.com", "9f8e"] {
        assert!(
            !report.to_string().contains(secret),
            "no browser message reaches an event: {report}"
        );
    }
    let unknown = PeersDriver::error("No open peer connection is named \"peer-9\".");
    assert_eq!(report["unknownConnection"], unknown);
    assert_eq!(report["closeChannel"], json!({"value": null}));
    assert_eq!(report["close"], json!({"value": null}));
    assert_eq!(
        report["closed"],
        json!([
            event(
                "PeerChannelClosed",
                json!({"connection": "peer-1", "channel": "channel-1"})
            ),
            event(
                "PeerState",
                json!({"connection": "peer-1", "state": "closed"})
            ),
        ]),
        "a closed connection sends nothing more: {report}"
    );
    assert_eq!(
        report["lookup"],
        json!([true, null]),
        "other page modules (Media) reach an open connection by its id"
    );
    assert_eq!(
        report["announced"],
        json!([
            ["opened", "peer-1", true],
            ["opened", "peer-2", true],
            ["opened", "peer-3", true],
            ["closed", "peer-3"],
            ["closed", "peer-1"],
        ]),
        "hooks hear a connection before its first offer, and its close"
    );
    assert_eq!(
        report["afterClose"],
        PeersDriver::error("No open peer connection is named \"peer-1\".")
    );
    assert_eq!(
        report["tooMany"],
        PeersDriver::error("A page may hold at most 32 peer connections.")
    );
    assert_eq!(
        report["tooManyChannels"],
        PeersDriver::error("A page may hold at most 16 peer channels on one connection.")
    );
}

#[test]
fn the_page_refuses_peers_without_the_module_the_grant_or_webrtc() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PeersDriver::run();
    assert_eq!(
        report["noModule"],
        PeersDriver::error("The peers browser API did not load on this page.")
    );
    let not_granted = "The peers browser API is not granted. Add \"peers\" to tac.browserApis in client/manifest.json.";
    assert_eq!(
        report["notGranted"],
        json!([{"value": true}, {"error": not_granted}])
    );
    assert_eq!(
        report["unsupported"],
        json!([{"value": false}, {"error": "Peer connections are not available here (unsupported)."}])
    );
}

#[test]
fn only_a_build_that_grants_peers_ships_the_module_and_the_server_list() {
    let root = tempfile::tempdir().expect("project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages");
    std::fs::write(pages.join("tac.html"), "<page-home />").expect("page");
    let component = root.path().join("client/components/page/home");
    std::fs::create_dir_all(&component).expect("component");
    std::fs::write(component.join("tac.html"), "<main>Home</main>").expect("component view");
    let manifest = |tac: &str| {
        std::fs::write(
            root.path().join("client/manifest.json"),
            format!(r#"{{"tac":{tac}}}"#),
        )
        .expect("manifest");
    };
    let built = |native: bool| {
        let output = tempfile::tempdir().expect("output");
        let project = crate::ProjectDiscovery::discover(root.path())?;
        let options = super::BuildOptions {
            output_directory: output.path().to_path_buf(),
            incremental: false,
            ..super::BuildOptions::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let target = native.then_some(tachyon_contracts::NativeTarget::Android);
        let result = runtime.block_on(super::WebCompiler::build_project_with_target(
            &project, &options, target,
        ));
        result.map(|_| {
            let page = std::fs::read_to_string(output.path().join("index.html")).expect("page");
            let worker =
                std::fs::read_to_string(output.path().join("tachyon-sw.js")).expect("worker");
            let shipped = output.path().join(PagePeers::PATH).is_file();
            (page, worker, shipped)
        })
    };
    let tag =
        r#"<script type="module" src="/.tachyon/tac-peers.js" data-tachyon-runtime></script>"#;
    let settings = r#"<meta name="tachyon-peers" content="{&quot;iceServers&quot;:[&quot;turn:turn.example.com:3478&quot;],&quot;relayOnly&quot;:true}">"#;
    manifest(
        r#"{"browserApis":["peers"],"outgoing":{"hosts":["turn.example.com"]},"peers":{"iceServers":["turn:turn.example.com:3478"],"relayOnly":true}}"#,
    );
    for native in [false, true] {
        let (page, worker, shipped) = built(native).expect("build");
        let module = page.find(tag).expect("the page loads the peers module");
        let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
        assert!(module < runtime, "{page}");
        assert!(page.contains(settings), "{page}");
        assert!(shipped, "native {native}");
        assert!(
            worker.contains("tac-peers.js"),
            "offline pages keep it: {worker}"
        );
    }
    manifest(r#"{"browserApis":["share"]}"#);
    let (page, _, shipped) = built(false).expect("build");
    assert!(!page.contains("tac-peers"), "no grant, no module: {page}");
    assert!(
        !page.contains("tachyon-peers"),
        "no grant, no settings: {page}"
    );
    assert!(!shipped);
    manifest(r#"{"browserApis":["share"],"peers":{"iceServers":[]}}"#);
    assert!(built(false).is_err(), "tac.peers needs the grant");
    manifest(r#"{"browserApis":["peers"],"peers":{"iceServers":["turn:relay.other.org:3478"]}}"#);
    assert!(built(false).is_err(), "an undeclared TURN host is refused");
}

#![allow(clippy::expect_used)]

use super::page_media::PageMedia;
use crate::node_driver::NodeDriver;
use std::path::Path;

struct MediaDriver;

impl MediaDriver {
    const WEB_RUNTIME: &'static str = include_str!("../template/tac-web.js");

    const DRIVER: &'static str = r#"import { readFile } from 'node:fs/promises'
const grants = { content: 'microphone camera' }
const activation = { isActive: true }
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: (selector) => selector === 'meta[name="tachyon-browser-apis"]' ? grants : null }
globalThis.addEventListener = () => {}
class FakeTrack extends EventTarget {
  constructor(kind, deviceId, groupId, label) { super(); Object.assign(this, { kind, deviceId, groupId, label, enabled: true, readyState: 'live' }) }
  getSettings() { return { deviceId: this.deviceId, groupId: this.groupId, width: this.kind === 'video' ? 640 : undefined, height: this.kind === 'video' ? 360 : undefined } }
  stop() { this.readyState = 'ended' }
  end() { this.readyState = 'ended'; this.dispatchEvent(new Event('ended')) }
}
const list = [
  { kind: 'audioinput', deviceId: 'default', groupId: 'g1', label: 'Default' },
  { kind: 'audioinput', deviceId: 'mic-1', groupId: 'g1', label: 'Built-in' },
  { kind: 'videoinput', deviceId: 'cam-1', groupId: 'g3', label: 'Camera' },
  { kind: 'audiooutput', deviceId: 'default', groupId: 'g1', label: 'Speaker' },
]
const asked = []
const tracks = []
const browser = { refuse: null }
const media = new EventTarget()
media.enumerateDevices = async () => list.map((device) => ({ ...device }))
media.getUserMedia = async (constraints) => {
  asked.push(constraints)
  if (browser.refuse) throw new DOMException('no', browser.refuse)
  const kind = constraints.audio ? 'audio' : 'video'
  const wanted = (constraints.audio ?? constraints.video).deviceId?.exact
  const device = list.find((item) => item.kind === (kind === 'audio' ? 'audioinput' : 'videoinput') && (wanted ? item.deviceId === wanted : true))
  const track = new FakeTrack(kind, device.deviceId === 'default' ? 'mic-1' : device.deviceId, device.groupId, device.label)
  tracks.push(track)
  return { getTracks: () => [track] }
}
media.getDisplayMedia = async (constraints) => {
  asked.push({ display: constraints })
  const video = new FakeTrack('video', 'screen', 'gs', 'Screen 1')
  const audio = new FakeTrack('audio', 'screen-audio', 'gs', 'System audio')
  tracks.push(video)
  return { getVideoTracks: () => [video], getAudioTracks: () => constraints.audio ? [audio] : [] }
}
Object.defineProperty(navigator, 'mediaDevices', { configurable: true, value: media })
Object.defineProperty(navigator, 'userActivation', { configurable: true, value: activation })
globalThis.HTMLMediaElement = class { setSinkId() {} }
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-media.js', 'utf8'))
const sent = []
TacWebApis.events = { send: (owner, name, value) => sent.push([owner, name, value]), publish: () => {} }
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const call = (owner, name, ...values) => { try { TacWebApis.calling(owner, () => TacWebApis.apis.media[name](...values)); return 'ok' } catch (error) { return error.message } }
const take = () => sent.splice(0)
const tracksApi = globalThis[Symbol.for('tachyon.tac.media.tracks')]
const report = {}
report.supported = ['capture', 'screen', 'speakers', 'frames', 'background', 'nothing'].map((feature) => TacWebApis.apis.media.supported(feature))
report.sync = [call('a', 'captureMicrophone', 'bad name', '', true), call('a', 'captureScreen', 'screen', false)]
report.calls = [call('a', 'captureMicrophone', 'mic', '', true), call('a', 'captureCamera', 'cam', 'cam-1', 'low')]
await settle()
report.duplicate = call('a', 'captureCamera', 'cam', '', 'low')
report.started = take()
report.constraints = asked.splice(0)
call('b', 'devices', 'r1')
await settle()
report.devices = take()
call('a', 'mute', 'mic', true)
report.micEnabled = tracksApi.track('mic').enabled
call('a', 'mute', 'cam', true)
report.camMuted = [tracksApi.track('cam'), tracks[1].readyState]
call('a', 'mute', 'cam', false)
await settle()
report.camBack = [tracksApi.track('cam') === tracks[2], take().map((event) => event[1])]
tracks[2].end()
await settle()
report.camGone = take()
list[1].groupId = 'g9'
list[0].groupId = 'g9'
list.push({ kind: 'audioinput', deviceId: 'mic-2', groupId: 'g9', label: 'Headset' })
media.dispatchEvent(new Event('devicechange'))
await settle()
report.followed = [take(), tracksApi.track('mic') === tracks[3], tracks[0].readyState, tracks[3].enabled]
browser.refuse = 'NotAllowedError'
call('c', 'captureCamera', 'refused', '', 'high')
browser.refuse = 'NotReadableError'
call('c', 'captureCamera', 'busy', '', 'high')
await settle()
report.refused = take()
browser.refuse = null
report.unknown = call('a', 'stopTrack', 'nothing')
call('a', 'stopTrack', 'mic')
report.stopped = [take(), tracks[3].readyState]
grants.content = 'screen'
activation.isActive = false
report.noGesture = call('a', 'captureScreen', 'share', true)
activation.isActive = true
report.notGranted = call('a', 'captureMicrophone', 'mic2', '', true)
call('a', 'captureScreen', 'share', true)
await settle()
report.screen = take()
tracksApi.track('share').end()
await settle()
report.screenEnded = take()
console.log(JSON.stringify(report))
"#;

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        Self::write(root.path(), "tac-web.js", Self::WEB_RUNTIME.as_bytes());
        let [(_, media), _, _] = PageMedia::files();
        Self::write(root.path(), "tac-media.js", &media);
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

    fn started(
        owner: &str,
        track: &str,
        kind: &str,
        device: &str,
        label: &str,
    ) -> serde_json::Value {
        let (width, height) = if kind == "video" { (640, 360) } else { (0, 0) };
        serde_json::json!([owner, "MediaTrackStarted", {"track": track, "kind": kind, "device": device, "label": label, "width": width, "height": height}])
    }

    fn ended(owner: &str, track: &str, reason: &str) -> serde_json::Value {
        serde_json::json!([owner, "MediaTrackEnded", {"track": track, "reason": reason}])
    }

    fn failed(
        owner: &str,
        track: &str,
        call: &str,
        reason: &str,
        message: &str,
    ) -> serde_json::Value {
        serde_json::json!([owner, "MediaFailed", {"track": track, "call": call, "reason": reason, "message": message}])
    }

    fn device(id: &str, kind: &str, label: &str, chosen: bool, index: u8) -> serde_json::Value {
        serde_json::json!(["b", "MediaDeviceFound", {"request": "r1", "id": id, "kind": kind, "label": label, "chosen": chosen, "index": index, "count": 4}])
    }
}

struct PhoneDriver;

impl PhoneDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
globalThis.document = { querySelector: () => ({ content: 'microphone' }), addEventListener: () => {} }
globalThis.addEventListener = () => {}
const media = new EventTarget()
media.enumerateDevices = async () => [
  { kind: 'audioinput', deviceId: 'mic-1', groupId: 'g1', label: 'Built-in' },
  { kind: 'audiooutput', deviceId: 'default', groupId: 'g1', label: 'Browser speaker' },
]
Object.defineProperty(navigator, 'mediaDevices', { configurable: true, value: media })
const asked = []
const answers = {
  'browserApi.media.speakers': () => ({ value: [{ id: 'earpiece', label: 'iPhone', chosen: true }, { id: 'speaker', label: 'Speaker', chosen: false }] }),
  'browserApi.media.chooseSpeaker': (values) => values.device === 'speaker' ? { value: { device: 'speaker' } } : { error: 'There is no such speaker (unknown).' },
  'browserApi.media.callActive': (values) => values.title === 'fail' ? { error: 'Allow the microphone before a call runs in the background.' } : { value: { on: values.on, background: true } },
}
globalThis.__tachyonNativeHostCall = async (capability, payload) => { asked.push([capability, JSON.parse(payload)]); return JSON.stringify(answers[capability](JSON.parse(payload))) }
globalThis.__tachyonBrowserApiBridge = Object.freeze(['media'])
;(0, eval)(await readFile('./tac-media.js', 'utf8'))
const api = globalThis[Symbol.for('tachyon.tac.media')]
const sent = []
const reply = (event) => sent.push(event)
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const report = {}
report.speakers = api.supported('speakers')
api.devices(reply, 'r')
await settle()
report.devices = sent.splice(0)
api.chooseSpeaker(reply, 'speaker')
api.chooseSpeaker(reply, 'nothing')
await settle()
report.chosen = sent.splice(0)
api.callActive(reply, true, 'Call with Ana')
api.callActive(reply, true, 'fail')
await settle()
report.phone = sent.splice(0)
report.asked = asked.splice(0)
globalThis.__tachyonBrowserApiBridge = Object.freeze([])
api.callActive(reply, true, 'Call with Ana')
await settle()
report.web = sent.splice(0)
report.webAsked = asked.length
console.log(JSON.stringify(report))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        let [(_, media), _, _] = PageMedia::files();
        std::fs::write(root.path().join("tac-media.js"), media).expect("media");
        std::fs::write(root.path().join("drive.mjs"), Self::DRIVER).expect("driver");
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
}

#[test]
fn on_a_phone_the_host_lists_and_chooses_speakers_and_runs_call_audio() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = PhoneDriver::run();
    assert_eq!(report["speakers"], true);
    let found = |id: &str, kind: &str, label: &str, chosen: bool, index: u8| serde_json::json!(["MediaDeviceFound", {"request": "r", "id": id, "kind": kind, "label": label, "chosen": chosen, "index": index, "count": 3}]);
    assert_eq!(
        report["devices"],
        serde_json::json!([
            found("mic-1", "microphone", "Built-in", false, 0),
            found("earpiece", "speaker", "iPhone", true, 1),
            found("speaker", "speaker", "Speaker", false, 2),
        ]),
        "the host's speakers replace the browser's"
    );
    assert_eq!(
        report["chosen"],
        serde_json::json!([
            ["MediaSpeakerChosen", {"device": "speaker"}],
            ["MediaFailed", {"track": "", "call": "chooseSpeaker", "reason": "unknown", "message": "There is no such track or device."}],
        ])
    );
    assert_eq!(
        report["phone"],
        serde_json::json!([
            ["MediaCallActive", {"on": true, "background": true}],
            ["MediaFailed", {"track": "", "call": "callActive", "reason": "failed", "message": "The call audio could not start or stop."}],
        ])
    );
    assert_eq!(
        report["asked"],
        serde_json::json!([
            ["browserApi.media.speakers", {}],
            ["browserApi.media.chooseSpeaker", {"device": "speaker"}],
            ["browserApi.media.chooseSpeaker", {"device": "nothing"}],
            ["browserApi.media.callActive", {"on": true, "title": "Call with Ana"}],
            ["browserApi.media.callActive", {"on": true, "title": "fail"}],
        ])
    );
    assert_eq!(
        report["web"],
        serde_json::json!([["MediaCallActive", {"on": true, "background": false}]]),
        "on the web callActive only answers"
    );
    assert_eq!(report["webAsked"], 0);
}

struct CallsPageDriver;

impl CallsPageDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
let made = 0
class FakeTrack extends EventTarget { constructor(kind) { super(); made += 1; this.kind = kind; this.id = `t${made}`; this.label = kind; this.enabled = true } getSettings() { return { deviceId: `d-${this.kind}`, width: 640, height: 360 } } stop() { this.stopped = true } }
class FakeStream { constructor(tracks = []) { made += 1; this.tracks = tracks; this.id = `s${made}` } }
globalThis.MediaStream = FakeStream
globalThis.HTMLMediaElement = class { setSinkId() {} }
const players = []
let blockNext = false
const player = () => ({ autoplay: false, srcObject: null, sink: '', plays: 0, paused: false,
  play() { this.plays += 1; if (blockNext) { blockNext = false; return Promise.reject(Object.assign(new Error('no'), { name: 'NotAllowedError' })) } return Promise.resolve() },
  pause() { this.paused = true }, setSinkId(id) { this.sink = id; return Promise.resolve() } })
globalThis.document = { querySelector: () => ({ content: 'microphone camera' }), addEventListener: () => {}, createElement: (tag) => { const element = player(); players.push(element); return element } }
globalThis.addEventListener = () => {}
const media = new EventTarget()
media.getUserMedia = async (constraints) => ({ getTracks: () => [new FakeTrack(constraints.audio ? 'audio' : 'video')] })
media.enumerateDevices = async () => []
Object.defineProperty(navigator, 'mediaDevices', { configurable: true, value: media })
const peer = new EventTarget()
peer.mids = 0
peer.signalingState = 'stable'
peer.removed = []
peer.addTransceiver = (trackOrKind, options) => {
  const kind = typeof trackOrKind === 'string' ? trackOrKind : trackOrKind.kind
  const transceiver = { mid: String(peer.mids), options, currentDirection: 'sendrecv',
    sender: { track: typeof trackOrKind === 'string' ? null : trackOrKind, replaceTrack(track) { this.track = track; return Promise.resolve() } },
    receiver: { track: new FakeTrack(kind) } }
  peer.mids += 1
  return transceiver
}
peer.removeTrack = (sender) => peer.removed.push(sender)
const hooks = { opened: [], closed: [] }
globalThis[Symbol.for('tachyon.tac.peers.hooks')] = { connection: (id) => id === 'peer-1' ? peer : null, opened: (hook) => hooks.opened.push(hook), closed: (hook) => hooks.closed.push(hook) }
const Media = (0, eval)((await readFile('./tac-media.js', 'utf8')) + '\nTacMedia')
const api = globalThis[Symbol.for('tachyon.tac.media')]
const tracks = globalThis[Symbol.for('tachyon.tac.media.tracks')]
const sent = []
const other = []
const reply = (event) => sent.push(event)
const otherReply = (event) => other.push(event)
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const attempt = (work) => { try { work(); return 'ok' } catch (error) { return error.message } }
const report = {}
report.hooked = [hooks.opened.length, hooks.closed.length]
api.captureMicrophone(reply, 'mic', '', true)
api.captureCamera(reply, 'cam', '', 'medium')
api.captureCamera(reply, 'cam2', '', 'low')
await settle()
sent.splice(0)
api.send(reply, 'peer-1', 'mic', 'me', false)
api.send(reply, 'peer-1', 'cam', 'me', true)
const [micSent, camSent] = [Media.sending.get('peer-1 mic').transceiver, Media.sending.get('peer-1 cam').transceiver]
report.sending = sent.splice(0)
report.options = [micSent.options.direction, micSent.options.sendEncodings ?? null, camSent.options.sendEncodings, micSent.options.streams[0] === camSent.options.streams[0]]
report.refused = [attempt(() => api.send(reply, 'peer-1', 'mic', 'me', false)), attempt(() => api.send(reply, 'peer-9', 'mic', '', false)), attempt(() => api.send(reply, 'peer-1', 'nothing', '', false)), attempt(() => api.send(reply, 'peer-1', 'cam2', 'bad name!', false))]
api.mute(reply, 'cam', true)
await settle()
report.cameraOff = camSent.sender.track
api.mute(reply, 'cam', false)
await settle()
report.cameraBack = [camSent.sender.track?.kind ?? null, sent.splice(0).map((event) => event[0])]
const cam2 = tracks.track('cam2')
api.replace(reply, 'peer-1', 'cam', 'cam2')
await settle()
report.replaced = [camSent.sender.track === cam2, Media.sending.has('peer-1 cam2'), Media.sending.has('peer-1 cam')]
report.replaceRefused = attempt(() => api.replace(reply, 'peer-1', 'cam2', 'mic'))
api.unsend(reply, 'peer-1', 'mic')
report.unsent = [peer.removed.length, peer.removed[0] === micSent.sender, attempt(() => api.unsend(reply, 'peer-1', 'mic'))]
const arrive = (kind, mid, stream) => {
  const transceiver = { mid, currentDirection: 'recvonly', sender: {}, receiver: { track: new FakeTrack(kind), levels: [], getSynchronizationSources() { return this.levels.map((audioLevel) => ({ audioLevel })) } } }
  const event = new Event('track')
  event.transceiver = transceiver
  event.streams = stream === '' ? [] : [{ id: stream }]
  peer.dispatchEvent(event)
  return transceiver
}
api.stats(otherReply, 'warmup', 'peer-1')
await settle()
other.splice(0)
const audioIn = arrive('audio', '5', 'alice')
const videoIn = arrive('video', '6', '')
await settle()
report.arrived = sent.splice(0)
report.arrivedToCaller = other.splice(0)
report.remoteTracks = [tracks.track('peer-1.5') === audioIn.receiver.track, tracks.track('peer-1.6') === videoIn.receiver.track]
report.players = players.map((element) => [element.autoplay, element.plays, element.srcObject?.tracks[0] === audioIn.receiver.track])
audioIn.receiver.track.dispatchEvent(new Event('mute'))
audioIn.receiver.track.dispatchEvent(new Event('unmute'))
report.muted = other.splice(0)
blockNext = true
const blockedIn = arrive('audio', '7', 'bob')
await settle()
report.blocked = other.splice(0)
sent.splice(0)
const blockedPlayer = players[players.length - 1]
api.resumeAudio(reply)
await settle()
report.resumed = [blockedPlayer.plays, sent.splice(0)]
api.chooseSpeaker(reply, 'spk-2')
await settle()
report.speaker = [players.map((element) => element.sink), sent.splice(0)]
const outbound = { id: 'out-h', type: 'outbound-rtp', kind: 'video', mid: camSent.mid, rid: 'h', bytesSent: 10000, timestamp: 1000, remoteId: 'far-h', framesPerSecond: 29.6, frameWidth: 640, frameHeight: 360 }
const inbound = { id: 'in-5', type: 'inbound-rtp', kind: 'audio', mid: '5', bytesReceived: 4000, packetsLost: 2, packetsReceived: 98, jitter: 0.012, timestamp: 1000 }
const far = { id: 'far-h', type: 'remote-inbound-rtp', packetsLost: 3, fractionLost: 0.02, jitter: 0.005, roundTripTime: 0.041 }
const pair = { id: 'pair', type: 'candidate-pair', nominated: true, state: 'succeeded', currentRoundTripTime: 0.03 }
const stray = { id: 'out-x', type: 'outbound-rtp', kind: 'video', mid: '99', bytesSent: 1, timestamp: 1000 }
peer.getStats = async () => new Map([outbound, inbound, far, pair, stray].map((item) => [item.id, { ...item }]))
api.stats(reply, 'r1', 'peer-1')
await settle()
outbound.bytesSent = 72500
outbound.timestamp = 2000
inbound.bytesReceived = 8000
inbound.timestamp = 2000
api.stats(reply, 'r2', 'peer-1')
await settle()
report.stats = sent.splice(0)
peer.getStats = async () => new Map()
api.stats(reply, 'r3', 'peer-1')
await settle()
report.noStats = sent.splice(0)
audioIn.currentDirection = 'inactive'
peer.signalingState = 'have-local-offer'
peer.dispatchEvent(new Event('signalingstatechange'))
report.notYet = sent.length
peer.signalingState = 'stable'
peer.dispatchEvent(new Event('signalingstatechange'))
report.gone = [sent.splice(0), tracks.track('peer-1.5'), players[0].srcObject, players[0].paused]
const carolIn = arrive('audio', '8', 'carol')
await settle()
sent.splice(0)
api.speaking(reply, true)
const script = [[0.3, 0], [0.3, 0], [0.3, 0.5], [0.3, 0.5], [0, 0.5], [0, 0.5], [0, 0.5], [0, 0.5], [0, 0.5]]
report.speakingTicks = []
for (const [bob, carol] of script) {
  blockedIn.receiver.levels = [bob]
  carolIn.receiver.levels = [carol]
  Media.measure()
  report.speakingTicks.push(sent.splice(0))
}
api.speaking(reply, false)
hooks.closed[0]('peer-1')
report.closed = [sent.splice(0), Media.sending.size, tracks.track('peer-1.8')]
console.log(JSON.stringify(report))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        let [(_, media), _, _] = PageMedia::files();
        std::fs::write(root.path().join("tac-media.js"), media).expect("media");
        std::fs::write(root.path().join("drive.mjs"), Self::DRIVER).expect("driver");
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
}

#[test]
fn the_page_runtime_sends_receives_measures_and_hears_tracks_on_a_peer_connection() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = CallsPageDriver::run();
    assert_eq!(
        report["hooked"],
        serde_json::json!([1, 1]),
        "Media watches every connection Peers opens and closes"
    );
    assert_eq!(
        report["sending"],
        serde_json::json!([["MediaSending", {"connection": "peer-1", "track": "mic"}], ["MediaSending", {"connection": "peer-1", "track": "cam"}]])
    );
    assert_eq!(
        report["options"],
        serde_json::json!(["sendrecv", null, [
            {"rid": "q", "scaleResolutionDownBy": 4, "maxBitrate": 150_000},
            {"rid": "h", "scaleResolutionDownBy": 2, "maxBitrate": 500_000},
            {"rid": "f", "scaleResolutionDownBy": 1, "maxBitrate": 1_500_000}
        ], true]),
        "simulcast sends three layers, and tracks with the same stream name share one stream"
    );
    assert_eq!(
        report["refused"],
        serde_json::json!([
            "The track mic is already sent on peer-1 (exists).",
            "There is no peer connection named peer-9 (unknown).",
            "There is no track named nothing (unknown).",
            "A track name is 1 to 64 letters, digits, \".\", \"_\" or \"-\" (invalid)."
        ])
    );
    assert_eq!(
        report["cameraOff"],
        serde_json::Value::Null,
        "camera off sends no frames"
    );
    assert_eq!(
        report["cameraBack"],
        serde_json::json!(["video", ["MediaTrackStarted"]]),
        "camera on sends the new camera track on the same sender"
    );
    assert_eq!(report["replaced"], serde_json::json!([true, true, false]));
    assert_eq!(
        report["replaceRefused"],
        "A track can only replace a track of the same kind (invalid)."
    );
    assert_eq!(
        report["unsent"],
        serde_json::json!([1, true, "The track mic is not sent on peer-1 (unknown)."])
    );
    assert_eq!(
        report["arrived"],
        serde_json::json!([]),
        "remote events go to the last service that named the connection"
    );
    assert_eq!(
        report["arrivedToCaller"],
        serde_json::json!([
            ["MediaTrackArrived", {"connection": "peer-1", "track": "peer-1.5", "kind": "audio", "stream": "alice"}],
            ["MediaTrackArrived", {"connection": "peer-1", "track": "peer-1.6", "kind": "video", "stream": ""}]
        ])
    );
    assert_eq!(
        report["remoteTracks"],
        serde_json::json!([true, true]),
        "media-view finds remote tracks by name"
    );
    assert_eq!(
        report["players"],
        serde_json::json!([[true, 1, true]]),
        "only remote audio gets a hidden player"
    );
    assert_eq!(
        report["muted"],
        serde_json::json!([
            ["MediaTrackMuted", {"track": "peer-1.5", "muted": true}],
            ["MediaTrackMuted", {"track": "peer-1.5", "muted": false}]
        ])
    );
    assert_eq!(
        report["blocked"],
        serde_json::json!([
            ["MediaTrackArrived", {"connection": "peer-1", "track": "peer-1.7", "kind": "audio", "stream": "bob"}],
            ["MediaAudioBlocked", {}]
        ])
    );
    assert_eq!(
        report["resumed"],
        serde_json::json!([2, []]),
        "resumeAudio plays the blocked audio again"
    );
    assert_eq!(
        report["speaker"],
        serde_json::json!([["spk-2", "spk-2"], [["MediaSpeakerChosen", {"device": "spk-2"}]]]),
        "the chosen speaker moves every remote audio player"
    );
    assert_calls_are_measured_heard_and_closed(&report);
}

fn assert_calls_are_measured_heard_and_closed(report: &serde_json::Value) {
    let stat = |request: &str,
                track: &str,
                direction: &str,
                kind: &str,
                numbers: [u64; 9],
                layer: &str,
                index: u8| {
        serde_json::json!(["MediaStats", {
            "request": request, "connection": "peer-1", "track": track, "layer": layer, "direction": direction, "kind": kind,
            "bitrate": numbers[0], "packetsLost": numbers[1], "lossPercent": numbers[2], "jitterMs": numbers[3],
            "roundTripMs": numbers[4], "framesPerSecond": numbers[5], "width": numbers[6], "height": numbers[7],
            "index": index, "count": numbers[8]
        }])
    };
    assert_eq!(
        report["stats"],
        serde_json::json!([
            stat(
                "r1",
                "cam2",
                "send",
                "video",
                [0, 3, 2, 5, 41, 30, 640, 360, 2],
                "h",
                0
            ),
            stat(
                "r1",
                "peer-1.5",
                "receive",
                "audio",
                [0, 2, 2, 12, 30, 0, 0, 0, 2],
                "",
                1
            ),
            stat(
                "r2",
                "cam2",
                "send",
                "video",
                [500_000, 3, 2, 5, 41, 30, 640, 360, 2],
                "h",
                0
            ),
            stat(
                "r2",
                "peer-1.5",
                "receive",
                "audio",
                [32_000, 2, 2, 12, 30, 0, 0, 0, 2],
                "",
                1
            )
        ]),
        "one row per known track and layer; bitrate from the change since the last call"
    );
    assert_eq!(
        report["noStats"],
        serde_json::json!([stat("r3", "", "", "", [0; 9], "", 0)])
    );
    assert_eq!(
        report["notYet"], 0,
        "a track leaves only when negotiation is stable again"
    );
    assert_eq!(
        report["gone"],
        serde_json::json!([[["MediaTrackEnded", {"track": "peer-1.5", "reason": "remote"}]], null, null, true])
    );
    assert_eq!(
        report["speakingTicks"],
        serde_json::json!([
            [],
            [["MediaSpeaking", {"track": "peer-1.7", "speaking": true}]],
            [["MediaLoudest", {"track": "peer-1.7"}]],
            [["MediaSpeaking", {"track": "peer-1.8", "speaking": true}]],
            [["MediaLoudest", {"track": "peer-1.8"}]],
            [],
            [],
            [["MediaSpeaking", {"track": "peer-1.7", "speaking": false}]],
            []
        ]),
        "talking starts after 2 loud samples and stops after 4 quiet ones; the loudest holds for 2 samples"
    );
    assert_eq!(
        report["closed"],
        serde_json::json!([[
            ["MediaTrackEnded", {"track": "peer-1.6", "reason": "closed"}],
            ["MediaTrackEnded", {"track": "peer-1.7", "reason": "closed"}],
            ["MediaTrackEnded", {"track": "peer-1.8", "reason": "closed"}]
        ], 0, null]),
        "closing the connection ends its remote tracks and forgets its senders"
    );
}

struct FramesPageDriver;

impl FramesPageDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
globalThis.document = { querySelector: () => ({ content: 'camera crypto' }), addEventListener: () => {} }
globalThis.addEventListener = () => {}
const posted = []
class FakeWorker extends EventTarget { constructor(url) { super(); this.url = url; FakeWorker.made.push(url); FakeWorker.last = this } postMessage(message) { posted.push(message) } }
FakeWorker.made = []
globalThis.Worker = FakeWorker
class FakeTransform { constructor(worker, options) { this.worker = worker; this.options = options } }
globalThis.RTCRtpScriptTransform = FakeTransform
globalThis.RTCRtpReceiver = { getCapabilities: () => ({ codecs: [{ mimeType: 'video/H264' }, { mimeType: 'video/VP8' }, { mimeType: 'video/rtx' }] }) }
const transceiver = (kind, mid) => ({ mid, preferred: null, sender: { transform: 'none' }, receiver: { transform: 'none', track: Object.assign(new EventTarget(), { kind }) }, setCodecPreferences(codecs) { this.preferred = codecs.map((codec) => codec.mimeType) } })
const peer = new EventTarget()
peer.list = [transceiver('video', '0'), transceiver('audio', '1')]
peer.getTransceivers = () => peer.list
globalThis[Symbol.for('tachyon.tac.peers.hooks')] = { connection: (id) => id === 'peer-1' ? peer : null, opened: () => {}, closed: () => {} }
;(0, eval)(await readFile('./tac-media.js', 'utf8'))
const media = globalThis[Symbol.for('tachyon.tac.media')]
const sent = []
const reply = (event) => sent.push(event)
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const attempt = (work) => { try { work(); return 'ok' } catch (error) { return error.message } }
const report = {}
report.invalid = [attempt(() => media.useFrameKey(reply, -1, 'alice')), attempt(() => media.sendFrameKey(reply, 1.5)), attempt(() => media.forgetFrameKey(reply, 2 ** 53))]
media.useFrameKey(reply, 4, 'alice')
await settle()
report.noCrypto = sent.splice(0)
const keys = { alice: { name: 'alice-key' } }
globalThis[Symbol.for('tachyon.tac.crypto.hooks')] = { key: async (name) => keys[name] ?? null }
media.useFrameKey(reply, 5, 'nobody')
media.useFrameKey(reply, 6, 'alice')
await settle()
report.unknownKey = sent.splice(0)
report.workers = FakeWorker.made
report.posted = posted.splice(0)
media.sendFrameKey(reply, 6)
media.forgetFrameKey(reply, 4)
report.keyMessages = posted.splice(0)
report.unknownConnection = attempt(() => media.encryptFrames(reply, 'peer-9', true))
media.encryptFrames(reply, 'peer-1', true)
const video = peer.list[0]
const audio = peer.list[1]
report.attached = [video.sender.transform.options, video.receiver.transform.options, audio.receiver.transform.options, video.preferred, audio.preferred]
const later = transceiver('video', '2')
const event = new Event('track')
event.transceiver = later
peer.dispatchEvent(event)
report.later = later.receiver.transform.options
media.encryptFrames(reply, 'peer-1', false)
report.detached = [video.sender.transform, video.receiver.transform]
report.workerCount = FakeWorker.made.length
const emit = (data) => { const item = new Event('message'); item.data = data; FakeWorker.last.dispatchEvent(item) }
sent.splice(0)
emit({ kind: 'ready', keyId: 6, track: '' })
emit({ kind: 'rejected', keyId: 0, track: 'peer-1.0' })
emit({ kind: 'rejected', keyId: 0, track: 'peer-1.0' })
emit({ kind: 'rejected', keyId: 0, track: 'peer-1.0' })
emit({ kind: 'missing', keyId: 9, track: 'peer-1.0' })
emit({ kind: 'missing', keyId: 9, track: 'peer-1.0' })
emit({ kind: 'exhausted', keyId: 6, track: '' })
report.workerEvents = sent.splice(0)
delete globalThis.RTCRtpScriptTransform
report.noTransform = attempt(() => media.encryptFrames(reply, 'peer-1', true))
globalThis.RTCRtpScriptTransform = class { constructor(worker, options) { this.options = options } }
globalThis.__tachyonBrowserApiBridge = ['clipboard', 'crypto']
posted.splice(0)
report.nativeKeys = [attempt(() => media.useFrameKey(reply, 1, 'alice')), attempt(() => media.encryptFrames(reply, 'peer-1', true))]
await settle()
report.nativeSent = sent.splice(0)
report.nativePosted = posted.splice(0)
report.nativeTransforms = [video.sender.transform, video.receiver.transform, audio.sender.transform, audio.receiver.transform]
report.nativeOff = attempt(() => media.encryptFrames(reply, 'peer-1', false))
console.log(JSON.stringify(report))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        let [(_, media), _, _] = PageMedia::files();
        std::fs::write(root.path().join("tac-media.js"), media).expect("media");
        std::fs::write(root.path().join("drive.mjs"), Self::DRIVER).expect("driver");
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
}

#[test]
fn the_page_runtime_hands_frame_keys_to_one_worker_and_encrypts_every_transceiver() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = FramesPageDriver::run();
    let invalid = "A frame key id is a whole number from 0 to 9007199254740991 (invalid).";
    assert_eq!(
        report["invalid"],
        serde_json::json!([invalid, invalid, invalid])
    );
    assert_eq!(
        report["noCrypto"],
        serde_json::json!([["MediaFailed", {"track": "", "call": "useFrameKey", "reason": "unsupported", "message": "Frame keys need the crypto browser API."}]])
    );
    assert_eq!(
        report["unknownKey"],
        serde_json::json!([["MediaFailed", {"track": "", "call": "useFrameKey", "reason": "unknown", "message": "There is no Crypto key with that name."}]])
    );
    assert_eq!(
        report["workers"],
        serde_json::json!(["/.tachyon/tac-media-frames.js"])
    );
    assert_eq!(
        report["posted"],
        serde_json::json!([{"kind": "use", "keyId": 6, "key": {"name": "alice-key"}}]),
        "the key object goes to the worker; the page never reads its bytes"
    );
    assert_eq!(
        report["keyMessages"],
        serde_json::json!([{"kind": "send", "keyId": 6}, {"kind": "forget", "keyId": 4}])
    );
    assert_eq!(
        report["unknownConnection"],
        "There is no peer connection named peer-9 (unknown)."
    );
    assert_eq!(
        report["attached"],
        serde_json::json!([
            {"side": "send", "kind": "video"},
            {"side": "receive", "kind": "video", "track": "peer-1.0"},
            {"side": "receive", "kind": "audio", "track": "peer-1.1"},
            ["video/VP8"],
            null
        ])
    );
    assert_eq!(
        report["later"],
        serde_json::json!({"side": "receive", "kind": "video", "track": "peer-1.2"})
    );
    assert_eq!(report["detached"], serde_json::json!([null, null]));
    assert_eq!(report["workerCount"], 1);
    assert_eq!(
        report["workerEvents"],
        serde_json::json!([
            ["MediaFrameKeyReady", {"keyId": 6}],
            ["MediaFrameRejected", {"track": "peer-1.0", "count": 1}],
            ["MediaFrameKeyMissing", {"track": "peer-1.0", "keyId": 9}],
            ["MediaFailed", {"track": "", "call": "sendFrameKey", "reason": "failed", "message": "The send frame key has used every counter value; send a new key."}]
        ]),
        "repeated reports wait 5 seconds"
    );
    assert_eq!(
        report["noTransform"],
        "Frame encryption is not available here (unsupported)."
    );
    let native = "Frame encryption is not available in this app yet: its Crypto keys stay in native code (unsupported).";
    assert_eq!(
        report["nativeKeys"],
        serde_json::json!([native, native]),
        "a native Crypto host keeps keys out of the page, so SFrame refuses at once"
    );
    assert_eq!(report["nativeSent"], serde_json::json!([]));
    assert_eq!(
        report["nativePosted"],
        serde_json::json!([]),
        "no frame key reaches the worker"
    );
    assert_eq!(
        report["nativeTransforms"],
        serde_json::json!([null, null, null, null]),
        "no frame transform is attached, so no frame passes as if it were encrypted"
    );
    assert_eq!(report["nativeOff"], "ok");
}

#[test]
fn the_page_runtime_captures_devices_and_answers_with_media_events() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = MediaDriver::run();
    let started = MediaDriver::started;
    assert_eq!(
        report["supported"],
        serde_json::json!([true, true, true, false, false, false])
    );
    assert_eq!(report["calls"], serde_json::json!(["ok", "ok"]));
    assert_eq!(
        report["started"],
        serde_json::json!([
            started("a", "mic", "audio", "mic-1", "Default"),
            started("a", "cam", "video", "cam-1", "Camera"),
        ])
    );
    assert_eq!(
        report["constraints"],
        serde_json::json!([
            {"audio": {"echoCancellation": true, "noiseSuppression": true, "autoGainControl": true}},
            {"video": {"deviceId": {"exact": "cam-1"}, "width": {"ideal": 640}, "height": {"ideal": 360}}},
        ])
    );
    let device = MediaDriver::device;
    assert_eq!(
        report["devices"],
        serde_json::json!([
            device("default", "microphone", "Default", false, 0),
            device("mic-1", "microphone", "Built-in", true, 1),
            device("cam-1", "camera", "Camera", true, 2),
            device("default", "speaker", "Speaker", true, 3),
        ])
    );
}

#[test]
fn the_page_runtime_mutes_follows_the_default_device_and_ends_tracks() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = MediaDriver::run();
    let started = MediaDriver::started;
    let ended = MediaDriver::ended;
    assert_eq!(report["micEnabled"], false, "a muted microphone stays open");
    assert_eq!(
        report["camMuted"],
        serde_json::json!([null, "ended"]),
        "camera off closes the camera"
    );
    assert_eq!(
        report["camBack"],
        serde_json::json!([true, ["MediaTrackStarted"]])
    );
    assert_eq!(
        report["camGone"],
        serde_json::json!([ended("a", "cam", "failed")])
    );
    assert_eq!(
        report["followed"],
        serde_json::json!([
            [
                ["b", "MediaDevicesChanged", {}],
                started("a", "mic", "audio", "mic-1", "Default"),
            ],
            true,
            "ended",
            false
        ]),
        "a muted track on the default device moves to the new default and stays muted"
    );
    assert_eq!(
        report["stopped"],
        serde_json::json!([[ended("a", "mic", "stopped")], "ended"])
    );
    assert_eq!(
        report["screenEnded"],
        serde_json::json!([ended("a", "share", "ended")])
    );
}

#[test]
fn the_page_runtime_refuses_bad_calls_and_reports_browser_failures() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = MediaDriver::run();
    let failed = MediaDriver::failed;
    let sync = report["sync"].as_array().expect("sync");
    assert!(
        sync[0]
            .as_str()
            .is_some_and(|text| text.contains("(invalid)")),
        "{sync:?}"
    );
    assert!(
        sync[1]
            .as_str()
            .is_some_and(|text| text.contains("not granted")),
        "{sync:?}"
    );
    assert!(
        report["duplicate"]
            .as_str()
            .is_some_and(|text| text.contains("(exists)"))
    );
    assert!(
        report["unknown"]
            .as_str()
            .is_some_and(|text| text.contains("(unknown)"))
    );
    assert!(
        report["noGesture"]
            .as_str()
            .is_some_and(|text| text.contains("needs a user action")),
        "{report}"
    );
    assert!(
        report["notGranted"]
            .as_str()
            .is_some_and(|text| text.contains("\"microphone\"")),
        "{report}"
    );
    assert_eq!(
        report["refused"],
        serde_json::json!([
            failed(
                "c",
                "refused",
                "captureCamera",
                "denied",
                "The person or the system did not allow this device."
            ),
            failed(
                "c",
                "busy",
                "captureCamera",
                "busy",
                "The device is used by another application."
            ),
        ])
    );
    assert_eq!(
        report["screen"],
        serde_json::json!([
            MediaDriver::started("a", "share", "video", "screen", "Screen 1"),
            MediaDriver::started("a", "share.audio", "audio", "screen-audio", "System audio"),
        ])
    );
}

#[test]
fn only_a_build_that_grants_a_media_api_ships_the_media_modules() {
    let root = tempfile::tempdir().expect("project");
    let pages = root.path().join("client/pages");
    std::fs::create_dir_all(&pages).expect("pages");
    std::fs::write(pages.join("tac.html"), "<page-home />").expect("page");
    let component = root.path().join("client/components/page/home");
    std::fs::create_dir_all(&component).expect("component");
    std::fs::write(component.join("tac.html"), "<main>Home</main>").expect("component view");
    let manifest = |grants: &str| {
        std::fs::write(
            root.path().join("client/manifest.json"),
            format!(
                r#"{{"tac":{{"origin":"https://mail.example.com","browserApis":[{grants}]}}}}"#
            ),
        )
        .expect("manifest");
    };
    let built = || {
        let output = tempfile::tempdir().expect("output");
        let project = crate::ProjectDiscovery::discover(root.path()).expect("project");
        let options = super::BuildOptions {
            output_directory: output.path().to_path_buf(),
            incremental: false,
            ..super::BuildOptions::default()
        };
        super::WebCompiler::build_project(&project, &options).expect("build");
        let page = std::fs::read_to_string(output.path().join("index.html")).expect("page");
        let worker = std::fs::read_to_string(output.path().join("tachyon-sw.js")).expect("sw");
        let shipped = PageMedia::files().map(|(path, _)| output.path().join(path).is_file());
        (page, worker, shipped)
    };
    for grant in [r#""microphone""#, r#""screen""#] {
        manifest(grant);
        let (page, worker, shipped) = built();
        let media = page
            .find("/.tachyon/tac-media.js")
            .expect("the page loads the media module");
        let view = page
            .find("/.tachyon/tac-media-view.js")
            .expect("the page loads media-view");
        let runtime = page.find("/.tachyon/tac-client.js").expect("runtime");
        assert!(media < view && view < runtime, "{page}");
        assert_eq!(shipped, [true, true, true], "{grant}");
        assert!(
            worker.contains("tac-media.js")
                && worker.contains("tac-media-view.js")
                && worker.contains("tac-media-frames.js"),
            "{worker}"
        );
    }
    manifest(r#""share""#);
    let (page, _, shipped) = built();
    assert!(
        !page.contains("tac-media"),
        "no grant, no media modules: {page}"
    );
    assert_eq!(shipped, [false, false, false]);
}

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
const call = (owner, name, ...values) => { try { TacWebApis.calling(owner, () => TacWebApis.media[name](...values)); return 'ok' } catch (error) { return error.message } }
const take = () => sent.splice(0)
const tracksApi = globalThis[Symbol.for('tachyon.tac.media.tracks')]
const report = {}
report.supported = ['capture', 'screen', 'speakers', 'frames', 'background', 'nothing'].map((feature) => TacWebApis.media.supported(feature))
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
const transceiver = (kind, mid) => ({ mid, preferred: null, sender: { transform: 'none' }, receiver: { transform: 'none', track: { kind } }, setCodecPreferences(codecs) { this.preferred = codecs.map((codec) => codec.mimeType) } })
const peer = new EventTarget()
peer.list = [transceiver('video', '0'), transceiver('audio', '1')]
peer.getTransceivers = () => peer.list
globalThis[Symbol.for('tachyon.tac.peers.hooks')] = { connection: (id) => id === 'peer-1' ? peer : null }
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
globalThis.__tachyonBrowserApiBridge = ['share', 'notifications', 'crypto']
globalThis.__tachyonNativeFrameKeys = false
posted.splice(0)
report.nativeKeys = [attempt(() => media.useFrameKey(reply, 1, 'alice')), attempt(() => media.encryptFrames(reply, 'peer-1', true))]
await settle()
report.nativeSent = sent.splice(0)
report.nativePosted = posted.splice(0)
report.nativeTransforms = [video.sender.transform, video.receiver.transform, audio.sender.transform, audio.receiver.transform]
report.nativeOff = attempt(() => media.encryptFrames(reply, 'peer-1', false))
globalThis.__tachyonNativeFrameKeys = true
const frameKeyCalls = []
const refusal = (reason) => Object.assign(new Error(reason), { reason })
const nativeAnswers = { nobody: refusal('unknown'), old: refusal('unsupported'), wrongUse: refusal('invalid') }
globalThis[Symbol.for('tachyon.tac.crypto.hooks')] = {
  key: async () => { throw new Error('the page asked for a page key') },
  frameKey: async (name, keyId, publicKey) => {
    frameKeyCalls.push([name, keyId, publicKey])
    if (nativeAnswers[name]) throw nativeAnswers[name]
    return { enc: `enc-${keyId}`, key: `key-${keyId}`, salt: `salt-${keyId}` }
  },
}
posted.splice(0)
sent.splice(0)
report.hostKeys = [attempt(() => media.useFrameKey(reply, 7, 'alice')), attempt(() => media.encryptFrames(reply, 'peer-1', true))]
await settle()
report.hostBind = posted.splice(0)
emit({ kind: 'bound', keyId: 7, publicKey: 'worker-7' })
await settle()
report.hostWrapped = posted.splice(0)
report.hostCalls = frameKeyCalls.splice(0)
report.hostTransforms = [video.sender.transform?.options ?? null, video.receiver.transform?.options ?? null]
emit({ kind: 'ready', keyId: 7, track: '' })
report.hostReady = sent.splice(0)
const handOver = async (keyId, name) => {
  media.useFrameKey(reply, keyId, name)
  await settle()
  emit({ kind: 'bound', keyId, publicKey: `worker-${keyId}` })
  await settle()
  return { sent: sent.splice(0), posted: posted.splice(0).map((message) => message.kind) }
}
report.hostUnknown = await handOver(8, 'nobody')
report.hostOldSystem = await handOver(9, 'old')
report.hostWrongUse = await handOver(10, 'wrongUse')
report.hostWorkerRefused = await handOver(11, 'alice')
emit({ kind: 'refused', keyId: 11, track: '' })
report.hostWorkerRefused.after = sent.splice(0)
media.useFrameKey(reply, 12, 'alice')
await settle()
posted.splice(0)
emit({ kind: 'refused', keyId: 12, track: '' })
await settle()
report.hostBindRefused = { sent: sent.splice(0), posted: posted.splice(0), calls: frameKeyCalls.filter((call) => call[1] === 12) }
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
fn a_native_frame_key_host_hands_wrapped_keys_only_to_the_media_worker() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = FramesPageDriver::run();
    assert_eq!(
        report["hostKeys"],
        serde_json::json!(["ok", "ok"]),
        "{report}"
    );
    assert_eq!(
        report["hostBind"],
        serde_json::json!([{ "kind": "bind", "keyId": 7 }]),
        "the worker makes the key pair; the page never asks for a page key"
    );
    assert_eq!(
        report["hostCalls"],
        serde_json::json!([["alice", 7, "worker-7"]]),
        "the native host gets the worker's public key"
    );
    assert_eq!(
        report["hostWrapped"],
        serde_json::json!([{ "kind": "wrapped", "keyId": 7, "wrapped": { "enc": "enc-7", "key": "key-7", "salt": "salt-7" } }])
    );
    assert_eq!(
        report["hostTransforms"],
        serde_json::json!([{ "side": "send", "kind": "video" }, { "side": "receive", "kind": "video", "track": "peer-1.0" }])
    );
    assert_eq!(
        report["hostReady"],
        serde_json::json!([["MediaFrameKeyReady", { "keyId": 7 }]])
    );
}

#[test]
fn a_native_frame_key_host_fails_closed_when_a_key_cannot_be_handed_over() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = FramesPageDriver::run();
    let unsupported = serde_json::json!(["MediaFailed", { "track": "", "call": "useFrameKey", "reason": "unsupported", "message": "Frame keys need the crypto browser API." }]);
    let unknown = serde_json::json!(["MediaFailed", { "track": "", "call": "useFrameKey", "reason": "unknown", "message": "There is no Crypto key with that name." }]);
    assert_eq!(
        report["hostUnknown"],
        serde_json::json!({ "sent": [unknown], "posted": ["bind"] }),
        "{report}"
    );
    for case in ["hostOldSystem", "hostWrongUse"] {
        assert_eq!(
            report[case],
            serde_json::json!({ "sent": [unsupported], "posted": ["bind"] }),
            "{case}: no wrapped key reaches the worker"
        );
    }
    assert_eq!(report["hostWorkerRefused"]["sent"], serde_json::json!([]));
    assert_eq!(
        report["hostWorkerRefused"]["posted"],
        serde_json::json!(["bind", "wrapped"])
    );
    assert_eq!(
        report["hostWorkerRefused"]["after"],
        serde_json::json!([unsupported])
    );
    assert_eq!(
        report["hostBindRefused"],
        serde_json::json!({ "sent": [unsupported], "posted": [], "calls": [] }),
        "a worker that cannot make its key pair gets no native call"
    );
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

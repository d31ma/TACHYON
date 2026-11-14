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
        let [(_, media), _] = PageMedia::files();
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
        assert_eq!(shipped, [true, true], "{grant}");
        assert!(
            worker.contains("tac-media.js") && worker.contains("tac-media-view.js"),
            "{worker}"
        );
    }
    manifest(r#""share""#);
    let (page, _, shipped) = built();
    assert!(
        !page.contains("tac-media"),
        "no grant, no media modules: {page}"
    );
    assert_eq!(shipped, [false, false]);
}

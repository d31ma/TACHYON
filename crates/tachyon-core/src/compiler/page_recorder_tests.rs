#![allow(clippy::expect_used)]

use super::page_recorder::PageRecorder;
use crate::node_driver::NodeDriver;

struct RecorderDriver;

impl RecorderDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
const log = []
const supportedTypes = new Set(['video/mp4;codecs=avc1,mp4a.40.2', 'audio/mp4;codecs=mp4a.40.2', 'video/webm;codecs=vp8,opus', 'audio/webm;codecs=opus'])
class FakeRecorder extends EventTarget {
  static isTypeSupported(type) { return supportedTypes.has(type) }
  constructor(stream, options) { super(); this.stream = stream; this.mimeType = options.mimeType; this.state = 'inactive'; FakeRecorder.last = this }
  start(slice) { this.state = 'recording'; this.slice = slice }
  pause() { this.state = 'paused' }
  resume() { this.state = 'recording' }
  stop() { this.state = 'inactive'; this.dispatchEvent(new Event('stop')) }
  emit(bytes) { const event = new Event('dataavailable'); event.data = new Blob([new Uint8Array(bytes)]); this.dispatchEvent(event) }
}
globalThis.MediaRecorder = FakeRecorder
class FakeStream { constructor(tracks = []) { this.tracks = tracks } getAudioTracks() { return this.tracks.filter((track) => track.kind === 'audio') } }
globalThis.MediaStream = FakeStream
const sources = []
globalThis.AudioContext = class {
  createMediaStreamDestination() { return { stream: new FakeStream([{ kind: 'audio', id: 'mixed' }]) } }
  createMediaStreamSource(stream) { const source = { track: stream.tracks[0].id, connected: false, connect() { this.connected = true }, disconnect() { this.connected = false } }; sources.push(source); return source }
  close() { return Promise.resolve() }
}
const elements = { video: [], a: [] }
globalThis.document = {
  querySelector: () => ({ content: 'files microphone camera' }),
  createElement: (tag) => {
    if (tag === 'canvas') return { width: 0, height: 0, getContext: () => ({ fillRect() {}, drawImage() {} }), captureStream: () => ({ getVideoTracks: () => [{ kind: 'video', id: 'canvas' }] }) }
    const element = tag === 'a' ? { click() { log.push(['download', this.download, this.href]) } } : { srcObject: null, play() { return Promise.resolve() } }
    elements[tag].push(element)
    return element
  },
}
globalThis.addEventListener = () => {}
globalThis.URL.createObjectURL = () => 'blob:recording'
const tracks = { mic: { kind: 'audio', id: 'mic' }, other: { kind: 'audio', id: 'other' }, late: { kind: 'audio', id: 'late' }, cam: { kind: 'video', id: 'cam' }, cam2: { kind: 'video', id: 'cam2' } }
const ended = []
globalThis[Symbol.for('tachyon.tac.media.tracks')] = { track: (name) => tracks[name] ?? null, ended: (hook) => ended.push(hook) }
const writes = []
let failWrite = null
const writable = () => ({ write: async (chunk) => { if (failWrite) throw failWrite; writes.push([chunk.position, chunk.data instanceof Blob ? [...new Uint8Array(await chunk.data.arrayBuffer())] : [...chunk.data]]) }, close: async () => { log.push(['closed']) } })
let pickerAnswer = null
globalThis.showSaveFilePicker = async (options) => { log.push(['picker', options.suggestedName, Object.keys(options.types[0].accept)[0]]); if (pickerAnswer) throw pickerAnswer; return { createWritable: async () => writable() } }
const Recorder = (0, eval)((await readFile('./tac-recorder.js', 'utf8')) + '\nTacRecorder')
const api = globalThis[Symbol.for('tachyon.tac.recorder')]
const sent = []
const reply = (event) => sent.push(event)
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const attempt = (work) => { try { work(); return 'ok' } catch (error) { return error.message } }
const report = {}
report.calls = api.calls
report.supported = api.supported()
report.grant = api.grant()
report.mime = [Recorder.pickMime(true, ''), Recorder.pickMime(true, 'webm'), Recorder.pickMime(false, ''), Recorder.pickMime(true, 'avi')]
report.refused = [
  attempt(() => api.start(reply, 'bad name', 'mic', 'cam', 'Meeting', '')),
  attempt(() => api.start(reply, 'meet', 'mic', 'cam', 'a/b', '')),
  attempt(() => api.start(reply, 'meet', 'mic', 'cam', '', '')),
]
api.start(reply, 'meet', 'mic other', 'cam', 'Meeting', '')
report.oneAtATime = attempt(() => api.start(reply, 'second', 'mic', '', 'Other', ''))
await settle()
report.started = sent.splice(0)
report.picker = log.splice(0)
report.mixed = sources.map((source) => [source.track, source.connected])
report.recorder = [FakeRecorder.last.mimeType, FakeRecorder.last.slice, FakeRecorder.last.stream.tracks.map((track) => track.id)]
api.addAudio(reply, 'meet', 'late')
api.removeAudio(reply, 'meet', 'other')
report.afterChange = sources.map((source) => [source.track, source.connected])
api.showVideo(reply, 'meet', 'cam2')
report.shown = elements.video[0].srcObject.tracks[0].id
report.unknown = [attempt(() => api.showVideo(reply, 'meet', 'mic')), attempt(() => api.addAudio(reply, 'meet', 'nobody')), attempt(() => api.pause(reply, 'other'))]
FakeRecorder.last.emit([1, 2, 3])
FakeRecorder.last.emit([4, 5])
FakeRecorder.last.emit([])
await settle()
Recorder.progress(Recorder.current)
report.writes = writes.splice(0)
report.progress = sent.splice(0)
api.pause(reply, 'meet')
api.resume(reply, 'meet')
report.paused = sent.splice(0)
for (const hook of ended) hook('mic')
for (const hook of ended) hook('cam2')
report.trackEnded = [sources.find((source) => source.track === 'mic').connected, elements.video[0].srcObject]
api.stop(reply, 'meet')
await settle()
report.stopped = [sent.splice(0).map((event) => [event[0], event[1].recording, event[1].bytes]), log.splice(0), Recorder.current]
supportedTypes.delete('video/mp4;codecs=avc1,mp4a.40.2')
supportedTypes.delete('audio/mp4;codecs=mp4a.40.2')
api.start(reply, 'webm', 'mic', '', 'Talk', '')
await settle()
report.webmStarted = sent.splice(0)
log.splice(0)
const header = [0x1a, 0x45, 0xdf, 0xa3, 0x80, 0x18, 0x53, 0x80, 0x67, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x15, 0x49, 0xa9, 0x66, 0x84, 0x2a, 0xd7, 0xb1, 0x83, 0x1f, 0x43, 0xb6, 0x75]
FakeRecorder.last.emit(header)
FakeRecorder.last.emit([9, 9])
await settle()
api.stop(reply, 'webm')
await settle()
report.webmWrites = writes.splice(0)
sent.splice(0)
api.start(reply, 'firefox', 'mic', '', 'Kept', '')
await settle()
const withDuration = [0x1a, 0x45, 0xdf, 0xa3, 0x80, 0x18, 0x53, 0x80, 0x67, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x15, 0x49, 0xa9, 0x66, 0x8f, 0x2a, 0xd7, 0xb1, 0x83, 0x44, 0x89, 0x88, 0, 0, 0, 0, 0, 0, 0, 0, 0x1f, 0x43, 0xb6, 0x75]
FakeRecorder.last.emit(withDuration)
await settle()
api.stop(reply, 'firefox')
await settle()
report.keptDuration = writes.splice(0).map(([position, bytes]) => [position, bytes.length])
sent.splice(0)
failWrite = Object.assign(new Error('no space'), { name: 'QuotaExceededError' })
api.start(reply, 'full', 'mic', '', 'Full', '')
await settle()
FakeRecorder.last.emit([1])
await settle()
report.full = [sent.splice(0).map((event) => event[0] === 'RecordingFailed' ? event[1] : event[0]), Recorder.current]
failWrite = null
pickerAnswer = Object.assign(new Error('closed'), { name: 'AbortError' })
api.start(reply, 'gone', 'mic', '', 'Gone', '')
await settle()
report.cancelled = sent.splice(0)
pickerAnswer = null
delete globalThis.showSaveFilePicker
log.splice(0)
const files = []
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { storage: { getDirectory: async () => ({
  removeEntry: async (name) => { log.push(['removed', name]) },
  getDirectoryHandle: async (name) => ({ getFileHandle: async (file) => { files.push([name, file]); return { createWritable: async () => writable(), getFile: async () => new Blob(['x']) } } }),
}) } } })
api.start(reply, 'private', 'mic', '', 'Notes', '')
await settle()
FakeRecorder.last.emit([7])
await settle()
api.stop(reply, 'private')
await settle()
report.private = [files, log.splice(0), sent.splice(0).map((event) => event[0])]
Object.defineProperty(globalThis, 'navigator', { configurable: true, value: {} })
report.noSink = [api.supported(), attempt(() => api.start(reply, 'none', 'mic', '', 'None', ''))]
console.log(JSON.stringify(report))
";

    fn run() -> serde_json::Value {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("tac-recorder.js"), PageRecorder::script())
            .expect("module");
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
fn the_recorder_mixes_switches_writes_pieces_and_stops() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let report = RecorderDriver::run();
    assert_calls_formats_and_refusals(&report);
    assert_audio_video_pieces_and_stop(&report);
    assert_webm_duration_and_other_sinks(&report);
}

fn assert_calls_formats_and_refusals(report: &serde_json::Value) {
    assert_eq!(
        report["calls"],
        serde_json::json!([
            "start",
            "addAudio",
            "removeAudio",
            "showVideo",
            "pause",
            "resume",
            "stop"
        ])
    );
    assert_eq!(report["supported"], true);
    assert_eq!(
        report["grant"], "microphone",
        "files is granted, so the media grant is checked"
    );
    assert_eq!(
        report["mime"],
        serde_json::json!([
            "video/mp4;codecs=avc1,mp4a.40.2",
            "video/webm;codecs=vp8,opus",
            "audio/mp4;codecs=mp4a.40.2",
            "video/mp4;codecs=avc1,mp4a.40.2"
        ]),
        "the app's format first, then MP4, then WebM; always a type the browser accepts"
    );
    let bad_file = "A file name is 1 to 128 characters without / \\ : * ? \" < > | (invalid).";
    assert_eq!(
        report["refused"],
        serde_json::json!([
            "A recording name is 1 to 64 letters, digits, \".\", \"_\" or \"-\" (invalid).",
            bad_file,
            bad_file
        ])
    );
    assert_eq!(
        report["oneAtATime"],
        "A page makes one recording at a time (invalid)."
    );
}

fn assert_audio_video_pieces_and_stop(report: &serde_json::Value) {
    assert_eq!(
        report["started"],
        serde_json::json!([["RecordingStarted", {"recording": "meet", "format": "mp4", "fileName": "Meeting.mp4"}]])
    );
    assert_eq!(
        report["picker"],
        serde_json::json!([["picker", "Meeting.mp4", "video/mp4"]])
    );
    assert_eq!(
        report["mixed"],
        serde_json::json!([["mic", true], ["other", true]])
    );
    assert_eq!(
        report["recorder"],
        serde_json::json!(["video/mp4;codecs=avc1,mp4a.40.2", 1000, ["canvas", "mixed"]]),
        "one canvas video and one mixed audio track, a piece every second"
    );
    assert_eq!(
        report["afterChange"],
        serde_json::json!([["mic", true], ["other", false], ["late", true]])
    );
    assert_eq!(report["shown"], "cam2");
    assert_eq!(
        report["unknown"],
        serde_json::json!([
            "There is no video track named mic (unknown).",
            "There is no audio track named nobody (unknown).",
            "There is no recording named other (unknown)."
        ])
    );
    assert_eq!(
        report["writes"],
        serde_json::json!([[0, [1, 2, 3]], [3, [4, 5]]]),
        "each piece goes to the file at once, at its place; an empty piece writes nothing"
    );
    assert_eq!(
        report["progress"],
        serde_json::json!([["RecordingProgress", {"recording": "meet", "bytes": 5, "seconds": 0}]])
    );
    assert_eq!(
        report["paused"],
        serde_json::json!([["RecordingPaused", {"recording": "meet"}], ["RecordingResumed", {"recording": "meet"}]])
    );
    assert_eq!(
        report["trackEnded"],
        serde_json::json!([false, null]),
        "an ended track leaves the mix; an ended video shows black"
    );
    assert_eq!(
        report["stopped"],
        serde_json::json!([[["RecordingStopped", "meet", 5]], [["closed"]], null])
    );
}

fn assert_webm_duration_and_other_sinks(report: &serde_json::Value) {
    assert_eq!(
        report["webmStarted"],
        serde_json::json!([["RecordingStarted", {"recording": "webm", "format": "webm", "fileName": "Talk.webm"}]])
    );
    assert_eq!(
        report["webmWrites"],
        serde_json::json!([
            [
                0,
                [
                    0x1a, 0x45, 0xdf, 0xa3, 0x80, 0x18, 0x53, 0x80, 0x67, 0x01, 0xff, 0xff, 0xff,
                    0xff, 0xff, 0xff, 0xff, 0x15, 0x49, 0xa9, 0x66, 0x8f, 0x2a, 0xd7, 0xb1, 0x83,
                    0x44, 0x89, 0x88, 0, 0, 0, 0, 0, 0, 0, 0, 0x1f, 0x43, 0xb6, 0x75
                ]
            ],
            [41, [9, 9]],
            [29, [0, 0, 0, 0, 0, 0, 0, 0]]
        ]),
        "the WebM Info element gets an 8-byte Duration, written with the real length at stop"
    );
    assert_eq!(
        report["keptDuration"],
        serde_json::json!([[0, 41], [29, 8]]),
        "a WebM file that already has a Duration (Firefox) is kept as it is, and its value is written at stop"
    );
    assert_eq!(
        report["full"],
        serde_json::json!([[
            "RecordingStarted",
            {"recording": "full", "call": "record", "reason": "full", "message": "There is no disk space left for the recording."}
        ], null])
    );
    assert_eq!(
        report["cancelled"],
        serde_json::json!([["RecordingFailed", {"recording": "gone", "call": "start", "reason": "cancelled", "message": "The save dialog was closed."}]])
    );
    assert_eq!(
        report["private"],
        serde_json::json!([
            [["tachyon-recordings", "Notes.webm"]],
            [
                ["removed", "tachyon-recordings"],
                ["closed"],
                ["download", "Notes.webm", "blob:recording"]
            ],
            ["RecordingStarted", "RecordingStopped"]
        ]),
        "without a save picker the pieces go to a private file, downloaded at stop"
    );
    assert_eq!(
        report["noSink"],
        serde_json::json!([false, "Recording is not available here (unsupported)."])
    );
}

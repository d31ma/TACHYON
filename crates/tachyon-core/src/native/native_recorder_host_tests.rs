#![allow(clippy::expect_used)]

use super::native_browser_api_host_tests::BrowserApiHostTests;

struct RecorderHostTests;

impl RecorderHostTests {
    const FIXTURE: &str = include_str!("NativeRecorderCheck.fixture.swift");
    const GRANTS: [&str; 2] = ["files", "microphone"];
    #[cfg(target_os = "macos")]
    const PAGES: &str = "BrowserApiCheck.app/Contents/Resources/WebBundle";
    #[cfg(target_os = "macos")]
    const INDEX: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Home</title>
<meta name="tachyon-browser-apis" content="files microphone"></head><body>
<script src="recorder-page.js"></script><script src="tac-recorder.js"></script></body></html>"#;
    #[cfg(target_os = "macos")]
    const PAGE_SCRIPT: &str = r"window.events = []
const canvas = document.createElement('canvas')
canvas.width = 320
canvas.height = 240
const paint = canvas.getContext('2d')
let frame = 0
setInterval(() => { frame += 1; paint.fillStyle = frame % 2 === 0 ? '#c33' : '#33c'; paint.fillRect(0, 0, 320, 240) }, 33)
const sound = new AudioContext()
const tone = sound.createOscillator()
const output = sound.createMediaStreamDestination()
tone.connect(output)
tone.start()
const fakeTracks = { tone: output.stream.getAudioTracks()[0], picture: canvas.captureStream(30).getVideoTracks()[0] }
globalThis[Symbol.for('tachyon.tac.media.tracks')] = { track: (name) => fakeTracks[name] ?? null, ended: () => {} }
const recorder = () => globalThis[Symbol.for('tachyon.tac.recorder')]
const remember = (event) => window.events.push(event)
window.record = (name, file = 'Fixture') => recorder().start(remember, name, 'tone', 'picture', file, 'mp4')
window.stopRecording = (name) => recorder().stop(remember, name)
";

    fn host() -> String {
        BrowserApiHostTests::host(&Self::GRANTS, Self::FIXTURE)
    }

    #[cfg(target_os = "macos")]
    fn add_pages(root: &std::path::Path) {
        let pages = [
            ("index.html", Self::INDEX),
            ("recorder-page.js", Self::PAGE_SCRIPT),
            (
                "tac-recorder.js",
                include_str!("../template/tac-recorder.js"),
            ),
        ];
        for (name, text) in pages {
            BrowserApiHostTests::write(root, &format!("{}/{name}", Self::PAGES), text);
        }
    }
}

#[test]
fn the_macos_recorder_check_runs_against_the_generated_host() {
    let source = RecorderHostTests::host();
    assert!(source.contains("AppleBrowserApis.shared.granted = [\"files\",\"microphone\"]"));
    assert!(source.contains("final class AppleRecorderFiles"));
    assert!(source.contains("case \"browserApi.recorder.write\": reply(write(request))"));
    assert!(source.contains("private final class RecorderCheck"));
    assert_eq!(source.matches("\n@main\n").count(), 1);
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires an actual AppKit/WebKit window and its save dialog; run with mac-heavy --gui"]
async fn macos_records_a_call_to_the_file_picked_in_the_save_dialog() {
    let root = tempfile::tempdir().expect("fixture");
    let executable = BrowserApiHostTests::compile(root.path(), &RecorderHostTests::host()).await;
    RecorderHostTests::add_pages(root.path());
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[]).await;
    let progress = std::fs::read_to_string(root.path().join("progress.txt")).unwrap_or_default();
    let output = output.unwrap_or_else(|failure| panic!("{failure:?}\nprogress:\n{progress}"));
    println!("{output}");
    assert!(output.contains("closed dialog: cancelled"), "{output}");
    assert!(
        output.contains("bytes of MP4 through the save dialog"),
        "{output}"
    );
    assert!(output.contains("recorder host check passed"), "{output}");
}

#![allow(clippy::expect_used)]

use super::browser_api_permissions::BrowserApiPermissions;
use super::linux_recorder_files::LinuxRecorderFiles;

struct LinuxRecorderTests;

impl LinuxRecorderTests {
    const GRANTS: [&'static str; 2] = ["files", "microphone"];
    const PAGE_SCRIPT: &'static str = r"window.events = []
const canvas = document.createElement('canvas')
canvas.width = 320
canvas.height = 240
const paint = canvas.getContext('2d')
let frame = 0
setInterval(() => { frame += 1; paint.fillStyle = frame % 2 === 0 ? '#c33' : '#33c'; paint.fillRect(0, 0, 320, 240) }, 33)
const microphone = await navigator.mediaDevices.getUserMedia({ audio: true })
const fakeTracks = { voice: microphone.getAudioTracks()[0], picture: canvas.captureStream(30).getVideoTracks()[0] }
globalThis[Symbol.for('tachyon.tac.media.tracks')] = { track: (name) => fakeTracks[name] ?? null, ended: () => {} }
const videoTypes = { mp4: ['video/mp4;codecs=avc1,mp4a.40.2', 'video/mp4'], webm: ['video/webm;codecs=vp8,opus', 'video/webm'] }
window.format = ['mp4', 'webm'].find((name) => videoTypes[name].some((type) => MediaRecorder.isTypeSupported(type))) ?? 'none'
const recorder = () => globalThis[Symbol.for('tachyon.tac.recorder')]
const remember = (event) => window.events.push(event)
window.record = (name) => recorder().start(remember, name, 'voice', 'picture', 'Fixture', window.format)
window.stopRecording = (name) => recorder().stop(remember, name)
window.takeEvent = (name) => {
  const event = window.events.find((item) => item[0] === name || item[0] === 'RecordingFailed')
  window.events = window.events.filter((item) => item !== event)
  return event ? event[0] + ' ' + JSON.stringify(event[1]) : ''
}
return String(typeof window.record) + '|' + globalThis.__tachyonBrowserApiBridge.join(',') + '|' + window.format";

    fn permissions(granted: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    fn host(granted: &[&str]) -> String {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(granted);
        super::linux::c_source(
            &application,
            &super::linux_host_tests::LinuxHostTests::index(),
        )
    }

    fn setup_script() -> String {
        [
            include_str!("../template/tac-recorder.js"),
            Self::PAGE_SCRIPT,
        ]
        .join("\n")
    }

    fn fixture(folder: &str) -> String {
        concat!(
            include_str!("linux_wayland_input.c"),
            include_str!("linux_fixture_keys.c"),
            include_str!("linux_recorder_fixture.c")
        )
        .replace("__RECORDER_FOLDER__", folder)
        .replace(
            "__RECORDER_SETUP__",
            &super::host::c_string_escape(&Self::setup_script()).replace('?', "\\?"),
        )
    }

    #[cfg(target_os = "linux")]
    async fn compile(root: &std::path::Path, folder: &str) -> std::path::PathBuf {
        let source = super::native_menu_action_tests::MenuActionFixture::stage(root);
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(&Self::GRANTS);
        super::linux_host_tests::LinuxHostTests::compile_application(
            root,
            &source,
            &application,
            &Self::fixture(folder),
        )
        .await
    }
}

#[test]
fn the_linux_host_answers_the_recorder_calls_in_its_main_loop() {
    let source = LinuxRecorderTests::host(&LinuxRecorderTests::GRANTS);
    assert!(source.contains("if (g_str_has_prefix(capability, \"browserApi.recorder.\")) {\n    tachyon_recorder_start(capability, payload, value, reply);"));
    assert!(
        source.contains("tachyon_file_picker_dispose_all();\n    tachyon_recorder_dispose_all();")
    );
    assert!(source.contains("if (strcmp(api, \"files\") == 0) return TRUE;"));
    assert!(source.contains("static gboolean tachyon_recorder_granted(void) {"));
    assert!(!source.contains("__NATIVE_RECORDER__"));
    let refused = LinuxRecorderTests::host(&["microphone"]);
    assert!(!refused.contains("if (strcmp(api, \"files\") == 0) return TRUE;"));
}

#[test]
fn the_linux_recorder_keeps_the_page_limits_and_messages() {
    for line in [
        "#define TACHYON_RECORDER_CHUNK_LIMIT 49152",
        "#define TACHYON_RECORDER_FILE_NAME_LIMIT 128",
        "#define TACHYON_RECORDER_NAME_LIMIT 64",
        "There is no such recording (unknown).",
        "A recording call has a bad argument (invalid).",
        "A save dialog is already open (invalid).",
        "The save dialog was closed (cancelled).",
        "The recording file could not be made there (denied).",
        "The recording could not be written (failed).",
        "There is no disk space left for the recording (full).",
        "return number == ENOSPC || number == EDQUOT ? TACHYON_RECORDER_FULL : TACHYON_RECORDER_FAILED;",
        "ssize_t count = pwrite(file, data + written, length - written, position + (off_t)written);",
        "gboolean saved = fsync(GPOINTER_TO_INT(file)) == 0;",
        "gtk_file_dialog_save(dialog, tachyon_file_picker_parent(), cancellable, tachyon_recorder_saved, opening);",
        "gtk_file_dialog_set_initial_name(dialog, file_name);",
        "g_get_user_special_dir(G_USER_DIRECTORY_VIDEOS)",
        "gtk_file_filter_add_suffix(filter, format);",
    ] {
        assert!(LinuxRecorderFiles::SOURCE.contains(line), "{line}");
    }
    assert_eq!(
        include_str!("../template/tac-recorder.js")
            .matches("static hostChunkBytes = 45 * 1024")
            .count(),
        1
    );
}

#[test]
fn the_recorder_fixture_fills_every_placeholder() {
    let fixture = LinuxRecorderTests::fixture("/tmp/recordings");
    assert!(!fixture.contains("__RECORDER_"));
    assert!(fixture.contains("#define TACHYON_RECORDER_FOLDER \"/tmp/recordings\""));
    assert!(fixture.contains("tachyon_fixture_press(TACHYON_RECORDER_ESCAPE, FALSE);"));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 with GStreamer MediaRecorder, and an X11 display (Xvfb) with libXtst; run explicitly on the Linux lane"]
async fn linux_records_a_call_to_the_file_picked_in_the_gtk_save_dialog() {
    let root = tempfile::tempdir().expect("fixture");
    let folder = tempfile::tempdir().expect("recordings");
    let folder_text = folder.path().to_str().expect("folder").to_owned();
    let executable = LinuxRecorderTests::compile(root.path(), &folder_text).await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host recorder");
    println!("{output}");
    assert!(output.contains("closed dialog: cancelled"), "{output}");
    assert!(output.contains("through the"), "{output}");
    assert!(output.contains("linux recorder passed"), "{output}");
}

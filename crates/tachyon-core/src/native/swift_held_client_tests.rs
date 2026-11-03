#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;
use std::process::Command;

struct HeldClientFixture;

impl HeldClientFixture {
    const FEATURES: &'static str = "../../website/content/features";
    const FILES: [(&'static str, &'static str); 6] = [
        ("sockets", "client/services/ChatService.swift"),
        ("sockets", "client/models/ChatMessage.swift"),
        ("sockets", "client/remotes/ChatClient.swift"),
        ("realtime", "client/services/StreamService.swift"),
        ("realtime", "client/models/Tick.swift"),
        ("realtime", "client/remotes/TicksClient.swift"),
    ];
    const CALLER: &'static str =
        include_str!("../../tests/fixtures/native-swift-held-clients/HeldClientsCaller.swift");

    fn staged(root: &Path) -> std::path::PathBuf {
        for (feature, file) in Self::FILES {
            let from = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(Self::FEATURES)
                .join(feature)
                .join(file);
            let to = root.join(file);
            std::fs::create_dir_all(to.parent().expect("parent")).expect("directory");
            std::fs::copy(from, to).expect("feature file");
        }
        let companions =
            [("chat", "ChatService"), ("stream", "StreamService")].map(|(service, class)| {
                NativeCompanionInput {
                    snapshot_root: root.to_path_buf(),
                    language: NativeCompanion::Swift,
                    source: root.join(format!("client/services/{class}.swift")),
                    service: service.to_owned(),
                    class: class.to_owned(),
                    web_assembly: false,
                    application_provider: false,
                }
            });
        super::macos::stage_swift_companion(&companions, root, Some("https://api.example.com"))
            .expect("stage")
            .expect("a Swift companion")
    }

    fn available() -> bool {
        let installed = Command::new("swiftc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        [cfg!(unix), installed] == [true, true]
    }

    fn run(root: &Path, staged: &Path) -> String {
        let json = root.join("apple_json.swift");
        std::fs::write(&json, super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST).expect("json");
        let caller = root.join("caller.swift");
        std::fs::write(&caller, Self::CALLER).expect("caller");
        let program = root.join("caller");
        let compiled = Command::new("swiftc")
            .arg("-parse-as-library")
            .arg(staged)
            .arg(&json)
            .arg(&caller)
            .arg("-o")
            .arg(&program)
            .output()
            .expect("swiftc");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let output = Command::new(&program).output().expect("the caller runs");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn a_linked_swift_service_describes_its_clients_for_the_page_to_hold() {
    let root = tempfile::tempdir().expect("project");
    let staged = HeldClientFixture::staged(root.path());
    let source = std::fs::read_to_string(staged).expect("companion");
    for wanted in [
        "\"__stream_ChatClient_receive\": .stream(TacStream(\"ChatClient\", \"/chat\", \"ChatMessage\", true,",
        "\"__stream_TicksClient_receive\": .stream(TacStream(\"TicksClient\", \"/ticks\", \"Tick\", false,",
        "try await tacSocketSend(\"ChatClient\", message)",
        "func tacSocketSend<Message: Encodable>(_ client: String, _ message: Message) async throws {",
    ] {
        assert!(source.contains(wanted), "missing {wanted}");
    }
}

#[test]
fn a_linked_swift_service_hears_the_page_and_sends_on_its_socket_through_it() {
    if !HeldClientFixture::available() {
        eprintln!(
            "skipping: swiftc is unavailable, or the host is not Unix (the Swift host links Foundation and pthread)"
        );
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let staged = HeldClientFixture::staged(root.path());
    let printed = HeldClientFixture::run(root.path(), &staged);
    for wanted in [
        "\"client\":\"ChatClient\"",
        "\"path\":\"/chat\"",
        "\"socket\":true",
        "chat event {\"value\":null}",
        "from the page",
        "signals before send 0",
        "\"send\":\"ChatClient\"",
        "\"value\":{\"text\":\"Hello from a tab\"}",
        "\"client\":\"TicksClient\"",
        "\"path\":\"/ticks\"",
        "stream event {\"value\":null}",
        "\"level\":7",
    ] {
        assert!(printed.contains(wanted), "missing {wanted} in {printed}");
    }
}

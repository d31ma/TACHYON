#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct HeldStreamFixture;

impl HeldStreamFixture {
    const FEATURES: &'static str = "../../website/content/features";
    const FILES: [(&'static str, &'static str); 6] = [
        ("sockets", "client/services/ChatService.rs"),
        ("sockets", "client/models/ChatMessage.rs"),
        ("sockets", "client/remotes/ChatClient.rs"),
        ("realtime", "client/services/StreamService.rs"),
        ("realtime", "client/models/Tick.rs"),
        ("realtime", "client/remotes/TicksClient.rs"),
    ];

    fn stage(root: &Path) -> String {
        for (feature, file) in Self::FILES {
            let from = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(Self::FEATURES)
                .join(feature)
                .join(file);
            let to = root.join(file);
            std::fs::create_dir_all(to.parent().expect("parent")).expect("directory");
            std::fs::copy(from, to).expect("feature file");
        }
        let inputs =
            [("chat", "ChatService"), ("stream", "StreamService")].map(|(service, class)| {
                NativeCompanionInput {
                    snapshot_root: root.to_path_buf(),
                    language: NativeCompanion::Rust,
                    source: root.join(format!("client/services/{class}.rs")),
                    service: service.to_owned(),
                    class: class.to_owned(),
                    web_assembly: false,
                    application_provider: false,
                }
            });
        let staged = super::rust::stage(&inputs, root, "org.tachyon.held-streams")
            .expect("staged companion")
            .expect("native source");
        std::fs::read_to_string(staged).expect("companion source")
    }
}

#[test]
fn a_linked_rust_service_hands_its_stream_and_socket_clients_to_the_page() {
    let root = tempfile::tempdir().expect("root");
    let source = HeldStreamFixture::stage(root.path());
    for wanted in [
        r#"TacMember::Stream(TacStream { client: "ChatClient", path: "/chat", event: "ChatMessage", socket: true"#,
        r#"TacMember::Stream(TacStream { client: "TicksClient", path: "/ticks", event: "Tick", socket: false"#,
    ] {
        assert!(
            source.contains(wanted),
            "missing {wanted} in {:?}",
            source
                .lines()
                .filter(|line| line.contains("TacStream {"))
                .collect::<Vec<_>>()
        );
    }
}

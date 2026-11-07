#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};
use std::process::Command;

struct HeldClientFixture;

impl HeldClientFixture {
    const FEATURES: &'static str = "../../website/content/features";
    const FILES: [(&'static str, &'static str); 6] = [
        ("sockets", "client/services/ChatService.kt"),
        ("sockets", "client/models/ChatMessage.kt"),
        ("sockets", "client/remotes/ChatClient.kt"),
        ("realtime", "client/services/StreamService.kt"),
        ("realtime", "client/models/Tick.kt"),
        ("realtime", "client/remotes/TicksClient.kt"),
    ];
    const CALLER: &'static str =
        include_str!("../../tests/fixtures/native-kotlin-held-clients/HeldClientsCaller.kt");
    const PREFERENCES: &'static str =
        include_str!("../../tests/fixtures/native-kotlin-held-clients/SharedPreferences.kt");
    const JSON_JAR: &'static str = "TAC_ORG_JSON_JAR";

    fn staged(root: &Path) -> PathBuf {
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
                    language: NativeCompanion::Kotlin,
                    source: root.join(format!("client/services/{class}.kt")),
                    service: service.to_owned(),
                    class: class.to_owned(),
                    web_assembly: false,
                    application_provider: false,
                }
            });
        let project = root.join("android");
        assert!(
            super::android::stage_companion(&companions, &project, "app.held", "app/held")
                .expect("stage")
        );
        project.join("app/src/main/kotlin")
    }

    fn json_jar() -> Option<PathBuf> {
        let tools = ["kotlinc", "java"].iter().all(|tool| {
            Command::new(tool)
                .arg("-version")
                .output()
                .is_ok_and(|output| output.status.success())
        });
        std::env::var_os(Self::JSON_JAR)
            .map(PathBuf::from)
            .filter(|jar| [tools, jar.is_file()] == [true, true])
    }

    fn run(root: &Path, sources: &Path, json: &Path) -> String {
        std::fs::write(sources.join("HeldClientsCaller.kt"), Self::CALLER).expect("caller");
        std::fs::write(sources.join("SharedPreferences.kt"), Self::PREFERENCES)
            .expect("preferences");
        let jar = root.join("held.jar");
        let compiled = Command::new("kotlinc")
            .arg("-classpath")
            .arg(json)
            .arg(sources)
            .arg("-include-runtime")
            .arg("-d")
            .arg(&jar)
            .output()
            .expect("kotlinc");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let classpath = std::env::join_paths([jar.as_path(), json]).expect("classpath");
        let output = Command::new("java")
            .arg("-cp")
            .arg(classpath)
            .arg("app.held.HeldClientsCallerKt")
            .output()
            .expect("the caller runs");
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

#[test]
fn a_linked_kotlin_service_describes_its_clients_for_the_page_to_hold() {
    let root = tempfile::tempdir().expect("project");
    let sources = HeldClientFixture::staged(root.path());
    let source =
        std::fs::read_to_string(sources.join("app/held/TachyonCompanion.kt")).expect("companion");
    for wanted in [
        "\"__stream_ChatClient_receive\" to TacStream(\"ChatClient\", \"/chat\", \"ChatMessage\", true,",
        "\"__stream_TicksClient_streamTicks\" to TacStream(\"TicksClient\", \"/ticks\", \"Tick\", false,",
        "behaviorSocketSend(\"ChatClient\", message)",
        "private suspend fun behaviorSocketSend(client: String, message: Any) { TacPageSocket.send(client, message) }",
        "        \"Tick\" -> listOf(\"sku\", \"level\")\n",
    ] {
        assert!(source.contains(wanted), "missing {wanted}");
    }
}

#[test]
fn a_linked_kotlin_service_hears_the_page_and_sends_on_its_socket_through_it() {
    let Some(json) = HeldClientFixture::json_jar() else {
        eprintln!("skipping: kotlinc, java or TAC_ORG_JSON_JAR is unavailable");
        return;
    };
    let root = tempfile::tempdir().expect("project");
    let sources = HeldClientFixture::staged(root.path());
    let printed = HeldClientFixture::run(root.path(), &sources, &json);
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

#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;

struct SwiftYonProject;

impl SwiftYonProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/models/Greeting.swift",
            "import Foundation\n\nstruct Greeting: Codable {\n    let message: String\n}\n",
        ),
        (
            "client/remotes/GreetingClient.swift",
            "@HTTPClient(\"/api\")\nprotocol GreetingClient {\n    @GetRequest(\"/greeting\")\n    func getGreeting() async throws -> Greeting\n\n    @PostRequest(\"/greeting\")\n    func saveGreeting(@RequestBody message: String) async throws -> Greeting\n\n    @GetRequest(\"/missing\")\n    func missing() async throws -> Greeting\n}\n",
        ),
        (
            "client/setups/GreetingSetup.swift",
            "@Setup\nfinal class GreetingSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        BehaviorAdapters()\n    }\n}\n",
        ),
        (
            "client/services/GreetingService.swift",
            "@Service\nfinal class GreetingService {\n    @Needs\n    private let greetingClient: GreetingClient\n\n    func ask() async throws -> String {\n        try await greetingClient.getGreeting().message\n    }\n\n    func save() async throws -> String {\n        try await greetingClient.saveGreeting(message: \"from Swift\").message\n    }\n\n    func missing() async -> String {\n        do {\n            return try await greetingClient.missing().message\n        } catch let error as BehaviorError {\n            return error.code\n        } catch {\n            return \"other\"\n        }\n    }\n}\n",
        ),
    ];
    const CALLER: &'static str = "@main\nstruct CompanionCaller {\n    static let request = \"{\\\"route\\\":\\\"/\\\",\\\"owner\\\":\\\"6F9619FF-8B86-D011-B42D-00C04FC964FF\\\",\\\"service\\\":\\\"greeting\\\",\"\n\n    static func main() {\n        print(tacNativeInvoke(request + \"\\\"op\\\":\\\"init\\\"}\"))\n        for name in [\"ask\", \"save\", \"missing\"] {\n            print(tacNativeInvoke(request + \"\\\"op\\\":\\\"call\\\",\\\"name\\\":\\\"\" + name + \"\\\",\\\"args\\\":[]}\"))\n        }\n    }\n}\n";

    fn write(root: &Path, relative: &str, source: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("source");
        path
    }

    fn input(root: &Path) -> NativeCompanionInput {
        for (relative, source) in Self::FILES {
            Self::write(root, relative, source);
        }
        NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Swift,
            source: root.join("client/services/GreetingService.swift"),
            service: String::from("greeting"),
            class: String::from("GreetingService"),
            web_assembly: false,
            application_provider: false,
        }
    }

    fn swift_available() -> bool {
        Command::new("swiftc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn serve(listener: TcpListener) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
        let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let kept = std::sync::Arc::clone(&received);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let request = Self::answer(stream);
                kept.lock().expect("requests").push(request);
            }
        });
        received
    }

    fn answer(mut stream: std::net::TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().expect("reader"));
        let mut head = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).expect("line") > 2 {
            head.push_str(&line);
            line.clear();
        }
        let length = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(|value| value.trim().parse::<usize>().unwrap_or(0))
            })
            .unwrap_or(0);
        let mut body = vec![0; length];
        std::io::Read::read_exact(&mut reader, &mut body).expect("body");
        let received = format!("{head}\n{}", String::from_utf8_lossy(&body));
        let (status, answer) = if head.starts_with("GET /api/missing ") {
            ("404 Not Found", String::from("{}"))
        } else if head.starts_with("POST /api/greeting ") {
            (
                "200 OK",
                format!("{{\"message\":\"Saved {}\"}}", Self::saved(&body)),
            )
        } else {
            ("200 OK", String::from("{\"message\":\"Hi from Yon\"}"))
        };
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
            answer.len()
        )
        .expect("answer");
        received
    }

    fn saved(body: &[u8]) -> String {
        serde_json::from_slice::<serde_json::Value>(body)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
}

#[test]
fn a_linked_swift_http_client_reaches_the_declared_yon_origin() {
    if !SwiftYonProject::swift_available() {
        eprintln!("skipping: swiftc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let input = SwiftYonProject::input(root.path());
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let origin = format!(
        "http://127.0.0.1:{}",
        listener.local_addr().expect("address").port()
    );
    let staged = super::macos::stage_swift_companion(&[input], root.path(), Some(&origin))
        .expect("stage")
        .expect("a Swift companion");
    let json = SwiftYonProject::write(
        root.path(),
        "apple_json.swift",
        super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
    );
    let caller = SwiftYonProject::write(root.path(), "caller.swift", SwiftYonProject::CALLER);
    let program = root.path().join("caller");
    let compiled = Command::new("swiftc")
        .arg("-parse-as-library")
        .args([&staged, &json, &caller])
        .arg("-o")
        .arg(&program)
        .output()
        .expect("swiftc");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let server = SwiftYonProject::serve(listener);
    let output = Command::new(&program).output().expect("the caller runs");
    let printed = String::from_utf8_lossy(&output.stdout);
    let received = server
        .lock()
        .expect("requests")
        .join("\n")
        .to_ascii_lowercase();
    assert!(printed.contains("{\"value\":\"Hi from Yon\"}"), "{printed}");
    assert!(
        printed.contains("{\"value\":\"Saved from Swift\"}"),
        "{printed}"
    );
    assert!(printed.contains("{\"value\":\"HTTP_STATUS\"}"), "{printed}");
    assert!(
        received.contains("get /api/greeting http/1.1"),
        "{received}"
    );
    assert!(
        received.contains("content-type: application/json"),
        "{received}"
    );
    assert!(!received.contains("cookie:"), "{received}");
}

#[tokio::test]
async fn apple_builds_link_the_declared_yon_origin_into_the_swift_companion() {
    for target in [
        tachyon_contracts::NativeTarget::Macos,
        tachyon_contracts::NativeTarget::Ios,
    ] {
        let root = tempfile::tempdir().expect("project");
        SwiftYonProject::input(root.path());
        SwiftYonProject::write(
            root.path(),
            "client/pages/tac.html",
            "<greeting-panel hydrate=\"load\" />\n",
        );
        SwiftYonProject::write(
            root.path(),
            "client/components/greeting/panel/tac.html",
            "<button on:click=\"greeting.ask()\">Ask</button>\n",
        );
        SwiftYonProject::write(
            root.path(),
            "client/manifest.json",
            r#"{"name":"Notes","start_url":"/","tac":{"application":{"id":"com.example.notes","version":"1.0.0"},"yon":{"origin":"https://api.example.com"},"names":{"check":false},"constants":{"check":false},"visibility":{"check":false},"duplication":{"check":false},"cohesion":{"check":false}}}"#,
        );
        let options = super::compiler::NativeBuildOptions {
            output_directory: root.path().join("dist"),
            target,
            package: false,
            ..Default::default()
        };
        let built = super::compiler::NativeCompiler::build_async(root.path(), &options)
            .await
            .unwrap_or_else(|error| panic!("{target:?}: {error}"));
        let companion = built
            .output_directory()
            .join("project/TachyonCompanion.swift");
        let source = std::fs::read_to_string(&companion).expect("companion");
        assert!(
            source.contains("static let origin = \"https://api.example.com\""),
            "{target:?}"
        );
        assert!(
            source.contains("try await BehaviorYonHttp.send(request)"),
            "{target:?}"
        );
    }
}

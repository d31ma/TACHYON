#![allow(clippy::expect_used)]

use super::macos::MacOsHostGenerator;
use super::native_rust_task_tests::NativeRustTaskCheck;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct AppleRustHostCheck;

impl AppleRustHostCheck {
    const HEADER: &'static str = "import Foundation\n#if canImport(FoundationNetworking)\nimport FoundationNetworking\n#endif\n";
    const STUBS: &'static str = r#"
@MainActor
final class NativeBridge {
    static let shared = NativeBridge()

    func handle(_ capability: String, _ payload: String) -> String {
        "{\"ok\":false}"
    }
}

final class ApplePlatformProbe: @unchecked Sendable {
    static let shared = ApplePlatformProbe()
    static let owner = "00000000-0000-4000-8000-000000000084"
    static let wanted = 2
    var heard: [String] = []

    static func request(_ operation: String, _ name: String) -> String {
        "{\"route\":\"/\",\"service\":\"root\",\"owner\":\"" + owner + "\",\"op\":\"" + operation + "\",\"name\":\"" + name + "\",\"args\":[]}"
    }

    func publish(_ payload: String) {
        precondition(Thread.isMainThread, "a Rust task answered off the main thread")
        heard.append(payload)
        print(payload)
        DispatchQueue.main.async { self.next() }
    }

    func next() {
        guard heard.count == Self.wanted else { return print(tacRustInvoke(Self.request("call", "askRefused"))) }
        print(tacRustInvoke(Self.request("get", "message")))
        exit(0)
    }
}

private func tachyonRelayPublish(_ payload: String) {
    ApplePlatformProbe.shared.publish(payload)
}

@main
struct ApplePlatformMain {
    static func main() {
        tacRustNativeSetEmit(tacRustEmit)
        TacRustTasks.install()
        print(tacRustInvoke(ApplePlatformProbe.request("init", "")))
        print(tacRustInvoke(ApplePlatformProbe.request("call", "askServer")))
        DispatchQueue.main.asyncAfter(deadline: .now() + 30) {
            print("timed out after hearing \(ApplePlatformProbe.shared.heard)")
            exit(2)
        }
        while true {
            _ = RunLoop.main.run(mode: .default, before: Date().addingTimeInterval(0.05))
        }
    }
}
"#;

    fn swift_available() -> bool {
        let installed = std::process::Command::new("swiftc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        [cfg!(unix), installed] == [true, true]
    }

    async fn run(mut command: tokio::process::Command, label: &str) -> String {
        let output =
            crate::external_command::run(&mut command, std::time::Duration::from_mins(3), 262_144)
                .await
                .expect(label);
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "{label}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        stdout
    }

    async fn library(source: &Path) -> PathBuf {
        let library = source.with_file_name("libcompanion.a");
        let mut command = tokio::process::Command::new("rustc");
        command
            .args([
                "--edition",
                "2024",
                "--crate-type",
                "staticlib",
                "--crate-name",
                "companion",
            ])
            .arg(source)
            .arg("-o")
            .arg(&library);
        Self::run(command, "Rust staticlib").await;
        library
    }

    async fn host(root: &Path, library: &Path) -> PathBuf {
        let swift = root.join("TachyonHost.swift");
        let program = format!(
            "{}{}{}",
            Self::HEADER,
            MacOsHostGenerator::RUST_SHIM,
            Self::STUBS
        );
        std::fs::write(&swift, program).expect("host source");
        let executable = root.join("apple-host-probe");
        let mut command = tokio::process::Command::new("swiftc");
        command
            .arg("-parse-as-library")
            .arg(&swift)
            .arg(library)
            .args(["-lpthread", "-ldl", "-lm"])
            .arg("-o")
            .arg(&executable);
        Self::run(command, "Swift host").await;
        executable
    }
}

#[tokio::test]
async fn the_apple_host_shim_resumes_rust_tasks_and_sends_their_http_on_the_main_queue() {
    if !AppleRustHostCheck::swift_available() {
        eprintln!(
            "skipping: swiftc is not installed, or the host is not Unix (the Swift host links Foundation and pthread)"
        );
        return;
    }
    let root = tempfile::tempdir().expect("fixture");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let origin = format!("http://{}", listener.local_addr().expect("address"));
    let heads = Arc::new(Mutex::new(Vec::new()));
    let server = tokio::spawn(NativeRustTaskCheck::serve(listener, heads.clone()));
    let source = NativeRustTaskCheck::asking_from(root.path(), &origin);
    let library = AppleRustHostCheck::library(&source).await;
    let executable = AppleRustHostCheck::host(root.path(), &library).await;
    let stdout =
        AppleRustHostCheck::run(tokio::process::Command::new(&executable), "Swift host run").await;
    server.abort();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 6, "{stdout}");
    assert!(!lines[0].contains("error"), "{stdout}");
    assert!(lines[1].starts_with(r#"{"task":"#), "{stdout}");
    assert!(
        lines[2].ends_with(r#""answer":"{\"value\":\"hello from Yon\"}"}"#),
        "{stdout}"
    );
    assert!(lines[3].starts_with(r#"{"task":"#), "{stdout}");
    assert!(lines[4].contains(r#"\"code\":\"HTTP_STATUS\""#), "{stdout}");
    assert_eq!(lines[5], r#"{"value":"hello from Yon"}"#, "{stdout}");
    let heads = heads.lock().expect("heads").clone();
    assert_eq!(heads.len(), 2, "{heads:?}");
    assert!(
        heads
            .iter()
            .all(|head| !head.to_ascii_lowercase().contains("\r\ncookie:")),
        "{heads:?}"
    );
}

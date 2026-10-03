#![allow(clippy::expect_used)]

use crate::project::NativeCompanion;

struct RustCapabilityCheck;

impl RustCapabilityCheck {
    const SERVICE: &str = "#[Service]\n#[NeedsAll]\n#[derive(Default)]\nstruct ImportService {\n    file_picker: FilePicker,\n    window_controls: WindowControls,\n}\nimpl ImportService {\n    fn choose(&self) -> String { self.file_picker.open(&FilePickerOptions::default()).status.as_str().to_owned() }\n    fn allowed(&self) -> bool { self.window_controls.granted(\"minimize\").unwrap_or(false) }\n    fn minimize(&self) -> String { self.window_controls.minimize().err().unwrap_or_else(|| String::from(\"unexpected\")) }\n    fn customize(&self) -> String {\n        if self.window_controls.set_title(\"Tachyon owned native DI fixture\").is_err() || self.window_controls.resize(480, 320).is_err() { return String::from(\"failed\"); }\n        String::from(\"customized\")\n    }\n}\n";
    #[cfg(target_os = "macos")]
    const WINDOW_CALLER: &str = r#"@main @MainActor private struct NativeWindowCheck {
    static func main() {
        let application = NSApplication.shared
        application.setActivationPolicy(.regular)
        DispatchQueue.main.async {
            exercise(application)
            application.stop(nil)
        }
        application.run()
    }
    static func exercise(_ application: NSApplication) {
        let window = NSWindow(contentRect: NSRect(x: 80, y: 80, width: 300, height: 200), styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.makeKeyAndOrderFront(nil)
        application.activate(ignoringOtherApps: true)
        NativeBridge.shared.window = window
        tacRustNativeSetCapabilityAdapter(tacRustCapabilityCall, tacRustCapabilityFree)
        let prefix = "{\"route\":\"/\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"service\":\"import\","
        let initialized = tacRustInvoke(prefix + "\"op\":\"init\"}")
        precondition(!initialized.contains("error"), initialized)
        let changed = tacRustInvoke(prefix + "\"op\":\"call\",\"name\":\"customize\",\"args\":[]}")
        precondition(changed.contains("customized"), changed)
        precondition(window.title == "Tachyon owned native DI fixture", "actual title: \(window.title), key window: \(String(describing: application.keyWindow))")
        precondition(Int(window.contentRect(forFrameRect: window.frame).width) == 480, "actual content width: \(window.contentRect(forFrameRect: window.frame).width)")
        precondition(Int(window.contentRect(forFrameRect: window.frame).height) == 320, "actual content height: \(window.contentRect(forFrameRect: window.frame).height)")
        let denied = tacRustInvoke(prefix + "\"op\":\"call\",\"name\":\"allowed\",\"args\":[]}")
        precondition(denied.contains("false"), denied)
        let attempted = tacRustInvoke(prefix + "\"op\":\"call\",\"name\":\"minimize\",\"args\":[]}")
        precondition(attempted.contains("denied"), attempted)
        precondition(!window.isMiniaturized)
        _ = tacRustInvoke(prefix + "\"op\":\"dispose\"}")
        window.close()
        NativeBridge.shared.window = nil
        print("real native Rust service changed its owned macOS window title/size; ungranted minimize denied")
    }
}
"#;
    const CALLER: &str = r#"
fn fake_capability(capability: &str, _: &str) -> String {
    if capability == "window.granted" { String::from("{\"granted\":true}") }
    else { String::from("{\"status\":\"cancelled\"}") }
}
fn main() {
    let prefix = "{\"route\":\"/\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"service\":\"import\",";
    let init = tac_handle(&format!("{prefix}\"op\":\"init\"}}"));
    assert!(!init.contains("error"), "{init}");
    let unsupported = tac_handle(&format!("{prefix}\"op\":\"call\",\"name\":\"choose\",\"args\":[]}}"));
    assert!(unsupported.contains("unsupported"), "{unsupported}");
    tac_handle(&format!("{prefix}\"op\":\"dispose\"}}"));
    NativeCapabilities::register(fake_capability);
    tac_handle(&format!("{prefix}\"op\":\"init\"}}"));
    let chosen = tac_handle(&format!("{prefix}\"op\":\"call\",\"name\":\"choose\",\"args\":[]}}"));
    assert!(chosen.contains("cancelled"), "{chosen}");
    let allowed = tac_handle(&format!("{prefix}\"op\":\"call\",\"name\":\"allowed\",\"args\":[]}}"));
    assert!(allowed.contains("true"), "{allowed}");
    let (picker, scope) = NativeCapabilities::create("6F9619FF-8B86-D011-B42D-00C04FC964FE", FilePicker::default);
    scope.dispose();
    assert_eq!(picker.open(&FilePickerOptions::default()).status, FilePickerStatus::Disposed);
    println!("Rust native providers: unsupported, captured fake, window grants and disposal passed");
}
"#;

    fn stage(root: &std::path::Path) -> std::path::PathBuf {
        let source = root.join("client/services/ImportService.rs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("directory");
        std::fs::write(&source, Self::SERVICE).expect("service");
        let input = super::registry::NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source,
            service: String::from("import"),
            class: String::from("ImportService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "native-capability-test")
            .expect("stage")
            .expect("Rust companion")
    }

    async fn run() -> String {
        let root = tempfile::tempdir().expect("project");
        let staged = Self::stage(root.path());
        let mut program = std::fs::read_to_string(&staged).expect("source");
        program.push_str(Self::CALLER);
        std::fs::write(&staged, program).expect("caller");
        let binary = root.path().join("caller");
        super::host::run_tool(
            "rustc",
            &[
                "--edition=2024",
                &staged.to_string_lossy(),
                "-o",
                &binary.to_string_lossy(),
            ],
        )
        .await
        .expect("compile");
        super::host::run_tool(&binary.to_string_lossy(), &[])
            .await
            .expect("execute")
    }

    #[cfg(target_os = "macos")]
    async fn host_links(exercise_window: bool) -> String {
        let root = tempfile::tempdir().expect("project");
        let staged = Self::stage(root.path());
        let library = root.path().join("companion.a");
        super::rust::compile(&staged, super::rust::Linkage::Static, None, &library)
            .await
            .expect("Rust static library");
        let mut application =
            super::config::NativeApplication::discover(root.path()).expect("application");
        application.window.controls = vec![String::from("title"), String::from("resize")];
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        };
        let mut host = super::macos::swift_source(
            &application,
            super::macos::companion_call(None, Some(&staged)),
            &index,
        );
        if exercise_window {
            host.truncate(host.rfind("@main").expect("host main"));
            host.push_str(Self::WINDOW_CALLER);
        }
        let source = root.path().join("Host.swift");
        std::fs::write(&source, host).expect("host");
        let executable = root.path().join("Host");
        super::host::run_tool(
            "/usr/bin/xcrun",
            &[
                "swiftc",
                "-parse-as-library",
                "-framework",
                "AppKit",
                "-framework",
                "WebKit",
                &source.to_string_lossy(),
                &library.to_string_lossy(),
                "-o",
                &executable.to_string_lossy(),
            ],
        )
        .await
        .expect("link host");
        if exercise_window {
            return super::host::run_tool(&executable.to_string_lossy(), &[])
                .await
                .expect("owned native window");
        }
        String::new()
    }

    async fn wasm_compiles() {
        let root = tempfile::tempdir().expect("project");
        let staged = Self::stage(root.path());
        super::rust::compile(
            &staged,
            super::rust::Linkage::Web(crate::BuildProfile::Development),
            Some("wasm32-unknown-unknown"),
            &root.path().join("companion.wasm"),
        )
        .await
        .expect("existing WASM window API and unsupported picker compile");
    }
}

#[tokio::test]
async fn a_rust_native_service_receives_scoped_capabilities() {
    assert!(RustCapabilityCheck::run().await.contains("disposal passed"));
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn a_rust_native_companion_links_its_capability_adapter_to_macos() {
    assert!(RustCapabilityCheck::host_links(false).await.is_empty());
}

#[tokio::test]
#[cfg(target_os = "macos")]
#[ignore = "requires a connected macOS GUI; opens and closes one owned fixture window"]
async fn a_rust_service_controls_its_real_macos_window_through_di() {
    assert!(
        RustCapabilityCheck::host_links(true)
            .await
            .contains("real native Rust service")
    );
}

#[tokio::test]
async fn a_rust_web_companion_preserves_the_browser_window_provider() {
    RustCapabilityCheck::wasm_compiles().await;
}

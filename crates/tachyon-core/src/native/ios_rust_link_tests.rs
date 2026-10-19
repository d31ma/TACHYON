#![allow(clippy::expect_used)]

use super::ios_destination::IosDestination;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

struct IosRustLinkCheck;

impl IosRustLinkCheck {
    const SERVICE: &str = "#[Service]\n#[derive(Default)]\nstruct TallyService {\n    count: i64,\n}\nimpl TallyService {\n    pub fn doubled(&self) -> i64 { self.count * 2 }\n}\n";

    fn stage(root: &Path) -> PathBuf {
        let source = root.join("client/services/TallyService.rs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("directory");
        std::fs::write(&source, Self::SERVICE).expect("service");
        std::fs::write(root.join("client/manifest.json"), r#"{"name":"Tally"}"#).expect("manifest");
        let input = super::registry::NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source,
            service: String::from("tally"),
            class: String::from("TallyService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "dev.tachyon.ios-rust")
            .expect("stage")
            .expect("Rust companion")
    }

    fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        }
    }

    fn host(call: &str) -> String {
        let root = tempfile::tempdir().expect("project");
        Self::stage(root.path());
        let application =
            super::config::NativeApplication::discover(root.path()).expect("application");
        super::ios::swift_source(&application, call, &Self::index())
    }

    #[cfg(target_os = "macos")]
    async fn link(destination: IosDestination) -> Option<String> {
        let target = destination.rust_target();
        let installed = super::rust_standard_library::RustStandardLibrary::require(target, "test")
            .await
            .is_ok();
        if !installed {
            eprintln!("skipping the iOS link: the {target} standard library is not installed");
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        let staged = Self::stage(root.path());
        let application =
            super::config::NativeApplication::discover(root.path()).expect("application");
        let project = root.path().join("project");
        let library = super::ios_rust_library::IosRustLibrary::compile(
            &application,
            Some(&staged),
            destination,
            &project,
        )
        .await
        .expect("Rust static library for iOS")
        .expect("a Rust service builds a library");
        let call = super::macos::companion_call(None, Some(&staged));
        let source = project.join("TachyonHost.swift");
        std::fs::write(
            &source,
            super::ios::swift_source(&application, call, &Self::index()),
        )
        .expect("host");
        let executable = root.path().join("Tally.app/Tally");
        std::fs::create_dir_all(executable.parent().expect("app")).expect("app folder");
        super::ios::link_swift(&source, &library, destination, &executable)
            .await
            .expect("link the iOS host with the Rust library");
        let linked = std::fs::read(&executable).expect("executable");
        assert!(
            linked
                .windows(b"_tac_native_invoke".len())
                .any(|bytes| bytes == b"_tac_native_invoke"),
            "the app must hold the Rust entry point"
        );
        let platform = super::host::run_tool(
            "/usr/bin/xcrun",
            &["vtool", "-show-build", &executable.to_string_lossy()],
        )
        .await
        .expect("build version");
        Some(platform)
    }
}

#[test]
fn the_ios_host_calls_a_linked_rust_service_through_the_c_abi() {
    let call = super::macos::companion_call(None, Some(Path::new("companion.rs")));
    let source = IosRustLinkCheck::host(call);
    assert!(source.contains("@_silgen_name(\"tac_native_invoke\")"));
    assert!(
        source.contains("case \"companion.invoke\":\n            return tacRustInvoke(payload)")
    );
    assert!(source.contains(
        "tacRustNativeSetEmit(tacRustEmit); tacRustNativeSetCapabilityAdapter(tacRustCapabilityCall, tacRustCapabilityFree)"
    ));
    assert!(source.contains("fileprivate func handle(_ capability: String, _ payload: String)"));
    assert!(!source.contains("__TACHYON_RUST_SHIM__"));
}

#[test]
fn swift_and_rust_services_share_one_ios_app() {
    let call = super::macos::companion_call(
        Some(Path::new("TachyonCompanion.swift")),
        Some(Path::new("companion.rs")),
    );
    let source = IosRustLinkCheck::host(call);
    assert!(source.contains("tacServiceKnown(request[\"service\"] as? String ?? \"\") ? tacNativeInvoke(payload) : tacRustInvoke(payload)"));
    assert!(
        source.contains("TacBridge.emit = tachyonRelayPublish; tacRustNativeSetEmit(tacRustEmit)")
    );
}

#[tokio::test]
async fn a_missing_rust_standard_library_names_the_rustup_command() {
    let failure = super::rust_standard_library::RustStandardLibrary::require(
        "aarch64-tachyon-missing",
        "The iOS app's Rust services",
    )
    .await
    .expect_err("an unknown target has no standard library");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1605);
    assert!(
        diagnostic
            .message
            .starts_with("The iOS app's Rust services needs")
    );
    assert_eq!(
        diagnostic.help.as_deref(),
        Some("Run rustup target add aarch64-tachyon-missing, then build again.")
    );
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn a_rust_service_links_into_the_ios_simulator_app() {
    if let Some(platform) = IosRustLinkCheck::link(IosDestination::SIMULATOR).await {
        assert!(platform.contains("IOSSIMULATOR"), "{platform}");
    }
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn a_rust_service_links_into_the_ios_device_app() {
    if let Some(platform) = IosRustLinkCheck::link(IosDestination::DEVICE).await {
        assert!(platform.contains("platform IOS\n"), "{platform}");
    }
}

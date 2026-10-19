#![allow(clippy::expect_used)]

#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

struct BrowserApiHostTests;

impl BrowserApiHostTests {
    #[cfg(target_os = "macos")]
    const BUNDLE: &str = "BrowserApiCheck.app/Contents";

    #[cfg(target_os = "macos")]
    fn write(root: &Path, path: &str, source: &str) -> PathBuf {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("fixture");
        path
    }

    fn host() -> String {
        let application = super::config::NativeApplication {
            name: String::from("Browser API Check"),
            executable_name: String::from("BrowserApiCheck"),
            application_id: String::from("org.tachyon.browser-api-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: super::native_rust_project::NativeRustProject::default(),
            outgoing: super::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: super::browser_api_permissions::BrowserApiPermissions::declared(
                &[String::from("clipboard")],
                &std::collections::BTreeMap::new(),
            ),
            minimum_system: super::minimum_system::MinimumSystem::default(),
            description: String::new(),
        };
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        };
        let call = super::macos::companion_call(None, None);
        let host = super::macos::swift_source(&application, call, &index);
        let body = host.rsplit_once("\n@main\n").expect("generated main").0;
        format!(
            "{body}\n{}",
            include_str!("NativeBrowserApiCheck.fixture.swift")
        )
    }

    #[cfg(target_os = "macos")]
    async fn compile(root: &Path) -> PathBuf {
        Self::write(
            root,
            &format!("{}/Resources/WebBundle/index.html", Self::BUNDLE),
            "<!doctype html><html><head><title>Home</title></head><body>Home</body></html>",
        );
        Self::write(
            root,
            &format!("{}/Info.plist", Self::BUNDLE),
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>BrowserApiCheck</string><key>CFBundleIdentifier</key><string>org.tachyon.browser-api-check</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",
        );
        let host = Self::write(root, "BrowserApiCheck.swift", &Self::host());
        let executable = root.join(format!("{}/MacOS/BrowserApiCheck", Self::BUNDLE));
        std::fs::create_dir_all(executable.parent().expect("binary directory"))
            .expect("binary directory");
        super::host::run_tool(
            "/usr/bin/xcrun",
            &[
                "swiftc",
                "-parse-as-library",
                "-framework",
                "AppKit",
                "-framework",
                "WebKit",
                host.to_str().expect("host"),
                "-o",
                executable.to_str().expect("binary"),
            ],
        )
        .await
        .expect("actual generated AppDelegate SDK");
        executable
    }
}

#[test]
fn the_macos_browser_api_check_runs_against_the_generated_host() {
    let source = BrowserApiHostTests::host();
    assert!(source.contains("AppleBrowserApis.shared.granted = [\"clipboard\"]"));
    assert!(source.contains("private final class BrowserApiCheck"));
    assert!(source.contains("browserApi.clipboard.writeText"));
    assert_eq!(source.matches("\n@main\n").count(), 1);
    assert!(!source.contains("__NATIVE_BROWSER_API"));
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires an actual AppKit/WebKit window and the general pasteboard; run explicitly in coordinator SDK lane"]
async fn macos_page_calls_reach_the_pasteboard_through_the_browser_api_bridge() {
    let root = tempfile::tempdir().expect("fixture");
    let executable = BrowserApiHostTests::compile(root.path()).await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated host browser API bridge");
    assert!(
        output.contains("clipboard bridge, refusals and page list passed"),
        "{output}"
    );
}

#![allow(clippy::expect_used)]

use super::native_menu_action_tests::MenuActionFixture;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

struct MenuHostTests;

impl MenuHostTests {
    #[cfg(target_os = "macos")]
    const BUNDLE: &str = "MenuActionCheck.app/Contents";

    #[cfg(target_os = "macos")]
    fn write(root: &Path, path: &str, source: &str) -> PathBuf {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("fixture");
        path
    }

    fn host() -> String {
        let application = super::config::NativeApplication {
            name: String::from("Menu Action Check"),
            executable_name: String::from("MenuActionCheck"),
            application_id: String::from("org.tachyon.menu-action-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus: MenuActionFixture::menus(),
            rust: crate::native::native_rust_project::NativeRustProject::default(),
            outgoing: crate::native::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: crate::native::minimum_system::MinimumSystem::default(),
            description: String::new(),
        };
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: [("/", "index.html"), ("/help", "help/index.html")]
                .into_iter()
                .map(|(route, document)| super::routes::NativeRouteIndexEntry {
                    route: route.to_owned(),
                    document: document.to_owned(),
                })
                .collect(),
        };
        let host = super::macos::swift_source(&application, "tacRustInvoke(payload)", &index);
        let body = host.rsplit_once("\n@main\n").expect("generated main").0;
        format!(
            "{body}\n{}",
            include_str!("NativeMenuActionCheck.fixture.swift")
        )
    }

    #[cfg(target_os = "macos")]
    fn pages(root: &Path) {
        let resources = format!("{}/Resources/WebBundle", Self::BUNDLE);
        for (document, title) in [("index.html", "Home"), ("help/index.html", "Help")] {
            Self::write(
                root,
                &format!("{resources}/{document}"),
                &format!(
                    "<!doctype html><html><head><title>{title}</title></head><body>{title}</body></html>"
                ),
            );
        }
        Self::write(
            root,
            &format!("{}/Info.plist", Self::BUNDLE),
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>MenuActionCheck</string><key>CFBundleIdentifier</key><string>org.tachyon.menu-action-check</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",
        );
    }

    #[cfg(target_os = "macos")]
    async fn compile(root: &Path) -> PathBuf {
        let source = MenuActionFixture::stage(root);
        let library = root.join("libmenuactioncheck.a");
        super::rust::compile(&source, super::rust::Linkage::Static, None, &library)
            .await
            .expect("actual Rust SDK");
        Self::pages(root);
        let host = Self::write(root, "MenuActionCheck.swift", &Self::host());
        let executable = root.join(format!("{}/MacOS/MenuActionCheck", Self::BUNDLE));
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
                library.to_str().expect("library"),
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
fn macos_host_owns_close_window_and_reopens_with_fresh_menu_owners() {
    let source = MenuHostTests::host();
    assert!(source.contains(
        "windowItem(\"Close Window\", #selector(NSWindow.performClose(_:)), \"w\", window)"
    ));
    assert!(source.contains("func windowWillClose(_ notification: Notification)"));
    assert!(source.contains("func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {\n        false"));
    assert!(source.contains("func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool"));
    assert!(source.contains("installMenus(nil, nil)"));
    assert!(source.contains("guard message.webView === webView, message.frameInfo.isMainFrame"));
    assert!(source.contains("MacOsMenuAction(id: \"show-running\""));
    assert!(source.contains("@_silgen_name(\"tac_native_menu_ready\")"));
    assert!(!source.contains("__NATIVE_MENU_"));
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires actual AppKit/WebKit owned window and generated AppDelegate menus; run explicitly in coordinator SDK lane"]
async fn macos_menu_actions_reach_the_page_service_and_close_window_reopens_cleanly() {
    let root = tempfile::tempdir().expect("fixture");
    let executable = MenuHostTests::compile(root.path()).await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated host menu lifecycle");
    assert!(
        output.contains("menu action, disabled recipient, Close Window, reopen and Quit passed"),
        "{output}"
    );
}

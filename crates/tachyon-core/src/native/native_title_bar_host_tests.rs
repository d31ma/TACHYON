#![allow(clippy::expect_used)]

use super::native_title_bar_button_inset::NativeTitleBarButtonInset;
use std::path::{Path, PathBuf};

struct TitleBarHostTests;

struct TitleBarVariant {
    name: &'static str,
    button_inset: Option<NativeTitleBarButtonInset>,
    height: i32,
    leading: i32,
}

impl TitleBarHostTests {
    const BUNDLE: &str = "TitleBarCheck.app/Contents";
    const PAGE: &str = r#"<!doctype html><html><head><title>__TITLE__</title><meta name="color-scheme" content="light dark"><style>
:root { color-scheme: light dark; }
body { margin: 0; font: 13px -apple-system, sans-serif; background: light-dark(#ffffff, #1c1c1e); color: light-dark(#1d1d1f, #f5f5f7); }
header { display: flex; align-items: center; justify-content: space-between; box-sizing: border-box; min-height: max(84px, var(--tac-titlebar-height, 0px)); padding-inline-start: max(16px, var(--tac-titlebar-leading-inset, 0px)); padding-inline-end: 16px; background: light-dark(#eef1f5, #26292e); border-bottom: 1px solid light-dark(#d6dbe1, #3a3f45); }
main { padding: 24px; }
#below { position: fixed; right: 16px; bottom: 16px; }
</style></head><body>
<header data-tac-drag-region><strong>Title Bar Check</strong><button id="action">Refresh</button></header>
<main id="main"><h1>__TITLE__</h1><p>The page continues under the window buttons.</p></main>
<button id="below">Below</button>
<script>
globalThis.__downs = []
globalThis.__clicks = 0
addEventListener('mousedown', (event) => globalThis.__downs.push(event.target.closest('[id]')?.id ?? 'none'))
document.getElementById('action').addEventListener('click', () => { globalThis.__clicks += 1 })
</script>
</body></html>"#;

    fn variants() -> [TitleBarVariant; 2] {
        [
            TitleBarVariant {
                name: "inset",
                button_inset: Some(NativeTitleBarButtonInset {
                    leading: 20,
                    top: 35,
                }),
                height: 84,
                leading: 100,
            },
            TitleBarVariant {
                name: "default",
                button_inset: None,
                height: 0,
                leading: 0,
            },
        ]
    }

    fn write(root: &Path, path: &str, source: &str) -> PathBuf {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("fixture");
        path
    }

    fn host(variant: &TitleBarVariant) -> String {
        let application = super::config::NativeApplication {
            name: String::from("Title Bar Check"),
            executable_name: String::from("TitleBarCheck"),
            application_id: String::from("org.tachyon.title-bar-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration {
                width: 900,
                height: 560,
                title_bar: super::native_title_bar::NativeTitleBar::Integrated,
                title_bar_button_inset: variant.button_inset,
                ..super::config::WindowConfiguration::default()
            },
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: crate::native::native_rust_project::NativeRustProject::default(),
            outgoing: crate::native::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: crate::native::minimum_system::MinimumSystem::default(),
            description: String::new(),
            tray: None,
            links: crate::native::native_links::NativeLinks::default(),
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
        let call = super::macos::companion_call(None, None);
        let host = super::macos::swift_source(&application, call, &index);
        let body = host.rsplit_once("\n@main\n").expect("generated main").0;
        let (button_leading, button_top) = variant.button_inset.map_or((-1, -1), |inset| {
            (i64::from(inset.leading), i64::from(inset.top))
        });
        let expectation = format!(
            "enum TitleBarExpectation {{\n    static let variant = \"{}\"\n    static let height = {}\n    static let leading = {}\n    static let buttonLeading = {button_leading}\n    static let buttonTop = {button_top}\n}}\n",
            variant.name, variant.height, variant.leading
        );
        format!(
            "{body}\n{expectation}{}",
            include_str!("NativeTitleBarCheck.fixture.swift")
        )
    }

    fn pages(root: &Path) {
        let resources = format!("{}/Resources/WebBundle", Self::BUNDLE);
        for (document, title) in [("index.html", "Home"), ("help/index.html", "Help")] {
            Self::write(
                root,
                &format!("{resources}/{document}"),
                &Self::PAGE.replace("__TITLE__", title),
            );
        }
        Self::write(
            root,
            &format!("{}/Info.plist", Self::BUNDLE),
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>TitleBarCheck</string><key>CFBundleIdentifier</key><string>org.tachyon.title-bar-check</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",
        );
    }

    async fn compile(root: &Path, variant: &TitleBarVariant) -> PathBuf {
        Self::pages(root);
        let host = Self::write(root, "TitleBarCheck.swift", &Self::host(variant));
        let executable = root.join(format!("{}/MacOS/TitleBarCheck", Self::BUNDLE));
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

#[tokio::test]
#[ignore = "Requires actual AppKit/WebKit owned window, full screen and screen capture; run explicitly on a macOS desktop session"]
async fn macos_integrated_title_bar_keeps_buttons_insets_drag_regions_and_full_screen() {
    for variant in TitleBarHostTests::variants() {
        let root = tempfile::tempdir().expect("fixture");
        let screenshots = std::env::var("TAC_TITLE_BAR_SCREENSHOTS")
            .unwrap_or_else(|_| root.path().display().to_string());
        let executable = TitleBarHostTests::compile(root.path(), &variant).await;
        let output = super::host::run_tool_with(
            executable.to_str().expect("binary"),
            &["-AppleActionOnDoubleClick", "Maximize"],
            None,
            &[("TAC_TITLE_BAR_SCREENSHOTS", screenshots.as_str())],
        )
        .await
        .expect("actual generated host title bar");
        assert!(
            output.contains("integrated title bar style, buttons, insets, light, dark, drag regions, double-click, resize, full screen, title and close passed"),
            "{}: {output}",
            variant.name
        );
        println!("{} title bar output:\n{output}", variant.name);
    }
}

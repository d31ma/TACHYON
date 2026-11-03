#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::native_display::NativeDisplay;
use super::native_theme_color::NativeThemeColor;
use std::path::{Path, PathBuf};

struct WindowAppearanceHostTests;

struct WindowAppearanceVariant {
    name: &'static str,
    window: WindowConfiguration,
}

impl WindowAppearanceHostTests {
    const BUNDLE: &str = "AppearanceCheck.app/Contents";
    const PAGE: &str =
        "<!doctype html><html><head><title>Home</title></head><body><p>Home</p></body></html>";
    const THEME: NativeThemeColor = NativeThemeColor {
        red: 0x1a,
        green: 0x23,
        blue: 0x7e,
    };
    const BACKGROUNDS: [NativeThemeColor; 2] = [
        NativeThemeColor {
            red: 0xfd,
            green: 0xf6,
            blue: 0xe3,
        },
        NativeThemeColor {
            red: 0x10,
            green: 0x60,
            blue: 0x30,
        },
    ];

    fn variants() -> [WindowAppearanceVariant; 2] {
        [
            WindowAppearanceVariant {
                name: "themed",
                window: WindowConfiguration {
                    width: 900,
                    height: 560,
                    theme_color: Some(Self::THEME),
                    background_color: Some(Self::BACKGROUNDS[0]),
                    ..WindowConfiguration::default()
                },
            },
            WindowAppearanceVariant {
                name: "fullscreen",
                window: WindowConfiguration {
                    background_color: Some(Self::BACKGROUNDS[1]),
                    display: NativeDisplay::Fullscreen,
                    ..WindowConfiguration::default()
                },
            },
        ]
    }

    fn channels(color: Option<NativeThemeColor>) -> String {
        let color = color.unwrap_or(Self::THEME);
        format!("[{}, {}, {}]", color.red, color.green, color.blue)
    }

    fn write(root: &Path, path: &str, source: &str) -> PathBuf {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("fixture");
        path
    }

    fn host(variant: &WindowAppearanceVariant) -> String {
        let application = super::config::NativeApplication {
            name: String::from("Appearance Check"),
            executable_name: String::from("AppearanceCheck"),
            application_id: String::from("org.tachyon.window-appearance-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: variant.window.clone(),
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
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        };
        let call = super::macos::companion_call(None, None);
        let host = super::macos::swift_source(&application, call, &index);
        let body = host.rsplit_once("\n@main\n").expect("generated main").0;
        let expectation = format!(
            "enum WindowAppearanceExpectation {{\n    static let fullscreen = {}\n    static let theme: [CGFloat] = {}\n    static let background: [CGFloat] = {}\n}}\n",
            variant.window.display.starts_fullscreen(),
            Self::channels(variant.window.theme_color),
            Self::channels(variant.window.background_color),
        );
        format!(
            "{body}\n{expectation}{}",
            include_str!("NativeWindowAppearanceCheck.fixture.swift")
        )
    }

    async fn compile(root: &Path, variant: &WindowAppearanceVariant) -> PathBuf {
        Self::write(
            root,
            &format!("{}/Resources/WebBundle/index.html", Self::BUNDLE),
            Self::PAGE,
        );
        Self::write(
            root,
            &format!("{}/Info.plist", Self::BUNDLE),
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>AppearanceCheck</string><key>CFBundleIdentifier</key><string>org.tachyon.window-appearance-check</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",
        );
        let host = Self::write(root, "AppearanceCheck.swift", &Self::host(variant));
        let executable = root.join(format!("{}/MacOS/AppearanceCheck", Self::BUNDLE));
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
async fn macos_window_takes_theme_color_background_color_and_fullscreen_display() {
    for variant in WindowAppearanceHostTests::variants() {
        let root = tempfile::tempdir().expect("fixture");
        let screenshots = std::env::var("TAC_WINDOW_APPEARANCE_SCREENSHOTS")
            .unwrap_or_else(|_| root.path().display().to_string());
        let executable = WindowAppearanceHostTests::compile(root.path(), &variant).await;
        let output = super::host::run_tool_with(
            executable.to_str().expect("binary"),
            &[],
            None,
            &[("TAC_WINDOW_APPEARANCE_SCREENSHOTS", screenshots.as_str())],
        )
        .await
        .expect("actual generated host window appearance");
        assert!(
            output.contains("window appearance passed"),
            "{}: {output}",
            variant.name
        );
        println!("{} window appearance output:\n{output}", variant.name);
    }
}

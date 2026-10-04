#![allow(clippy::expect_used)]

use super::native_application_tests::ApplicationLifetimeTests;
use std::path::{Path, PathBuf};

struct ApplicationHostTests;

impl ApplicationHostTests {
    fn write(root: &Path, path: &str, source: &str) -> PathBuf {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("fixture");
        path
    }

    fn host() -> String {
        let application = super::config::NativeApplication {
            name: String::from("Application Lifetime Check"),
            executable_name: String::from("ApplicationCheck"),
            application_id: String::from("org.tachyon.application-check"),
            version: String::from("1.0.0"),
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: crate::native::native_rust_project::NativeRustProject::default(),
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
            include_str!("NativeApplicationCheck.fixture.swift")
        )
    }

    async fn compile(root: &Path) -> PathBuf {
        let source = ApplicationLifetimeTests::stage(
            root,
            ApplicationLifetimeTests::COLLECTOR,
            Some(ApplicationLifetimeTests::READER),
        )
        .expect("staged Rust DI");
        let authored = std::fs::read_to_string(&source).expect("staged source");
        let export = r#"
#[unsafe(no_mangle)]
pub extern "C" fn tac_fixture_application_count(event: *const std::ffi::c_char) -> i64 {
    let event = unsafe { std::ffi::CStr::from_ptr(event) }.to_str().unwrap();
    Probe::count(event) as i64
}
"#;
        std::fs::write(&source, authored + ApplicationLifetimeTests::PROBE + export)
            .expect("test-owned count ABI");
        let library = root.join("libapplicationcheck.a");
        super::rust::compile(&source, super::rust::Linkage::Static, None, &library)
            .await
            .expect("actual Rust SDK");
        let bundle = "ApplicationCheck.app/Contents";
        let resources = format!("{bundle}/Resources/WebBundle");
        Self::write(
            root,
            &format!("{resources}/index.html"),
            "<!doctype html><html><head><title>Home</title></head><body>Home</body></html>",
        );
        Self::write(
            root,
            &format!("{resources}/help/index.html"),
            "<!doctype html><html><head><title>Help</title></head><body>Help</body></html>",
        );
        Self::write(
            root,
            &format!("{bundle}/Info.plist"),
            "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>ApplicationCheck</string><key>CFBundleIdentifier</key><string>org.tachyon.application-check</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",
        );
        let host = Self::write(root, "ApplicationCheck.swift", &Self::host());
        let executable = root.join(format!("{bundle}/MacOS/ApplicationCheck"));
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

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires actual AppKit/WebKit owned window and generated AppDelegate navigation; run explicitly in coordinator SDK lane"]
async fn native_application_lifetime_generated_host_preserves_collector_through_navigation() {
    let root = tempfile::tempdir().expect("fixture");
    let executable = ApplicationHostTests::compile(root.path()).await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated host lifecycle");
    assert!(
        output.contains("Home/Help/Home and joined Quit passed"),
        "{output}"
    );
}

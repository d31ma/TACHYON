#![allow(clippy::expect_used)]

use std::path::Path;

struct MacOsMenuFixture;

impl MacOsMenuFixture {
    const CALLER: &str = include_str!("../../../../scripts/native/MacOsMenuCheck.swift");

    fn host(root: &Path) -> String {
        std::fs::create_dir_all(root.join("client")).expect("client directory");
        std::fs::write(
            root.join("client/manifest.json"),
            r#"{"name":"Menu Check"}"#,
        )
        .expect("real manifest");
        let application =
            super::config::NativeApplication::discover(root).expect("native identity");
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: "/".to_owned(),
            entry_document: "index.html".to_owned(),
            routes: Vec::new(),
        };
        super::macos::swift_source(
            &application,
            super::macos::companion_call(None, None),
            &index,
        )
    }
}

#[test]
fn macos_menus_are_installed_after_the_owned_window_without_changing_bridge_lifetime() {
    let root = tempfile::tempdir().expect("menu project");
    let source = MacOsMenuFixture::host(root.path());
    let ownership = source.find("self.window = window").expect("owned window");
    let install = source
        .find("menus.install(NSApplication.shared, window, \"Menu Check\")")
        .expect("host-owned menu installation");
    assert!(ownership < install);
    assert!(source.contains("private var menus: MacOsMenus?"));
    assert!(source.contains("applicationWillTerminate"));
    assert!(source.contains("tachyonDisposeCompanions()"));
    assert!(source.contains("MacOsFilePicker.shared.disposeAll()"));
    assert!(!source.contains("__NATIVE_MENUS__"));
    assert!(!source.contains("nativeMenus"));
}

#[tokio::test]
async fn macos_menus_compile_and_execute_native_roles_and_responder_actions() {
    let root = tempfile::tempdir().expect("menu SDK project");
    let original = MacOsMenuFixture::host(root.path());
    let (body, _) = original
        .rsplit_once("\n@main\n")
        .expect("generated host entry point");
    let source = root.path().join("MenuCheck.swift");
    std::fs::write(&source, format!("{body}\n{}", MacOsMenuFixture::CALLER))
        .expect("unchanged host with owned test entry");
    let executable = root.path().join("menu-check");
    super::host::run_tool(
        "/usr/bin/xcrun",
        &[
            "swiftc",
            "-parse-as-library",
            "-framework",
            "AppKit",
            "-framework",
            "WebKit",
            source.to_str().expect("Swift source path"),
            "-o",
            executable.to_str().expect("executable path"),
        ],
    )
    .await
    .expect("real generated host/menu Swift compile");
    let output = super::host::run_tool(executable.to_str().expect("executable path"), &[])
        .await
        .expect("real AppKit menu/responder execution");
    assert!(
        output.contains("PASS native macOS menu roles, actions, ownership and text responder"),
        "{output}"
    );
}

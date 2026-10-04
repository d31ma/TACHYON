#![allow(clippy::expect_used)]

use std::path::Path;

struct MacOsMenuFixture;

impl MacOsMenuFixture {
    #[cfg(target_os = "macos")]
    const CALLER: &str = include_str!("../../../../scripts/native/MacOsMenuCheck.swift");
    #[cfg(target_os = "macos")]
    const OWNERSHIP: &str =
        include_str!("../../../../scripts/native/MacOsMenuOwnershipCheck.swift");

    #[cfg(target_os = "macos")]
    fn retained_root() -> std::path::PathBuf {
        let output = std::env::var_os("TAC_NATIVE_MENU_OUT")
            .expect("Set TAC_NATIVE_MENU_OUT to an absolute retained evidence directory");
        let output = std::path::PathBuf::from(output);
        assert!(
            output.is_absolute(),
            "Menu evidence directory must be absolute"
        );
        std::fs::create_dir_all(&output).expect("menu evidence directory");
        let root = tempfile::Builder::new()
            .prefix("native-menu-")
            .tempdir_in(output)
            .expect("fresh owned menu evidence")
            .keep();
        println!("Retained native menu evidence: {}", root.display());
        root
    }

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
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: "/".to_owned(),
                document: "index.html".to_owned(),
            }],
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
        .find("installMenus(view, window)")
        .expect("host-owned menu installation");
    assert!(ownership < install);
    assert!(source.contains(
        "menus.install(NSApplication.shared, window, \"Menu Check\", menuNavigation, menuActions)"
    ));
    assert!(source.contains("private var menus: MacOsMenus?"));
    assert!(source.contains("applicationWillTerminate"));
    assert!(source.contains("tachyonDisposeCompanions()"));
    assert!(source.contains("MacOsFilePicker.shared.disposeAll()"));
    assert!(source.contains("let fullscreen = windowItem(\"Enter Full Screen\", #selector(NSWindow.toggleFullScreen(_:)), \"f\", window)"));
    assert!(source.contains("fullscreen.keyEquivalentModifierMask = [.command, .control]"));
    assert!(!source.contains("__NATIVE_MENUS__"));
    assert!(!source.contains("nativeMenus"));
}

#[tokio::test]
#[cfg(target_os = "macos")]
#[ignore = "Requires explicit AppKit window/focus permission and TAC_NATIVE_MENU_OUT evidence path"]
async fn macos_menus_compile_and_execute_native_roles_and_responder_actions() {
    let root = MacOsMenuFixture::retained_root();
    let original = MacOsMenuFixture::host(&root);
    std::fs::write(root.join("GeneratedHost.swift"), &original).expect("retain ordinary host");
    let (body, _) = original
        .rsplit_once("\n@main\n")
        .expect("generated host entry point");
    let source = root.join("MenuCheck.swift");
    std::fs::write(
        &source,
        format!(
            "{body}\n{}\n{}",
            MacOsMenuFixture::OWNERSHIP,
            MacOsMenuFixture::CALLER
        ),
    )
    .expect("unchanged host with owned test entry");
    let executable = root.join("menu-check");
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

#![allow(clippy::expect_used)]

use super::config::NativeApplication;
use super::native_menu_config::NativeMenuConfig;
use super::native_tray::NativeTray;
use crate::AppTarget;
use serde_json::{Value, json};
use std::path::Path;

struct TrayFixture;

impl TrayFixture {
    const ICON: &'static [u8] =
        include_bytes!("../../../../website/client/shared/assets/logo-192.png");

    fn tray() -> Value {
        json!({
            "icon": "/shared/tray.png",
            "tooltip": "Tray tooltip é",
            "template": true,
            "items": [
                {"id": "dashboard", "label": "Dashboard", "route": "/help"},
                {"id": "tray-refresh", "label": "Refresh", "action": {"service": "SectionMenuService", "method": "refresh"}}
            ],
            "closeHidesWindow": true,
            "dock": false
        })
    }

    fn project(application: &Value) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("tray project");
        std::fs::create_dir_all(root.path().join("client")).expect("client directory");
        let manifest = json!({"name": "Tray Check", "tac": {"application": application}});
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("manifest");
        root
    }

    fn discovered(application: &Value) -> Result<NativeApplication, crate::Failure> {
        let root = Self::project(application);
        NativeApplication::discover(root.path())
    }

    fn refusal(tray: &Value) -> String {
        Self::discovered(&json!({"tray": tray}))
            .expect_err("invalid tray")
            .to_string()
    }

    fn application() -> NativeApplication {
        let mut application =
            super::linux_host_tests::LinuxHostTests::application(Self::menus_with_tray());
        application.tray = Some(serde_json::from_value(Self::tray()).expect("typed tray"));
        application
    }

    fn menus_with_tray() -> NativeMenuConfig {
        let tray: NativeTray = serde_json::from_value(Self::tray()).expect("typed tray");
        NativeMenuConfig {
            tray: tray.items,
            ..super::native_menu_action_tests::MenuActionFixture::menus()
        }
    }

    fn icon_bundle(bytes: &[u8]) -> tempfile::TempDir {
        let bundle = tempfile::tempdir().expect("web bundle");
        std::fs::create_dir_all(bundle.path().join("shared")).expect("shared");
        std::fs::write(bundle.path().join("shared/tray.png"), bytes).expect("icon");
        bundle
    }

    #[cfg(target_os = "linux")]
    fn stage(root: &Path) -> std::path::PathBuf {
        let source = super::rust::stage(
            &super::native_menu_action_tests::MenuActionFixture::inputs(root),
            root,
            "org.tachyon.tray",
        )
        .expect("staged Rust services")
        .expect("native Rust source");
        super::mac_os_menu_action_source::MacOsMenuActionSource::stage(
            Some(&source),
            &Self::menus_with_tray(),
        )
        .expect("menu and tray dispatch");
        let staged = std::fs::read_to_string(&source).expect("staged source");
        std::fs::write(
            &source,
            staged + super::native_menu_action_tests::MenuActionFixture::PROBE,
        )
        .expect("probe");
        source
    }
}

#[test]
fn a_tray_declaration_reaches_the_application_and_its_items_join_the_menu_table() {
    let application = TrayFixture::discovered(&json!({
        "menus": {"file": [{"id": "refresh", "label": "Refresh", "route": "/"}]},
        "tray": TrayFixture::tray()
    }))
    .expect("valid tray");
    let tray = application.tray.as_ref().expect("tray");
    assert_eq!(tray.icon_path(), "shared/tray.png");
    assert_eq!(tray.tooltip_or("Tray Check"), "Tray tooltip é");
    assert!(tray.close_hides_window);
    assert!(!tray.shows_dock_icon());
    assert_eq!(application.menus.tray.len(), 2);
    let ids = application
        .menus
        .items()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["refresh", "dashboard", "tray-refresh"]);
    assert!(!application.menus.is_empty());
    let quiet =
        TrayFixture::discovered(&json!({"tray": {"icon": "/tray.png"}})).expect("icon only");
    let tray = quiet.tray.as_ref().expect("tray");
    assert!(tray.shows_dock_icon());
    assert_eq!(tray.tooltip_or("Tray Check"), "Tray Check");
    assert!(quiet.menus.is_empty());
}

#[test]
fn a_tray_declaration_refuses_bad_icons_text_shortcuts_and_shared_ids() {
    for icon in [
        "",
        "tray.svg",
        "../tray.png",
        "//cdn/tray.png",
        "C:\\tray.png",
    ] {
        let error = TrayFixture::refusal(&json!({"icon": icon}));
        assert!(error.contains("must name a PNG file"), "{icon}: {error}");
    }
    for tooltip in [json!(""), json!("x".repeat(128)), json!("line\nbreak")] {
        let error = TrayFixture::refusal(&json!({"icon": "/tray.png", "tooltip": tooltip}));
        assert!(error.contains("tray.tooltip"), "{error}");
    }
    let shortcut = TrayFixture::refusal(&json!({"icon": "/tray.png", "items": [
        {"id": "x", "label": "X", "route": "/", "shortcut": {"key": "k", "modifiers": ["command"]}}
    ]}));
    assert!(shortcut.contains("cannot have a shortcut"), "{shortcut}");
    let many = (0..33)
        .map(|index| json!({"id": format!("item{index}"), "label": "X", "route": "/"}))
        .collect::<Vec<_>>();
    let crowded = TrayFixture::refusal(&json!({"icon": "/tray.png", "items": many}));
    assert!(crowded.contains("at most 32 items"), "{crowded}");
    let neither =
        TrayFixture::refusal(&json!({"icon": "/tray.png", "items": [{"id": "x", "label": "X"}]}));
    assert!(
        neither.contains("exactly one of route or action"),
        "{neither}"
    );
    let unknown = TrayFixture::refusal(&json!({"icon": "/tray.png", "click": "menu"}));
    assert!(unknown.contains("unknown field"), "{unknown}");
    let shared = TrayFixture::discovered(&json!({
        "menus": {"file": [{"id": "same", "label": "Menu", "route": "/"}]},
        "tray": {"icon": "/tray.png", "items": [{"id": "same", "label": "Tray", "route": "/"}]}
    }))
    .expect_err("shared id")
    .to_string();
    assert!(shared.contains("must differ from the ids"), "{shared}");
}

#[test]
fn the_build_checks_that_the_tray_icon_is_a_png_in_the_bundle() {
    let application = TrayFixture::application();
    let tray = application.tray.as_ref();
    let good = TrayFixture::icon_bundle(TrayFixture::ICON);
    super::native_tray_icon::NativeTrayIcon::check(tray, good.path()).expect("192 pixel PNG");
    super::native_tray_icon::NativeTrayIcon::check(None, Path::new("/missing")).expect("no tray");
    let mut tiny = TrayFixture::ICON[..24].to_vec();
    tiny[16..24].copy_from_slice(&[0, 0, 0, 8, 0, 0, 0, 8]);
    let mut huge = TrayFixture::ICON[..24].to_vec();
    huge[16..24].copy_from_slice(&[0, 0, 8, 0, 0, 0, 8, 0]);
    for bytes in [b"GIF89a not a png at all".to_vec(), tiny, huge] {
        let bundle = TrayFixture::icon_bundle(&bytes);
        let error = super::native_tray_icon::NativeTrayIcon::check(tray, bundle.path())
            .expect_err("bad icon")
            .to_string();
        assert!(error.contains("16 to 1024 pixels"), "{error}");
    }
    let empty = tempfile::tempdir().expect("empty bundle");
    assert!(super::native_tray_icon::NativeTrayIcon::check(tray, empty.path()).is_err());
    let resources = tempfile::tempdir().expect("resources");
    super::native_tray_icon::NativeTrayIcon::stage_for_windows(tray, good.path(), resources.path())
        .expect("Windows icon");
    let ico = std::fs::read(resources.path().join("tray.ico")).expect("tray.ico");
    assert_eq!(&ico[..6], &[0, 0, 1, 0, 1, 0]);
    assert_eq!(&ico[22..30], &TrayFixture::ICON[..8]);
}

#[test]
fn tray_actions_are_checked_and_dispatched_like_menu_actions() {
    let root = tempfile::tempdir().expect("services");
    let inputs = super::native_menu_action_tests::MenuActionFixture::inputs(root.path());
    let menus = TrayFixture::menus_with_tray();
    super::native_menu_action_check::NativeMenuActionCheck::validate(&menus, &inputs)
        .expect("tray action names an application service");
    let mut wrong = menus.clone();
    wrong.tray[1] = serde_json::from_value(json!({"id": "tray-refresh", "label": "Refresh", "action": {"service": "SectionService", "method": "refresh"}})).expect("item");
    let error = super::native_menu_action_check::NativeMenuActionCheck::validate(&wrong, &inputs)
        .expect_err("page service")
        .to_string();
    assert!(
        error.contains("TY1370") && error.contains("tray-refresh"),
        "{error}"
    );
    let dispatch = super::mac_os_menu_action_source::MacOsMenuActionSource::rust_dispatch(&menus);
    assert!(dispatch.contains("\"tray-refresh\" => tac_with_application_SectionMenuService(|service| { let _ = service.refresh(); }),"), "{dispatch}");
    assert!(!dispatch.contains("\"dashboard\""));
}

#[test]
fn desktop_hosts_get_the_tray_settings_and_rows_only_when_a_tray_is_declared() {
    let application = TrayFixture::application();
    let index = super::linux_host_tests::LinuxHostTests::index();
    let linux = super::linux::c_source(&application, &index);
    for expected in [
        "#define TACHYON_TRAY 1\n",
        "#define TACHYON_TRAY_CLOSE_HIDES 1\n",
        "#define TACHYON_TRAY_ICON \"shared/tray.png\"\n",
        "#define TACHYON_TRAY_TOOLTIP \"Tray tooltip \\303\\251\"\n",
        "#define TACHYON_TRAY_OPEN_LABEL \"Open Linux Host Check\"\n",
        "#define TACHYON_TRAY_QUIT_LABEL \"Quit Linux Host Check\"\n",
        "{\"dashboard\", \"Dashboard\", TACHYON_MENU_TRAY, \"/help\", NULL, 0, 0}",
        "{\"tray-refresh\", \"Refresh\", TACHYON_MENU_TRAY, NULL, \"\", 0, 0}",
        "org.kde.StatusNotifierItem-%d-1",
        "g_signal_connect(window, \"close-request\", G_CALLBACK(tachyon_tray_close_request), NULL);",
        "if (TACHYON_MENU_ITEMS[index].group != TACHYON_MENU_TRAY) tachyon_menu_item(",
    ] {
        assert!(linux.contains(expected), "{expected}");
    }
    assert!(!linux.contains("__NATIVE_TRAY"));
    let windows = super::windows::c_source(&application, &index, &[]);
    for expected in [
        "#define TACHYON_TRAY_QUIT_LABEL \"Exit Linux Host Check\"\n",
        "#define TACHYON_TRAY_ICON_FILE L\"tray.ico\"\n",
        "Shell_NotifyIconW(NIM_ADD, &g_tray)",
        "RegisterWindowMessageW(L\"TaskbarCreated\")",
        "if (tachyon_tray_message(window, message, wparam, lparam)) return 0;",
        "if (tachyon_tray_hides_window(window)) return 0;",
        "tachyon_tray_install(g_window);",
        "if (tachyon_tray_command(window, command)) return TRUE;",
    ] {
        assert!(windows.contains(expected), "{expected}");
    }
    let plain = super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default());
    for source in [
        super::linux::c_source(&plain, &index),
        super::windows::c_source(&plain, &index, &[]),
    ] {
        assert!(source.contains("#define TACHYON_TRAY 0\n"));
        assert!(!source.contains("#define TACHYON_TRAY_ICON"));
        assert!(!source.contains("__NATIVE_TRAY"));
    }
}

#[test]
fn the_macos_host_builds_a_status_item_and_can_leave_the_dock() {
    let application = TrayFixture::application();
    let index = super::linux_host_tests::LinuxHostTests::index();
    let source = super::macos::swift_source(&application, "tacRustInvoke(payload)", &index);
    for expected in [
        "NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)",
        "tray = MacOsTray(\"Linux Host Check\", \"shared/tray.png\", true, \"Tray tooltip é\", [MacOsTrayItem(id: \"dashboard\", label: \"Dashboard\", route: \"/help\", event: \"\")",
        "MacOsTrayItem(id: \"tray-refresh\", label: \"Refresh\", route: \"\", event: \"\")",
        "private let tachyonTrayHidesWindow = true",
        "application.setActivationPolicy(.accessory)",
        "func windowShouldClose(_ sender: NSWindow) -> Bool {",
        "@_silgen_name(\"tac_native_menu_action\")",
    ] {
        assert!(source.contains(expected), "{expected}");
    }
    assert!(!source.contains("__NATIVE_TRAY"));
    assert!(!source.contains("__NATIVE_ACTIVATION_POLICY__"));
    let plain = super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default());
    let quiet = super::macos::swift_source(&plain, "tacRustInvoke(payload)", &index);
    assert!(quiet.contains("application.setActivationPolicy(.regular)"));
    assert!(quiet.contains("private let tachyonTrayHidesWindow = false"));
    assert!(!quiet.contains("tray = MacOsTray("));
    assert_eq!(
        super::mac_os_tray_source::MacOsTraySource::plist_entry(&application),
        "\n<key>LSUIElement</key><true/>"
    );
    assert_eq!(
        super::mac_os_tray_source::MacOsTraySource::plist_entry(&plain),
        ""
    );
}

#[test]
fn every_target_states_what_it_does_with_a_tray() {
    let manifest = json!({"tac": {"application": {"tray": {"icon": "/tray.png", "template": true, "closeHidesWindow": true, "dock": false}}}});
    let root = TrayFixture::project(&manifest["tac"]["application"]);
    for target in [AppTarget::Ios, AppTarget::Android] {
        let failure = super::target_support::TargetSupport::notes(root.path(), target)
            .expect_err("mobile has no tray");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1601);
        assert!(diagnostic.message.contains("tac.application.tray"));
    }
    let mac =
        super::target_support::TargetSupport::notes(root.path(), AppTarget::Macos).expect("macOS");
    assert_eq!(mac.len(), 4, "{mac:?}");
    assert!(
        mac.iter()
            .all(|note| note.contains("signed-in Mac desktop")),
        "{mac:?}"
    );
    let linux =
        super::target_support::TargetSupport::notes(root.path(), AppTarget::Linux).expect("Linux");
    assert!(
        linux.iter().any(|note| note.contains("StatusNotifierItem")),
        "{linux:?}"
    );
    assert!(
        linux
            .iter()
            .any(|note| note.contains("only while a tray host shows the icon")),
        "{linux:?}"
    );
    let windows = super::target_support::TargetSupport::notes(root.path(), AppTarget::Windows)
        .expect("Windows");
    assert!(
        windows.iter().any(|note| note.contains("macOS Dock")),
        "{windows:?}"
    );
    assert!(
        windows.iter().any(|note| note.contains("only under Wine")),
        "{windows:?}"
    );
    assert!(
        windows.iter().any(|note| note.contains("own colors")),
        "{windows:?}"
    );
    assert!(
        windows
            .iter()
            .any(|note| note.contains("closing, reopening and exiting")),
        "{windows:?}"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, a display and a D-Bus session bus; run explicitly on the Linux lane"]
async fn linux_generated_host_shows_a_status_notifier_item_with_a_working_menu() {
    let root = tempfile::tempdir().expect("fixture");
    let source = TrayFixture::stage(root.path());
    let icon = root
        .path()
        .join("LinuxHostCheck/resources/WebBundle/shared/tray.png");
    std::fs::create_dir_all(icon.parent().expect("parent")).expect("icon folder");
    std::fs::write(&icon, TrayFixture::ICON).expect("icon");
    let executable = super::linux_host_tests::LinuxHostTests::compile_application(
        root.path(),
        &source,
        &TrayFixture::application(),
        include_str!("linux_tray_fixture.c"),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host tray");
    assert!(output.contains("linux tray passed"), "{output}");
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and Wine; runs the Win32 notification-area code, not WebView2 or MSVC"]
async fn windows_tray_code_adds_an_icon_hides_on_close_and_quits_under_wine() {
    let root = tempfile::tempdir().expect("fixture");
    let application = TrayFixture::application();
    let settings =
        super::desktop_tray_source::DesktopTraySource::c_settings(&application, '&', "Exit");
    let harness = super::windows_host_tests::WindowsHostTests::harness_with(
        &application.menus,
        &settings,
        include_str!("windows_tray_fixture.c"),
    );
    let source = root.path().join("tray.c");
    std::fs::write(&source, harness).expect("harness");
    let executable = root.path().join("bin/tray.exe");
    std::fs::create_dir_all(root.path().join("bin")).expect("bin");
    let bundle = TrayFixture::icon_bundle(TrayFixture::ICON);
    super::native_tray_icon::NativeTrayIcon::stage_for_windows(
        application.tray.as_ref(),
        bundle.path(),
        &root.path().join("resources"),
    )
    .expect("tray.ico");
    super::host::run_tool(
        super::windows_host_tests::WindowsHostTests::MINGW,
        &[
            "-std=gnu17",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            source.to_str().expect("source"),
            "-luser32",
            "-lshell32",
            "-o",
            executable.to_str().expect("executable"),
        ],
    )
    .await
    .expect("Win32 tray code compiles and links");
    let prefix = tempfile::tempdir().expect("wine prefix");
    let output = tokio::process::Command::new("wine")
        .arg(&executable)
        .env("WINEPREFIX", prefix.path())
        .env("WINEDEBUG", "-all")
        .output()
        .await
        .expect("wine");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("windows tray passed"),
        "{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

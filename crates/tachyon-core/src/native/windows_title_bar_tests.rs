#![allow(clippy::expect_used)]

use super::config::{NativeApplication, WindowConfiguration};
use super::native_title_bar::NativeTitleBar;
use super::native_title_bar_button_inset::NativeTitleBarButtonInset;
use std::process::Command;

pub(super) struct WindowsTitleBarTests;

impl WindowsTitleBarTests {
    #[cfg(unix)]
    pub(super) const MINGW: &str = "x86_64-w64-mingw32-gcc";
    #[cfg(unix)]
    pub(super) const WARNINGS: [&str; 5] = [
        "-std=gnu17",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-Wno-missing-field-initializers",
    ];

    fn window(title_bar: NativeTitleBar) -> WindowConfiguration {
        WindowConfiguration {
            width: 800,
            height: 600,
            title_bar,
            controls: vec![String::from("title"), String::from("resize")],
            ..WindowConfiguration::default()
        }
    }

    pub(super) fn application(window: WindowConfiguration) -> NativeApplication {
        NativeApplication {
            name: String::from("Title Bar Check"),
            executable_name: String::from("TitleBarCheck"),
            application_id: String::from("org.tachyon.title-bar-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window,
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: super::native_rust_project::NativeRustProject::default(),
            outgoing: super::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: super::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: super::minimum_system::MinimumSystem::default(),
            description: String::new(),
            links: super::native_links::NativeLinks::default(),
            tray: None,
        }
    }

    fn host(title_bar: NativeTitleBar) -> String {
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        };
        super::windows::c_source(&Self::application(Self::window(title_bar)), &index, &[])
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    #[cfg(unix)]
    const EVENT_TOKEN: &str = "#pragma once\ntypedef struct EventRegistrationToken { __int64 value; } EventRegistrationToken;\n";

    fn webview2_include() -> Option<std::path::PathBuf> {
        let sdk = std::env::var_os("TAC_WEBVIEW2_SDK").map(std::path::PathBuf::from)?;
        let include = sdk.join("build/native/include");
        include.join("WebView2.h").is_file().then_some(include)
    }

    #[cfg(unix)]
    pub(super) fn headers(root: &std::path::Path) -> Vec<std::ffi::OsString> {
        let include = Self::webview2_include().expect("TAC_WEBVIEW2_SDK");
        let shims = root.join("shims");
        std::fs::create_dir_all(&shims).expect("shims");
        std::fs::write(shims.join("EventToken.h"), Self::EVENT_TOKEN).expect("EventToken.h");
        [
            "-isystem".into(),
            include.into_os_string(),
            "-isystem".into(),
            shims.into_os_string(),
        ]
        .into()
    }

    fn fixture(window: &WindowConfiguration) -> String {
        include_str!("../../tests/fixtures/native/windows_title_bar_hwnd.c")
            .replace(
                "__WINDOW_FRAME__",
                &super::windows_window_frame::WindowsWindowFrame::source(window),
            )
            .replace(
                "__WINDOW_TITLE_BAR__",
                &super::windows_title_bar::WindowsTitleBar::source(window),
            )
    }

    #[cfg(unix)]
    pub(super) fn display() -> Command {
        if std::env::var_os("DISPLAY").is_some() {
            return Command::new("env");
        }
        let mut command = Command::new("xvfb-run");
        command.args(["-a", "-s", "-screen 0 1920x1080x24"]);
        command
    }

    pub(super) fn success(command: &mut Command) -> String {
        let output = command.output().expect("program");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        stdout
    }
}

#[test]
fn the_integrated_windows_host_wires_the_title_bar_into_the_window_and_the_web_view() {
    let source = WindowsTitleBarTests::host(NativeTitleBar::Integrated);
    assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = TRUE;"));
    let procedure =
        WindowsTitleBarTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    for case in [
        "    case WM_NCCALCSIZE:\n      if (tachyon_title_bar_calculate(window, wparam, lparam)) return 0;",
        "      LRESULT hit = tachyon_title_bar_hit(window, wparam, lparam, &handled);\n      if (handled) return hit;",
        "    case WM_ACTIVATE:\n      tachyon_title_bar_layout(window);",
        "    case WM_PAINT:\n      if (tachyon_title_bar_paint(window)) return 0;",
        "    case WM_DPICHANGED:\n    case WM_SETTINGCHANGE:\n    case WM_THEMECHANGED:\n      tachyon_title_bar_layout(window);",
        "    case WM_SIZE:\n      tachyon_title_bar_layout(window);",
        "    case WM_SYSCOMMAND:\n      if (wparam < SC_SIZE && tachyon_menus_command(window, (UINT)wparam)) return 0;",
        "      tachyon_menus_refresh((HMENU)wparam);\n      if (HIWORD(lparam)) break;\n      return 0;",
    ] {
        assert!(procedure.contains(case), "{case}");
    }
    let main = WindowsTitleBarTests::section(&source, "int WINAPI wWinMain(");
    assert!(
        main.find("tachyon_menus_install(g_window);")
            .expect("menus")
            < main
                .find("if (!tachyon_title_bar_install(g_window, instance))")
                .expect("title bar")
    );
    assert!(
        main.find("tachyon_title_bar_install(").expect("title bar")
            < main
                .find("tachyon_window_open(g_window, show)")
                .expect("open")
    );
    assert!(source.contains("environment, tachyon_title_bar_web_parent(g_window),"));
    let controller = WindowsTitleBarTests::section(&source, "controller_invoke(");
    assert!(controller.contains("tachyon_title_bar_settings(g_webview);"));
    assert!(controller.contains("TACHYON_NATIVE_SHIM, TACHYON_TITLE_BAR_PAGE_SCRIPT);"));
    let handle = WindowsTitleBarTests::section(&source, "static char *tachyon_handle(");
    assert!(handle.starts_with(
        "const char *capability, const char *payload) {\n  if (strcmp(capability, \"window.titleBarInsets\") == 0) return tachyon_title_bar_insets();"
    ));
    let open = WindowsTitleBarTests::section(&source, "static SIZE tachyon_window_outer(");
    assert!(open.contains("outer.cy = bounds.bottom - 0;"));
    assert!(source.contains("bounds.right - bounds.left, bounds.bottom, SWP_NOMOVE"));
    assert!(
        source.contains("if (title[0] == 0) { wchar_t *name = tachyon_widen(TACHYON_APP_NAME);")
    );
    assert!(!source.contains("__TITLE_BAR_"));
    assert!(!source.contains("__WINDOW_"));
}

#[test]
fn the_standard_windows_host_keeps_its_frame_menu_bar_and_window_size() {
    let source = WindowsTitleBarTests::host(NativeTitleBar::Standard);
    assert!(source.contains("TACHYON_TITLE_BAR_INTEGRATED = FALSE;"));
    assert!(source.contains("TACHYON_TITLE_BAR_PAGE_SCRIPT = \"\";"));
    let open = WindowsTitleBarTests::section(&source, "static SIZE tachyon_window_outer(");
    assert!(open.contains("outer.cy = bounds.bottom - bounds.top;"));
    assert!(source.contains("bounds.right - bounds.left, bounds.bottom - bounds.top, SWP_NOMOVE"));
    assert!(!source.contains("if (title[0] == 0) { wchar_t *name"));
    let calculate =
        WindowsTitleBarTests::section(&source, "static BOOL tachyon_title_bar_calculate(");
    assert!(
        calculate.contains("if (!wparam || !tachyon_title_bar_captioned(window)) return FALSE;")
    );
}

#[test]
fn a_button_inset_does_not_change_the_windows_title_bar() {
    let mut window = WindowsTitleBarTests::window(NativeTitleBar::Integrated);
    let plain = super::windows_title_bar::WindowsTitleBar::source(&window);
    window.title_bar_button_inset = Some(NativeTitleBarButtonInset {
        leading: 20,
        top: 35,
    });
    assert_eq!(
        super::windows_title_bar::WindowsTitleBar::source(&window),
        plain
    );
}

#[cfg(unix)]
#[test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and TAC_WEBVIEW2_SDK; compiles the whole generated host against the Win32 and WebView2 headers, not MSVC"]
fn the_windows_host_with_each_title_bar_compiles_against_the_win32_and_webview2_headers() {
    let root = tempfile::tempdir().expect("fixture");
    let headers = WindowsTitleBarTests::headers(root.path());
    for (name, title_bar) in [
        ("standard", NativeTitleBar::Standard),
        ("integrated", NativeTitleBar::Integrated),
    ] {
        let source = root.path().join(format!("{name}.c"));
        std::fs::write(&source, WindowsTitleBarTests::host(title_bar)).expect("source");
        WindowsTitleBarTests::success(
            Command::new(WindowsTitleBarTests::MINGW)
                .args(WindowsTitleBarTests::WARNINGS)
                .args([
                    "-Wno-unused-parameter",
                    "-Wno-unused-function",
                    "-Wno-cast-function-type",
                ])
                .args(&headers)
                .arg("-c")
                .arg(&source)
                .arg("-o")
                .arg(root.path().join(format!("{name}.o"))),
        );
    }
}

#[cfg(unix)]
#[test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler, wine64, an X display or xvfb-run, and TAC_WEBVIEW2_SDK; drives the title bar frame, hit test, menus and insets on a Win32 window under Wine, without DWM or WebView2"]
fn the_windows_title_bar_drives_a_win32_window_under_wine() {
    let root = tempfile::tempdir().expect("fixture");
    let headers = WindowsTitleBarTests::headers(root.path());
    let window = WindowsTitleBarTests::window(NativeTitleBar::Integrated);
    let source = root.path().join("title-bar.c");
    let program = root.path().join("title-bar.exe");
    std::fs::write(&source, WindowsTitleBarTests::fixture(&window)).expect("source");
    WindowsTitleBarTests::success(
        Command::new(WindowsTitleBarTests::MINGW)
            .args(WindowsTitleBarTests::WARNINGS)
            .args(&headers)
            .arg(&source)
            .args(["-luser32", "-lgdi32", "-ldwmapi", "-o"])
            .arg(&program),
    );
    let prefix = root.path().join("prefix");
    let mut wine = WindowsTitleBarTests::display();
    let stdout = WindowsTitleBarTests::success(
        wine.args(["wine64"])
            .arg(&program)
            .env("WINEPREFIX", &prefix)
            .env("WINEDEBUG", "-all"),
    );
    assert!(stdout.contains("win32 title bar:"), "{stdout}");
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows, MSVC cl on PATH (vcvars) and TAC_WEBVIEW2_SDK; drives the title bar frame, hit test, menus and insets on a real Win32 window, not WebView2"]
fn the_windows_title_bar_drives_a_real_win32_window() {
    let include = WindowsTitleBarTests::webview2_include().expect("TAC_WEBVIEW2_SDK");
    let root = tempfile::tempdir().expect("fixture");
    let window = WindowsTitleBarTests::window(NativeTitleBar::Integrated);
    let source = root.path().join("title-bar.c");
    let program = root.path().join("title-bar.exe");
    std::fs::write(&source, WindowsTitleBarTests::fixture(&window)).expect("source");
    WindowsTitleBarTests::success(
        Command::new("cl")
            .current_dir(root.path())
            .args(["/nologo", "/W4", "/WX", "/I"])
            .arg(&include)
            .arg(&source)
            .arg(format!("/Fe:{}", program.display()))
            .args(["user32.lib", "gdi32.lib", "dwmapi.lib"]),
    );
    let stdout = WindowsTitleBarTests::success(&mut Command::new(&program));
    println!("{}", stdout.trim());
    assert!(stdout.contains("win32 title bar:"), "{stdout}");
}

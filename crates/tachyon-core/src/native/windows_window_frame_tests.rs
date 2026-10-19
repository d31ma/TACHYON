#![allow(clippy::expect_used)]

use super::config::{NativeApplication, WindowConfiguration};
use super::native_display::NativeDisplay;
use super::native_theme_color::NativeThemeColor;
use super::windows_window_frame::WindowsWindowFrame;
use std::process::Command;

struct WindowsWindowFrameTests;

impl WindowsWindowFrameTests {
    const FULLSCREEN: &str = include_str!("native_window_fullscreen_windows.c");

    fn declared(display: NativeDisplay) -> WindowConfiguration {
        WindowConfiguration {
            width: 800,
            height: 600,
            min_width: Some(480),
            min_height: Some(360),
            theme_color: Some(NativeThemeColor {
                red: 15,
                green: 108,
                blue: 189,
            }),
            display,
            ..WindowConfiguration::default()
        }
    }

    fn host(window: WindowConfiguration) -> String {
        let application = NativeApplication {
            name: String::from("Window Frame Check"),
            executable_name: String::from("WindowFrameCheck"),
            application_id: String::from("org.tachyon.window-frame-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window,
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: super::native_rust_project::NativeRustProject::default(),
            outgoing: super::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
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
        super::windows::c_source(&application, &index, &[])
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    fn fixture(template: &str, window: &WindowConfiguration, fullscreen: &str) -> String {
        template
            .replace("__WINDOW_FULLSCREEN__", fullscreen)
            .replace("__WINDOW_FRAME__", &WindowsWindowFrame::source(window))
            .replace(
                "__WINDOW_FRAME_MESSAGES__",
                WindowsWindowFrame::messages(window),
            )
    }

    fn success(command: &mut Command) -> String {
        let output = command.output().expect("program");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        stdout
    }

    fn compiler_available() -> bool {
        Command::new("cc")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    #[cfg(windows)]
    fn win32(name: &str, window: &WindowConfiguration, fullscreen: &str, defines: &[&str]) {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join(format!("{name}.c"));
        let program = root.path().join(format!("{name}.exe"));
        let template = include_str!("../../tests/fixtures/native/windows_window_frame_hwnd.c");
        std::fs::write(&source, Self::fixture(template, window, fullscreen)).expect("source");
        let stdout = Self::success(
            Command::new("cl")
                .current_dir(root.path())
                .args(["/nologo", "/W4", "/WX"])
                .args(defines)
                .arg(&source)
                .arg(format!("/Fe:{}", program.display()))
                .args(["user32.lib", "dwmapi.lib"]),
        );
        print!("{stdout}");
        println!("{}", Self::success(&mut Command::new(&program)).trim());
    }
}

#[test]
fn the_windows_host_opens_at_the_manifest_size_instead_of_a_fixed_one() {
    let source = WindowsWindowFrameTests::host(WindowConfiguration::default());
    let main = WindowsWindowFrameTests::section(&source, "int WINAPI wWinMain(");
    assert!(main.contains(
        "WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,\n                             CW_USEDEFAULT, NULL, NULL, instance, NULL);"
    ));
    assert!(!main.contains("1024, 768"));
    assert!(main.contains("ShowWindow(g_window, tachyon_window_open(g_window, show));"));
    assert!(
        main.find("tachyon_menus_install(g_window);")
            .expect("menus")
            < main.find("tachyon_window_open(").expect("open")
    );
    let open = WindowsWindowFrameTests::section(&source, "static int tachyon_window_open(");
    assert!(open.contains("SIZE outer = tachyon_window_outer(window, 1024, 768);"));
    assert!(!open.contains("DwmSetWindowAttribute"));
    assert!(!open.contains("tachyon_window_toggle_fullscreen"));
    assert!(!source.contains("tachyon_window_limits"));
    assert!(!source.contains("static BOOL tachyon_window_toggle_fullscreen("));
    assert!(source.contains("#include <dwmapi.h>"));
    assert!(super::windows::WindowsHostGenerator::LINKER_ARGUMENTS.contains(&"dwmapi.lib"));
    assert!(!source.contains("__WINDOW_"));
}

#[test]
fn the_windows_host_applies_the_minimum_size_caption_color_and_fullscreen_start() {
    let source =
        WindowsWindowFrameTests::host(WindowsWindowFrameTests::declared(NativeDisplay::Fullscreen));
    let open = WindowsWindowFrameTests::section(&source, "static int tachyon_window_open(");
    assert!(open.contains("SIZE outer = tachyon_window_outer(window, 800, 600);"));
    assert!(
        open.contains("COLORREF caption = RGB(15, 108, 189), caption_text = RGB(255, 255, 255);")
    );
    assert!(open.contains(
        "DwmSetWindowAttribute(window, TACHYON_CAPTION_COLOR_ATTRIBUTE, &caption, sizeof(caption));"
    ));
    assert!(open.contains(
        "if (tachyon_window_toggle_fullscreen(window) && show == SW_SHOWMAXIMIZED) show = SW_SHOWNORMAL;"
    ));
    assert_eq!(
        source
            .matches("static BOOL tachyon_window_toggle_fullscreen(")
            .count(),
        1
    );
    let limits = WindowsWindowFrameTests::section(&source, "static void tachyon_window_limits(");
    assert!(limits.contains("SIZE minimum = tachyon_window_outer(window, 480, 360);"));
    assert!(limits.contains("limits->ptMinTrackSize.x = minimum.cx;"));
    assert!(limits.contains("limits->ptMinTrackSize.y = minimum.cy;"));
    let procedure =
        WindowsWindowFrameTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    assert!(procedure.contains(
        "    case WM_GETMINMAXINFO:\n      tachyon_window_limits(window, (MINMAXINFO *)lparam);\n      return 0;\n    case WM_SIZE:"
    ));
    assert!(!source.contains("__WINDOW_"));
}

#[test]
fn a_light_theme_color_takes_dark_caption_text_and_one_minimum_sets_one_edge() {
    let window = WindowConfiguration {
        min_height: Some(300),
        theme_color: Some(NativeThemeColor {
            red: 255,
            green: 215,
            blue: 0,
        }),
        display: NativeDisplay::MinimalUi,
        ..WindowConfiguration::default()
    };
    let source = WindowsWindowFrame::source(&window);
    assert!(source.contains("caption_text = RGB(0, 0, 0);"));
    assert!(source.contains("SIZE minimum = tachyon_window_outer(window, 0, 300);"));
    assert!(!source.contains("ptMinTrackSize.x"));
    assert!(source.contains("ptMinTrackSize.y = minimum.cy;"));
    assert!(!source.contains("tachyon_window_toggle_fullscreen"));
}

#[test]
fn the_windows_window_frame_scales_sizes_and_limits_in_executed_host_code() {
    if !WindowsWindowFrameTests::compiler_available() {
        eprintln!("skipping: C compiler is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("fixture");
    let source = root.path().join("frame.c");
    let program = root.path().join("frame");
    let window = WindowsWindowFrameTests::declared(NativeDisplay::Fullscreen);
    std::fs::write(
        &source,
        WindowsWindowFrameTests::fixture(
            include_str!("../../tests/fixtures/native/windows_window_frame_host.c"),
            &window,
            WindowsWindowFrameTests::FULLSCREEN,
        ),
    )
    .expect("source");
    WindowsWindowFrameTests::success(
        Command::new("cc")
            .args([
                "-Wall",
                "-Wextra",
                "-Werror",
                "-Wno-missing-field-initializers",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&program),
    );
    WindowsWindowFrameTests::success(&mut Command::new(&program));
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows and MSVC cl on PATH (vcvars); drives a real Win32 window, not WebView2"]
fn the_windows_window_frame_drives_a_real_win32_window() {
    WindowsWindowFrameTests::win32(
        "sized",
        &WindowsWindowFrameTests::declared(NativeDisplay::Standalone),
        "",
        &[],
    );
    WindowsWindowFrameTests::win32(
        "fullscreen",
        &WindowsWindowFrameTests::declared(NativeDisplay::Fullscreen),
        WindowsWindowFrameTests::FULLSCREEN,
        &["/DTACHYON_EXPECT_FULLSCREEN"],
    );
}

#[cfg(unix)]
#[test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler; checks the Win32 window frame code against the Win32 headers, not WebView2 or MSVC"]
fn the_windows_window_frame_compiles_and_links_against_the_win32_headers() {
    let root = tempfile::tempdir().expect("fixture");
    let template = include_str!("../../tests/fixtures/native/windows_window_frame_hwnd.c");
    for (name, display, fullscreen, defines) in [
        ("sized", NativeDisplay::Standalone, "", &[][..]),
        (
            "fullscreen",
            NativeDisplay::Fullscreen,
            WindowsWindowFrameTests::FULLSCREEN,
            &["-DTACHYON_EXPECT_FULLSCREEN"][..],
        ),
    ] {
        let window = WindowsWindowFrameTests::declared(display);
        let source = root.path().join(format!("{name}.c"));
        std::fs::write(
            &source,
            WindowsWindowFrameTests::fixture(template, &window, fullscreen),
        )
        .expect("source");
        WindowsWindowFrameTests::success(
            Command::new("x86_64-w64-mingw32-gcc")
                .args([
                    "-std=gnu17",
                    "-O2",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-Wno-missing-field-initializers",
                ])
                .args(defines)
                .arg(&source)
                .args(["-luser32", "-ldwmapi", "-o"])
                .arg(root.path().join(format!("{name}.exe"))),
        );
    }
}

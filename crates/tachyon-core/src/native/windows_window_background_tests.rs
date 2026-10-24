#![allow(clippy::expect_used)]

use super::config::WindowConfiguration;
use super::native_theme_color::NativeThemeColor;
use super::native_title_bar::NativeTitleBar;
use super::windows_title_bar_tests::WindowsTitleBarTests;
use super::windows_window_background::WindowsWindowBackground;
use std::process::Command;

struct WindowsWindowBackgroundTests;

impl WindowsWindowBackgroundTests {
    const EXPECTED: [&'static str; 4] = [
        "-DTACHYON_EXPECT_RED=26",
        "-DTACHYON_EXPECT_GREEN=35",
        "-DTACHYON_EXPECT_BLUE=126",
        "-DTACHYON_EXPECT_ENVIRONMENT=L\"FF1A237E\"",
    ];

    fn window(title_bar: NativeTitleBar) -> WindowConfiguration {
        WindowConfiguration {
            title_bar,
            background_color: Some(NativeThemeColor {
                red: 26,
                green: 35,
                blue: 126,
            }),
            ..WindowConfiguration::default()
        }
    }

    fn host(title_bar: NativeTitleBar) -> String {
        super::windows::c_source(
            &WindowsTitleBarTests::application(Self::window(title_bar)),
            &super::linux_host_tests::LinuxHostTests::index(),
            &[],
        )
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    fn fixture() -> String {
        include_str!("../../tests/fixtures/native/windows_window_background_hwnd.c").replace(
            "__WINDOW_BACKGROUND__",
            &WindowsWindowBackground::window_source(&Self::window(NativeTitleBar::Standard)),
        )
    }
}

#[test]
fn the_windows_host_paints_background_color_before_webview2_draws_the_page() {
    let source = WindowsWindowBackgroundTests::host(NativeTitleBar::Standard);
    let main = WindowsWindowBackgroundTests::section(&source, "int WINAPI wWinMain(");
    assert!(main.contains("window_class.hbrBackground = tachyon_window_background_brush();"));
    assert!(
        main.find("tachyon_window_background_environment();")
            .expect("environment")
            < main
                .find("CreateCoreWebView2EnvironmentWithOptions(")
                .expect("web engine")
    );
    let controller = WindowsWindowBackgroundTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE controller_invoke(",
    );
    assert!(
        controller
            .find("tachyon_web_view_background(controller);")
            .expect("web view background")
            < controller
                .find("ICoreWebView2Controller_put_IsVisible(controller, TRUE);")
                .expect("visible")
    );
    assert!(source.contains("return CreateSolidBrush(RGB(26, 35, 126));"));
    assert!(source.contains("COREWEBVIEW2_COLOR background = {255, 26, 35, 126};"));
    assert!(!source.contains("__WINDOW_BACKGROUND"));
    assert!(!source.contains("__WEB_VIEW_"));
}

#[cfg(unix)]
#[test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and TAC_WEBVIEW2_SDK; compiles the whole generated host with background_color against the Win32 and WebView2 headers, not MSVC"]
fn the_windows_host_with_background_color_compiles_against_the_win32_and_webview2_headers() {
    let root = tempfile::tempdir().expect("fixture");
    let headers = WindowsTitleBarTests::headers(root.path());
    for (name, title_bar) in [
        ("standard", NativeTitleBar::Standard),
        ("integrated", NativeTitleBar::Integrated),
    ] {
        let source = root.path().join(format!("{name}.c"));
        std::fs::write(&source, WindowsWindowBackgroundTests::host(title_bar)).expect("source");
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
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler, wine64 and an X display or xvfb-run; paints background_color in a Win32 window under Wine, without DWM or WebView2"]
fn the_windows_window_background_paints_a_win32_window_under_wine() {
    let root = tempfile::tempdir().expect("fixture");
    let source = root.path().join("background.c");
    let program = root.path().join("background.exe");
    std::fs::write(&source, WindowsWindowBackgroundTests::fixture()).expect("source");
    WindowsTitleBarTests::success(
        Command::new(WindowsTitleBarTests::MINGW)
            .args(WindowsTitleBarTests::WARNINGS)
            .args(WindowsWindowBackgroundTests::EXPECTED)
            .arg(&source)
            .args(["-luser32", "-lgdi32", "-o"])
            .arg(&program),
    );
    let stdout = WindowsTitleBarTests::success(
        WindowsTitleBarTests::display()
            .arg("wine64")
            .arg(&program)
            .env("WINEPREFIX", root.path().join("prefix"))
            .env("WINEDEBUG", "-all"),
    );
    assert!(stdout.contains("win32 window background:"), "{stdout}");
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows and MSVC cl on PATH (vcvars); paints background_color in a real Win32 window, not WebView2"]
fn the_windows_window_background_paints_a_real_win32_window() {
    let root = tempfile::tempdir().expect("fixture");
    let source = root.path().join("background.c");
    let program = root.path().join("background.exe");
    std::fs::write(&source, WindowsWindowBackgroundTests::fixture()).expect("source");
    let defines =
        WindowsWindowBackgroundTests::EXPECTED.map(|define| define.replacen("-D", "/D", 1));
    let stdout = WindowsTitleBarTests::success(
        Command::new("cl")
            .current_dir(root.path())
            .args(["/nologo", "/W4", "/WX"])
            .args(defines)
            .arg(&source)
            .arg(format!("/Fe:{}", program.display()))
            .args(["user32.lib", "gdi32.lib"]),
    );
    print!("{stdout}");
    let stdout = WindowsTitleBarTests::success(&mut Command::new(&program));
    println!("{}", stdout.trim());
    assert!(stdout.contains("win32 window background:"), "{stdout}");
}

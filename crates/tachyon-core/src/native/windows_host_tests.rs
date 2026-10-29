#![allow(clippy::expect_used)]

use super::native_menu_config::NativeMenuConfig;
use serde_json::json;

pub(super) struct WindowsHostTests;

impl WindowsHostTests {
    #[cfg(unix)]
    pub(super) const MINGW: &str = "x86_64-w64-mingw32-gcc";
    #[cfg(target_arch = "aarch64")]
    pub(super) const MINGW_FOR_WINE: &str = "aarch64-w64-mingw32-gcc";
    #[cfg(not(target_arch = "aarch64"))]
    pub(super) const MINGW_FOR_WINE: &str = "x86_64-w64-mingw32-gcc";
    #[cfg(target_arch = "aarch64")]
    pub(super) const RUST_TARGET_FOR_WINE: &str = "aarch64-pc-windows-gnullvm";
    #[cfg(not(target_arch = "aarch64"))]
    pub(super) const RUST_TARGET_FOR_WINE: &str = "x86_64-pc-windows-gnu";
    #[cfg(unix)]
    const HARNESS_HEAD: &str = r"#define UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shellapi.h>
#include <ctype.h>
#include <stdbool.h>
#include <stdio.h>

static HWND g_window = NULL;
static BOOL tachyon_menu_window_open(void) { return g_window != NULL; }
static BOOL tachyon_menu_page_shown(void) { return g_window != NULL; }
static void tachyon_menu_open_route(const char *route) { (void)route; }
static BOOL tachyon_menu_edit(const char *command) { return command != NULL; }
";
    #[cfg(unix)]
    const HARNESS_MAIN: &str = r#"
int main(void) {
  HMODULE library = LoadLibraryW(L"TachyonRustCompanion.dll");
  if (library != NULL) tachyon_menus_bind(library);
  MSG message = {0};
  tachyon_menus_install(g_window);
  tachyon_menus_refresh(NULL);
  BOOL handled = tachyon_menus_command(g_window, 0) || tachyon_menus_translate(g_window, &message) || tachyon_menus_key(g_window, 'W');
  tachyon_menus_destroy();
  tachyon_tray_install(g_window);
  handled = handled || tachyon_tray_message(g_window, 0, 0, 0) || tachyon_tray_hides_window(g_window);
  tachyon_tray_remove();
  handled = handled || tachyon_tray_icon_event() != NULL;
  return handled ? 1 : 0;
}
"#;

    fn menus() -> NativeMenuConfig {
        serde_json::from_value(json!({
            "file": [
                {"id":"refresh","label":"Refresh & Reload","action":{"service":"SectionMenuService","method":"refresh"}}
            ],
            "view": [
                {"id":"show-running","label":"What's Running","action":{"service":"SectionMenuService","method":"show_running","event":"SectionChosen"},"shortcut":{"key":"1","modifiers":["command","shift"]}}
            ],
            "help": [
                {"id":"guide","label":"Guide","route":"/help"}
            ]
        }))
        .expect("menu declarations")
    }

    fn host(menus: NativeMenuConfig) -> String {
        let application = super::config::NativeApplication {
            name: String::from("Windows Host Check"),
            executable_name: String::from("WindowsHostCheck"),
            application_id: String::from("org.tachyon.windows-host-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus,
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
        super::windows::c_source(&application, &index, &[])
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    #[cfg(unix)]
    fn harness(menus: &NativeMenuConfig) -> String {
        Self::harness_with(menus, "#define TACHYON_TRAY 0\n", Self::HARNESS_MAIN)
    }

    #[cfg(unix)]
    pub(super) fn harness_with(menus: &NativeMenuConfig, tray: &str, main: &str) -> String {
        let block = include_str!("windows_menus.c").replace(
            "__NATIVE_MENU_ITEMS__",
            &super::desktop_menu_items::DesktopMenuItems::c_table(menus, '&'),
        );
        let tray = include_str!("windows_tray.c").replace("__NATIVE_TRAY_SETTINGS__", tray);
        format!("{}\n{block}\n{tray}\n{main}", Self::HARNESS_HEAD)
    }

    /// The Wine loader on PATH: `wine` (Wine 10 and later, built for aarch64
    /// or x86_64, and Ubuntu's wrapper), else Ubuntu's `wine64`.
    pub(super) fn wine() -> Option<&'static str> {
        ["wine", "wine64"].into_iter().find(|loader| {
            std::process::Command::new(loader)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
    }

    /// Copies llvm-mingw's `libunwind.dll`, which `aarch64-pc-windows-gnullvm`
    /// Rust binaries may load, next to them; `x86_64-pc-windows-gnu` needs none.
    pub(super) fn stage_wine_runtime(folder: &std::path::Path) {
        if !cfg!(target_arch = "aarch64") {
            return;
        }
        let Some(path) = std::env::var_os("PATH") else {
            return;
        };
        let library = std::env::split_paths(&path)
            .map(|bin| bin.join(Self::MINGW_FOR_WINE))
            .find(|compiler| compiler.is_file())
            .and_then(|compiler| Some(compiler.parent()?.parent()?.to_path_buf()))
            .map(|root| root.join("aarch64-w64-mingw32/bin/libunwind.dll"))
            .filter(|library| library.is_file());
        if let Some(library) = library {
            std::fs::copy(library, folder.join("libunwind.dll")).expect("libunwind.dll");
        }
    }
}

#[test]
fn windows_host_installs_a_menu_bar_with_accelerators_for_the_window_and_the_web_view() {
    let source = WindowsHostTests::host(WindowsHostTests::menus());
    assert!(source.contains("static const int TACHYON_MENU_ITEM_COUNT = 3;"));
    assert!(source.contains(
        "{\"show-running\", \"What's Running\", TACHYON_MENU_VIEW, NULL, \"SectionChosen\", '1', 5}"
    ));
    assert!(source.contains("\"Refresh && Reload\""));
    assert!(source.contains("SetMenu(window, bar);"));
    assert!(source.contains("g_menu_accelerators = CreateAcceleratorTableW(accelerators, count);"));
    assert!(source.contains("if (tachyon_menus_translate(g_window, &message)) continue;"));
    assert!(source.contains("tachyon_menus_install(g_window);"));
    assert!(source.contains("ICoreWebView2Controller_add_AcceleratorKeyPressed("));
    let window = WindowsHostTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    assert!(window.contains("case WM_INITMENUPOPUP:\n      tachyon_menus_refresh((HMENU)wparam);"));
    assert!(window.contains("if (tachyon_menus_command(window, LOWORD(wparam))) return 0;"));
    let command = WindowsHostTests::section(&source, "static BOOL tachyon_menus_command(");
    assert!(command.contains("if (!tachyon_menu_enabled(item)) MessageBeep(MB_OK);"));
    let key = WindowsHostTests::section(&source, "static BOOL tachyon_menus_key(");
    assert!(key.contains("if (g_menu_accelerators == NULL) return FALSE;"));
    assert!(!source.contains("__NATIVE_MENU"));
}

#[test]
fn windows_host_always_shows_file_edit_and_window_menus_sent_to_the_web_view() {
    let source = WindowsHostTests::host(NativeMenuConfig::default());
    assert!(source.contains("static const int TACHYON_MENU_ITEM_COUNT = 0;"));
    let install = WindowsHostTests::section(&source, "static void tachyon_menus_install(");
    assert!(!install.contains("TACHYON_MENU_ITEM_COUNT == 0"));
    assert!(install.contains("L\"&Edit\""));
    assert!(install.contains("L\"&Window\""));
    assert!(source.contains("{\"Undo\", \"Redo\", \"Cut\", \"Copy\", \"Paste\", \"SelectAll\"}"));
    assert!(source.contains("L\"&Redo\\tCtrl+Y\""));
    let command = WindowsHostTests::section(&source, "static BOOL tachyon_menus_command(");
    assert!(command.contains("ShowWindow(window, SW_MINIMIZE);"));
    assert!(command.contains("ShowWindow(window, IsZoomed(window) ? SW_RESTORE : SW_MAXIMIZE);"));
    assert!(command.contains("if (!tachyon_menu_edit(edit)) MessageBeep(MB_OK);"));
    let edit = WindowsHostTests::section(
        &source,
        "static BOOL tachyon_menu_edit(const char *command) {",
    );
    assert!(edit.contains("ICoreWebView2Controller_MoveFocus(g_controller, COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC);"));
    assert!(edit.contains("L\"Input.dispatchKeyEvent\""));
    assert!(edit.contains("\\\"commands\\\":[\\\"%hs\\\"]"));
}

#[test]
fn windows_host_gives_control_shortcuts_the_ctrl_accelerator() {
    let menus = serde_json::from_value(json!({
        "view": [
            {"id":"find","label":"Find","route":"/help","shortcut":{"key":"k","modifiers":["command","control","shift"]}}
        ]
    }))
    .expect("menu declarations");
    let source = WindowsHostTests::host(menus);
    assert!(source.contains("{\"find\", \"Find\", TACHYON_MENU_VIEW, \"/help\", NULL, 'k', 5}"));
}

#[test]
fn windows_host_starts_the_application_before_its_window_and_stops_it_once_after_the_loop() {
    let source = WindowsHostTests::host(NativeMenuConfig::default());
    let main = WindowsHostTests::section(&source, "int WINAPI wWinMain(");
    let start = main.find("!g_application_start()").expect("start");
    assert!(main.find("tachyon_companion_load();").expect("load") < start);
    assert!(start < main.find("CreateWindowExW").expect("window"));
    assert!(
        start
            < main
                .find("tachyon_record(\"controller.created\");")
                .expect("created")
    );
    assert!(
        main.find("GetMessageW").expect("loop")
            < main
                .find("tachyon_application_stop();\n  tachyon_menus_destroy();")
                .expect("stop")
    );
    let window = WindowsHostTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    assert!(
        window.contains("if (wparam) { tachyon_companion_dispose(); tachyon_application_stop(); }")
    );
    let navigation = WindowsHostTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE navigation_invoke(",
    );
    assert!(!navigation.contains("tachyon_application_stop"));
    let load = WindowsHostTests::section(&source, "static void tachyon_companion_load(void) {");
    assert!(load.contains("GetProcAddress(rust, \"tac_native_application_start\")"));
    let stop = WindowsHostTests::section(&source, "static void tachyon_application_stop(void) {");
    assert!(stop.contains("if (g_application_stopped) return;"));
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler; checks the Win32 menu code only, not WebView2 or MSVC"]
async fn windows_menu_code_compiles_and_links_against_the_win32_headers() {
    let root = tempfile::tempdir().expect("fixture");
    for (name, menus) in [
        ("declared", WindowsHostTests::menus()),
        ("empty", NativeMenuConfig::default()),
    ] {
        let source = root.path().join(format!("{name}.c"));
        std::fs::write(&source, WindowsHostTests::harness(&menus)).expect("harness");
        let executable = root.path().join(format!("{name}.exe"));
        super::host::run_tool(
            WindowsHostTests::MINGW,
            &[
                "-std=gnu17",
                "-O2",
                "-Wall",
                "-Wextra",
                "-Werror",
                source.to_str().expect("source"),
                "-luser32",
                "-o",
                executable.to_str().expect("executable"),
            ],
        )
        .await
        .expect("Win32 menu code compiles and links");
    }
}

#[test]
fn windows_host_sends_the_application_state_on_activation_and_minimize() {
    let source = WindowsHostTests::host(NativeMenuConfig::default());
    let window = WindowsHostTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    assert!(
        window.contains("case WM_ACTIVATEAPP:\n      tachyon_application_state_send(!wparam);")
    );
    assert!(window.contains("tachyon_application_state_resized(window, wparam);"));
    let resized =
        WindowsHostTests::section(&source, "static void tachyon_application_state_resized(");
    assert!(resized.contains("if (kind == SIZE_MINIMIZED) tachyon_application_state_send(TRUE);"));
    assert!(source.contains("L\"{\\\"applicationState\\\":\\\"background\\\"}\""));
    assert!(source.contains(
        "if(typeof e.data?.applicationState==='string')globalThis.__tachyonApplicationState(e.data.applicationState)"
    ));
    assert!(!source.contains("__APPLICATION_STATE__"));
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler; checks the application state code against the Win32 headers, not WebView2 or MSVC"]
async fn windows_application_state_code_compiles_against_the_win32_headers() {
    let root = tempfile::tempdir().expect("fixture");
    let harness = format!(
        "#define UNICODE\n#include <windows.h>\n#include <wchar.h>\ntypedef struct ICoreWebView2 ICoreWebView2;\nstatic ICoreWebView2 *g_webview = NULL;\nstatic int g_posted = 0;\nstatic void ICoreWebView2_PostWebMessageAsJson(ICoreWebView2 *view, const wchar_t *message) {{ (void)view; g_posted += message[0] == L'{{'; }}\nstatic const char *tachyon_current_route(void) {{ return \"/\"; }}\nstatic void tachyon_record(const char *event) {{ (void)event; }}\n{}\nint main(void) {{ tachyon_application_state_resized(NULL, SIZE_MINIMIZED); tachyon_application_state_send(FALSE); return g_posted; }}\n",
        include_str!("windows_application_state.c")
    );
    let source = root.path().join("state.c");
    std::fs::write(&source, harness).expect("harness");
    let executable = root.path().join("state.exe");
    super::host::run_tool(
        WindowsHostTests::MINGW,
        &[
            "-std=gnu17",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            source.to_str().expect("source"),
            "-luser32",
            "-o",
            executable.to_str().expect("executable"),
        ],
    )
    .await
    .expect("the application state code compiles and links");
}

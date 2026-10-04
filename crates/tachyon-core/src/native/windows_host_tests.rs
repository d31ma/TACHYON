#![allow(clippy::expect_used)]

use super::native_menu_config::NativeMenuConfig;
use serde_json::json;

struct WindowsHostTests;

impl WindowsHostTests {
    #[cfg(unix)]
    const MINGW: &str = "x86_64-w64-mingw32-gcc";
    #[cfg(unix)]
    const HARNESS_HEAD: &str = r"#define UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <ctype.h>
#include <stdbool.h>
#include <stdio.h>

static HWND g_window = NULL;
static BOOL tachyon_menu_window_open(void) { return g_window != NULL; }
static BOOL tachyon_menu_page_shown(void) { return g_window != NULL; }
static void tachyon_menu_open_route(const char *route) { (void)route; }
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
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus,
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
        super::windows::c_source(&application, &index, &[])
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    #[cfg(unix)]
    fn harness(menus: &NativeMenuConfig) -> String {
        let block = include_str!("windows_menus.c").replace(
            "__NATIVE_MENU_ITEMS__",
            &super::desktop_menu_items::DesktopMenuItems::c_table(menus, '&'),
        );
        format!("{}\n{block}\n{}", Self::HARNESS_HEAD, Self::HARNESS_MAIN)
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

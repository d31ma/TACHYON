#![allow(clippy::expect_used)]

use super::config::NativeApplication;

struct WindowsFilePickerTests;

impl WindowsFilePickerTests {
    const CHECK: &str = include_str!("../../tests/fixtures/native/windows_file_picker_check.c");
    #[cfg(windows)]
    const RUN_LIMIT: std::time::Duration = std::time::Duration::from_mins(2);

    fn host(grants: &[&str]) -> String {
        let application = NativeApplication {
            name: String::from("File Picker Check"),
            executable_name: String::from("FilePickerCheck"),
            application_id: String::from("org.tachyon.file-picker-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: grants.iter().copied().map(String::from).collect(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: super::native_rust_project::NativeRustProject::default(),
            outgoing: super::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: super::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: super::minimum_system::MinimumSystem::default(),
            description: String::new(),
            tray: None,
            links: super::native_links::NativeLinks::default(),
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

    fn harness() -> String {
        let json = include_str!("local_bundle.h")
            .split("static size_t tachyon_skip_space")
            .nth(1)
            .expect("JSON field parser");
        Self::CHECK
            .replace(
                "__LOCAL_JSON__",
                &format!("static size_t tachyon_skip_space{json}"),
            )
            .replace(
                "__WINDOW_PAYLOAD__",
                include_str!("native_window_payload.h"),
            )
            .replace(
                "__FILE_PICKER__",
                &include_str!("windows_file_picker.c").replace("__FILE_PICKER_GRANTED__", "TRUE"),
            )
            .replace("__PAGE_MESSAGES__", include_str!("windows_page_messages.c"))
    }

    #[cfg(unix)]
    async fn under_wine(mode: &str) -> String {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("picker.c");
        std::fs::write(&source, Self::harness()).expect("harness");
        let executable = root.path().join("picker.exe");
        super::host::run_tool(
            super::windows_host_tests::WindowsHostTests::MINGW_FOR_WINE,
            &[
                "-std=gnu17",
                "-O2",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-Wno-unused-function",
                "-municode",
                source.to_str().expect("source"),
                "-lole32",
                "-lshlwapi",
                "-luuid",
                "-luser32",
                "-o",
                executable.to_str().expect("executable"),
            ],
        )
        .await
        .expect("the Windows file picker compiles and links against Win32");
        let prefix = root.path().join("wine");
        let prefix = prefix.to_str().expect("prefix");
        super::host::run_tool_with(
            "xvfb-run",
            &["-a", "wine", executable.to_str().expect("executable"), mode],
            Some(root.path()),
            &[
                ("WINEPREFIX", prefix),
                ("WINEDEBUG", "-all"),
                ("LANG", "C.UTF-8"),
            ],
        )
        .await
        .expect("the Windows file picker check runs under Wine")
    }

    #[cfg(windows)]
    fn with_msvc(mode: &str) -> String {
        let root = tempfile::tempdir().expect("fixture");
        let source = root.path().join("picker.c");
        std::fs::write(&source, Self::harness()).expect("harness");
        let program = root.path().join("picker.exe");
        let compiled = std::process::Command::new("cl")
            .current_dir(root.path())
            .args(["/nologo", "/std:c17", "/W4", "/WX", "/wd4505", "/utf-8"])
            .arg(&source)
            .arg(format!("/Fe:{}", program.display()))
            .args(["ole32.lib", "shlwapi.lib", "uuid.lib", "user32.lib"])
            .output()
            .expect("cl");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stdout)
        );
        let mut child = std::process::Command::new(&program)
            .current_dir(root.path())
            .arg(mode)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("file picker check");
        let limit = std::time::Instant::now() + Self::RUN_LIMIT;
        while child.try_wait().expect("file picker check").is_none() {
            if std::time::Instant::now() >= limit {
                child.kill().expect("stop the file picker check");
                panic!(
                    "the file picker check ran past {:?}; a session without a desktop (SSH) never returns from the file dialog, and on a desktop the dialog never took the typed file",
                    Self::RUN_LIMIT
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let ran = child.wait_with_output().expect("file picker check");
        let printed = String::from_utf8_lossy(&ran.stdout).into_owned();
        assert!(
            ran.status.success(),
            "{printed}{}",
            String::from_utf8_lossy(&ran.stderr)
        );
        printed
    }
}

#[test]
fn the_windows_host_routes_file_picker_calls_from_the_page_and_linked_services() {
    let source = WindowsFilePickerTests::host(&["filePicker"]);
    let adapter = WindowsFilePickerTests::section(
        &source,
        "static unsigned char *tachyon_native_capability(",
    );
    assert!(adapter.contains(
        "else if (strncmp(capability, \"filePicker.\", 11) == 0) answer = tachyon_file_picker(capability + 11, payload);"
    ));
    let page = WindowsFilePickerTests::section(&source, "static char *tachyon_handle(");
    assert!(page.contains(
        "if (strncmp(capability, \"filePicker.\", 11) == 0) return tachyon_file_picker(capability + 11, payload);"
    ));
    assert!(source.contains("static BOOL g_file_picker_granted = TRUE;"));
    assert!(source.contains(
        "globalThis.__tachyonNativeFilePicker=true;globalThis.__tachyonNativeFilePickerRoute=true;globalThis.__tachyonNativeGrants=[\\\"filePicker\\\"];"
    ));
    assert!(source.contains("#include <shobjidl.h>"));
    assert!(super::windows::WindowsHostGenerator::LINKER_ARGUMENTS.contains(&"uuid.lib"));
    assert!(!source.contains("__FILE_PICKER"));
    assert!(!source.contains("__PAGE_MESSAGES__"));
    assert!(!source.contains("__NATIVE_GRANTS__"));
}

#[test]
fn the_windows_host_answers_webassembly_pickers_through_the_page_route() {
    let source = WindowsFilePickerTests::host(&["filePicker"]);
    let resource = WindowsFilePickerTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE resource_invoke(",
    );
    assert!(resource.contains("BOOL routed = tachyon_capability_route(request, arguments, text);"));
    let route = WindowsFilePickerTests::section(&source, "static BOOL tachyon_capability_route(");
    assert!(
        route.contains(
            "ICoreWebView2WebResourceRequestedEventArgs_GetDeferral(arguments, &deferral)"
        )
    );
    assert!(route.contains("PostMessageW(g_window, TACHYON_WM_CAPABILITY, 0, 0);"));
    assert!(!route.contains("tachyon_native_capability("));
    let body =
        WindowsFilePickerTests::section(&source, "static char *tachyon_capability_route_body(");
    assert!(body.contains("content->lpVtbl->Read(content, buffer + total,"));
    assert!(
        !source.contains("IStream_Read("),
        "MSVC's shlwapi.h turns IStream_Read into a three-argument function"
    );
    let allowed =
        WindowsFilePickerTests::section(&source, "static BOOL tachyon_capability_route_allowed(");
    for check in [
        "wcscmp(method, L\"POST\") == 0",
        "strncmp(capability, \"filePicker.\", 11) == 0",
        "tachyon_current_route() != NULL",
        "tachyon_capability_route_header(request, capability)",
    ] {
        assert!(allowed.contains(check), "{check}");
    }
    let run = WindowsFilePickerTests::section(&source, "static void tachyon_capability_route_run(");
    assert!(run.contains("if (tachyon_page_busy()) { tachyon_capability_route_finish("));
    assert!(run.contains("tachyon_native_capability(g_capability_call.capability, g_capability_call.payload, &length);"));
    let procedure =
        WindowsFilePickerTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    assert!(procedure.contains(
        "    case TACHYON_WM_CAPABILITY:\n      tachyon_capability_route_run();\n      return 0;"
    ));
    assert!(source.contains(
        "#define TACHYON_CAPABILITY_ROUTE \"https://tachyon.local/__tachyon/capability/\""
    ));
    assert!(!source.contains("__CAPABILITY_ROUTE__"));
}

#[test]
fn a_windows_host_without_the_grant_denies_the_picker() {
    let source = WindowsFilePickerTests::host(&[]);
    assert!(source.contains("static BOOL g_file_picker_granted = FALSE;"));
    assert!(source.contains("globalThis.__tachyonNativeGrants=[];"));
    let picker = WindowsFilePickerTests::section(&source, "static char *tachyon_file_picker(");
    assert!(picker.contains(
        "if (!g_file_picker_granted) { tachyon_file_picker_dispose(owner); return tachyon_file_picker_answer(\"denied\"); }"
    ));
}

#[test]
fn the_windows_host_answers_page_calls_outside_webview2_events_and_disposes_picks() {
    let source = WindowsFilePickerTests::host(&["filePicker"]);
    let received = WindowsFilePickerTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE message_invoke(",
    );
    assert!(received.contains("tachyon_page_message_hold(message);"));
    assert!(!received.contains("tachyon_on_message("));
    let procedure =
        WindowsFilePickerTests::section(&source, "static LRESULT CALLBACK tachyon_window_proc(");
    assert!(procedure.contains(
        "    case TACHYON_WM_PAGE_MESSAGE:\n      tachyon_page_messages_run();\n      return 0;"
    ));
    assert!(procedure.contains(
        "    case WM_DESTROY:\n      tachyon_capability_route_finish(\"{\\\"status\\\":\\\"unavailable\\\"}\");\n      tachyon_page_messages_clear();\n      tachyon_companion_dispose();\n      tachyon_file_picker_dispose_all();"
    ));
    let navigation = WindowsFilePickerTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE navigation_invoke(",
    );
    assert!(navigation.contains(
        "} else { tachyon_page_messages_clear(); tachyon_companion_dispose(); tachyon_file_picker_dispose_all(); }"
    ));
    assert!(
        source.contains(
            "capability==='filePicker.open'||capability==='companion.invoke'?40000:10000"
        )
    );
}

#[test]
fn the_windows_file_picker_check_fills_every_placeholder() {
    let harness = WindowsFilePickerTests::harness();
    assert!(!harness.contains("__"), "{harness}");
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the MinGW cross compiler for its architecture (x86_64-w64-mingw32-gcc, or aarch64-w64-mingw32-gcc from llvm-mingw on arm64 Linux), Wine and xvfb-run; checks the picker contract and page message order against Win32, not WebView2 or MSVC"]
async fn the_windows_file_picker_keeps_the_contract_under_wine() {
    let output = WindowsFilePickerTests::under_wine("check").await;
    assert!(
        output.contains("windows file picker contract passed"),
        "{output}"
    );
    assert!(output.contains("windows page messages passed"), "{output}");
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the MinGW cross compiler for its architecture (x86_64-w64-mingw32-gcc, or aarch64-w64-mingw32-gcc from llvm-mingw on arm64 Linux), Wine and xvfb-run; drives Wine's IFileOpenDialog, not the Windows one"]
async fn the_windows_file_picker_selects_a_file_in_the_wine_dialog() {
    let output = WindowsFilePickerTests::under_wine("dialog").await;
    assert!(output.contains("windows file dialog passed"), "{output}");
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows and MSVC cl on PATH (vcvars); checks the picker contract and page message order, not WebView2"]
fn the_windows_file_picker_keeps_the_contract_with_msvc() {
    let output = WindowsFilePickerTests::with_msvc("check");
    assert!(
        output.contains("windows file picker contract passed"),
        "{output}"
    );
    assert!(output.contains("windows page messages passed"), "{output}");
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows, MSVC cl on PATH (vcvars) and a desktop that can show the file dialog; drives the real IFileOpenDialog, not WebView2"]
fn the_windows_file_picker_selects_a_file_in_the_windows_dialog() {
    let output = WindowsFilePickerTests::with_msvc("dialog");
    assert!(output.contains("windows file dialog passed"), "{output}");
}

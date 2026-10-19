#![allow(clippy::expect_used)]

use super::browser_api_permissions::BrowserApiPermissions;
use super::windows_browser_apis::WindowsBrowserApis;
use std::process::Command;

struct WindowsBrowserApiTests;

impl WindowsBrowserApiTests {
    const ALL: [&'static str; 4] = ["clipboard", "geolocation", "notifications", "share"];
    const HARNESS_HEAD: &'static str = r#"#define UNICODE
#define _UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <objbase.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

typedef enum {
  COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION = 0,
  COREWEBVIEW2_PERMISSION_KIND_MICROPHONE = 1,
  COREWEBVIEW2_PERMISSION_KIND_CAMERA = 2,
  COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION = 3,
  COREWEBVIEW2_PERMISSION_KIND_NOTIFICATIONS = 4,
  COREWEBVIEW2_PERMISSION_KIND_OTHER_SENSORS = 5,
  COREWEBVIEW2_PERMISSION_KIND_CLIPBOARD_READ = 6
} COREWEBVIEW2_PERMISSION_KIND;
typedef enum {
  COREWEBVIEW2_PERMISSION_STATE_DEFAULT = 0,
  COREWEBVIEW2_PERMISSION_STATE_ALLOW = 1,
  COREWEBVIEW2_PERMISSION_STATE_DENY = 2
} COREWEBVIEW2_PERMISSION_STATE;

#define TACHYON_APP_NAME "Notes"
static HWND g_window = NULL;

static char *tachyon_narrow(const wchar_t *value) {
  int length = WideCharToMultiByte(CP_UTF8, 0, value, -1, NULL, 0, NULL, NULL);
  char *narrow = length > 0 ? (char *)calloc((size_t)length, sizeof(char)) : NULL;
  if (narrow != NULL) WideCharToMultiByte(CP_UTF8, 0, value, -1, narrow, length, NULL, NULL);
  return narrow;
}

static wchar_t *tachyon_widen(const char *value) {
  int length = MultiByteToWideChar(CP_UTF8, 0, value, -1, NULL, 0);
  wchar_t *wide = length > 0 ? (wchar_t *)calloc((size_t)length, sizeof(wchar_t)) : NULL;
  if (wide != NULL) MultiByteToWideChar(CP_UTF8, 0, value, -1, wide, length);
  return wide;
}
"#;
    #[cfg(unix)]
    const MINGW: &'static str = "x86_64-w64-mingw32-gcc";
    #[cfg(unix)]
    const WINE: &'static str = "wine";

    fn permissions(granted: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    fn host(granted: &[&str]) -> String {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(granted);
        super::windows::c_source(
            &application,
            &super::linux_host_tests::LinuxHostTests::index(),
            &[],
        )
    }

    fn harness(granted: &[&str]) -> String {
        [
            Self::HARNESS_HEAD,
            &super::routes::c_local_bundle(
                &super::linux_host_tests::LinuxHostTests::index(),
                &[],
                "https://tachyon.local",
            ),
            include_str!("native_window_payload.h"),
            &WindowsBrowserApis::source(&Self::permissions(granted)),
            include_str!("windows_browser_api_fixture.c"),
        ]
        .join("\n")
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

    fn cases() -> [(&'static str, &'static [&'static str], &'static str); 2] {
        [
            ("granted", &Self::ALL, "TACHYON_EXPECT_GRANTED"),
            ("refused", &[], "TACHYON_EXPECT_REFUSED"),
        ]
    }

    #[cfg(unix)]
    fn cross_compiled(
        root: &std::path::Path,
        name: &str,
        granted: &[&str],
        define: &str,
    ) -> std::path::PathBuf {
        let source = root.join(format!("{name}.c"));
        let program = root.join(format!("{name}.exe"));
        std::fs::write(&source, Self::harness(granted)).expect("source");
        Self::success(
            Command::new(Self::MINGW)
                .args(["-std=gnu17", "-Wall", "-Wextra", "-Werror", "-O2"])
                .arg(format!("-D{define}"))
                .arg(&source)
                .args(["-lole32", "-luser32", "-o"])
                .arg(&program),
        );
        program
    }
}

#[test]
fn the_windows_host_checks_only_the_browser_apis_the_manifest_grants() {
    let nothing = WindowsBrowserApis::source(&WindowsBrowserApiTests::permissions(&[]));
    assert!(!nothing.contains("if (strcmp(api, "));
    assert!(!nothing.contains("__GRANTED_BROWSER_APIS__"));
    for api in WindowsBrowserApiTests::ALL {
        let source = WindowsBrowserApis::source(&WindowsBrowserApiTests::permissions(&[api]));
        assert!(source.contains(&format!("if (strcmp(api, \"{api}\") == 0) return TRUE;")));
        assert_eq!(source.matches("if (strcmp(api, ").count(), 1);
    }
}

#[test]
fn the_windows_host_decides_web_view_permissions_and_answers_bridged_calls() {
    let source = WindowsBrowserApiTests::host(&WindowsBrowserApiTests::ALL);
    assert!(!source.contains("__NATIVE_BROWSER_APIS__"));
    assert!(!source.contains("__GRANTED_BROWSER_APIS__"));
    assert!(source.contains(
        "if (strncmp(capability, \"browserApi.\", 11) == 0) return tachyon_browser_api(capability, payload);"
    ));
    assert!(source.contains(
        "ICoreWebView2_add_PermissionRequested(g_webview, (ICoreWebView2PermissionRequestedEventHandler *)&g_permission_handler, &token);"
    ));
    assert!(source.contains("L\"{15e1c6a3-c72a-4df3-91d7-d097fbec6bfd}\""));
    assert!(source.contains(
        "BOOL own_page = text != NULL && tachyon_local_path(text, path, sizeof(path)) && tachyon_current_route() != NULL;"
    ));
    assert!(source.contains(
        "ICoreWebView2PermissionRequestedEventArgs_put_State(arguments, tachyon_permission_state(kind, own_page));"
    ));
    for operation in [
        "browserApi.clipboard.writeText",
        "browserApi.clipboard.readText",
        "browserApi.share.open",
        "browserApi.notifications.requestPermission",
        "browserApi.notifications.show",
    ] {
        assert!(source.contains(&format!(
            "if (strcmp(capability, \"{operation}\") == 0) return "
        )));
    }
    assert!(source.contains("SetClipboardData(CF_UNICODETEXT, memory)"));
    assert!(source.contains("interop->lpVtbl->ShowShareUIForWindow(interop, window)"));
    assert!(source.contains("GetProcAddress(kernel, \"GetCurrentApplicationUserModelId\")"));
    assert!(
        source.contains(
            "manager->lpVtbl->CreateToastNotifierWithId(manager, id, (void **)&notifier)"
        )
    );
}

#[test]
fn the_share_handler_answers_the_typed_event_handler_interface_windows_computes() {
    let source = WindowsBrowserApis::SOURCE;
    assert!(
        source.contains(
            "#define TACHYON_SHARE_REQUESTED L\"{ec6f9cc8-46d0-5e0e-b4d2-7d7773ae37a0}\""
        )
    );
    assert!(source.contains("_wcsicmp(identifier, TACHYON_SHARE_REQUESTED) != 0"));
    assert!(source.contains("_wcsicmp(identifier, TACHYON_AGILE_OBJECT) != 0"));
}

#[test]
fn the_browser_api_harness_fills_every_placeholder() {
    for (_, granted, _) in WindowsBrowserApiTests::cases() {
        let harness = WindowsBrowserApiTests::harness(granted);
        assert!(!harness.contains("__GRANTED_BROWSER_APIS__"));
        assert!(harness.contains(
            "static char *tachyon_browser_api(const char *capability, const char *payload) {"
        ));
    }
}

#[cfg(unix)]
#[test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and wine; runs the bridge with fake toasts and share sheet, not WebView2, MSIX or the real share sheet"]
fn the_windows_browser_apis_follow_the_manifest_grants_under_wine() {
    let root = tempfile::tempdir().expect("fixture");
    for (name, granted, define) in WindowsBrowserApiTests::cases() {
        let program = WindowsBrowserApiTests::cross_compiled(root.path(), name, granted, define);
        let output = WindowsBrowserApiTests::success(
            Command::new(WindowsBrowserApiTests::WINE)
                .env("WINEDEBUG", "-all")
                .env("WINEPREFIX", root.path().join(format!("wine-{name}")))
                .arg(&program),
        );
        println!("{name}: {}", output.trim());
        assert!(output.contains("windows browser APIs passed"), "{output}");
    }
}

#[cfg(windows)]
#[test]
#[ignore = "Requires Windows and MSVC cl on PATH (vcvars); checks the runtime interface slots against the Windows SDK and loads toast XML and a Uri, not WebView2 or MSIX"]
fn the_windows_browser_apis_match_the_windows_sdk_and_run_on_windows() {
    let root = tempfile::tempdir().expect("fixture");
    for (name, granted, define) in WindowsBrowserApiTests::cases() {
        let source = root.path().join(format!("{name}.c"));
        let program = root.path().join(format!("{name}.exe"));
        std::fs::write(&source, WindowsBrowserApiTests::harness(granted)).expect("source");
        print!(
            "{}",
            WindowsBrowserApiTests::success(
                Command::new("cl")
                    .current_dir(root.path())
                    .args(["/nologo", "/std:c17", "/W4", "/WX"])
                    .arg(format!("/D{define}"))
                    .arg("/DTACHYON_CHECK_RUNTIME_LAYOUT")
                    .arg(&source)
                    .arg(format!("/Fe:{}", program.display()))
                    .args(["ole32.lib", "user32.lib"]),
            )
        );
        let output = WindowsBrowserApiTests::success(&mut Command::new(&program));
        println!("{}", output.trim());
        assert!(output.contains("windows browser APIs passed"), "{output}");
    }
}

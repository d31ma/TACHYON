#![allow(clippy::expect_used)]

struct WindowsTaskResumeCheck;

impl WindowsTaskResumeCheck {
    #[cfg(unix)]
    const MINGW: &str = "x86_64-w64-mingw32-gcc";
    #[cfg(unix)]
    const HARNESS_HEAD: &str = r#"#define UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdio.h>
#include <string.h>

static PVOID volatile g_publish_window = NULL;
typedef const char *(*TachyonCompanionInvoke)(const char *);
typedef void (*TachyonCompanionFree)(const char *);
static TachyonCompanionInvoke g_companion_invoke[3] = {NULL, NULL, NULL};
static TachyonCompanionFree g_companion_free[3] = {NULL, NULL, NULL};
static int g_resumed = 0;

static const char *check_invoke(const char *request) {
  if (strcmp(request, "{\"op\":\"host-resume\"}") == 0) g_resumed++;
  return NULL;
}
"#;
    #[cfg(unix)]
    const HARNESS_MAIN: &str = r#"
static LRESULT CALLBACK check_window(HWND window, UINT message, WPARAM wparam, LPARAM lparam) {
  if (message == TACHYON_WM_RESUME) {
    tachyon_task_resume();
    return 0;
  }
  return DefWindowProcW(window, message, wparam, lparam);
}

static DWORD WINAPI check_waker(LPVOID unused) {
  (void)unused;
  tachyon_task_dispatch();
  tachyon_task_dispatch();
  return 0;
}

int main(void) {
  WNDCLASSW kind = {0};
  kind.lpfnWndProc = check_window;
  kind.lpszClassName = L"TachyonTaskResumeCheck";
  kind.hInstance = GetModuleHandleW(NULL);
  RegisterClassW(&kind);
  HWND window = CreateWindowExW(0, kind.lpszClassName, L"", 0, 0, 0, 0, 0, HWND_MESSAGE, NULL, kind.hInstance, NULL);
  InterlockedExchangePointer(&g_publish_window, window);
  tachyon_task_bind(GetModuleHandleW(NULL), 1);
  g_task_bound[1] = TRUE;
  g_companion_invoke[1] = check_invoke;
  HANDLE thread = CreateThread(NULL, 0, check_waker, NULL, 0, NULL);
  WaitForSingleObject(thread, INFINITE);
  MSG message;
  int posted = 0;
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) {
    if (message.message == TACHYON_WM_RESUME) posted++;
    DispatchMessageW(&message);
  }
  tachyon_task_dispatch();
  while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) {
    if (message.message == TACHYON_WM_RESUME) posted++;
    DispatchMessageW(&message);
  }
  printf("posted=%d resumed=%d\n", posted, g_resumed);
  return posted == 2 && g_resumed == 2 ? 0 : 1;
}
"#;

    fn host() -> String {
        let application = super::config::NativeApplication {
            name: String::from("Windows Task Check"),
            executable_name: String::from("WindowsTaskCheck"),
            application_id: String::from("org.tachyon.windows-task-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: super::config::WindowConfiguration::default(),
            native_capabilities: Vec::new(),
            menus: super::native_menu_config::NativeMenuConfig::default(),
            rust: crate::native::native_rust_project::NativeRustProject::default(),
            outgoing: crate::native::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: crate::native::minimum_system::MinimumSystem::default(),
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

    #[cfg(unix)]
    fn harness() -> String {
        format!(
            "{}\n{}\n{}",
            Self::HARNESS_HEAD,
            include_str!("windows_task_resume.c"),
            Self::HARNESS_MAIN
        )
    }

    #[cfg(unix)]
    fn wine() -> bool {
        std::process::Command::new("wine64")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }
}

#[test]
fn windows_host_resumes_companion_tasks_on_its_window_thread() {
    let source = WindowsTaskResumeCheck::host();
    assert!(source.contains("#define TACHYON_WM_RESUME (WM_APP + 2)"));
    assert!(source.contains("GetProcAddress(library, \"tac_native_set_dispatch\")"));
    assert!(source.contains("  tachyon_task_bind(library, language);\n"));
    assert!(
        source.contains(
            "    case TACHYON_WM_RESUME:\n      tachyon_task_resume();\n      return 0;\n"
        )
    );
    assert!(source.contains("g_companion_invoke[language](\"{\\\"op\\\":\\\"host-resume\\\"}\")"));
    assert!(!source.contains("__TASK_RESUME__"));
    let bind = source.find("static void tachyon_task_bind(").expect("bind");
    let load = source
        .find("static HMODULE tachyon_companion_load_one(")
        .expect("load");
    assert!(
        bind < load,
        "the bind helper is declared before the loader uses it"
    );
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler; runs the harness under wine64 when it is installed"]
async fn windows_task_resume_posts_one_message_per_wake_burst() {
    let root = tempfile::tempdir().expect("fixture");
    let source = root.path().join("resume.c");
    std::fs::write(&source, WindowsTaskResumeCheck::harness()).expect("harness");
    let executable = root.path().join("resume.exe");
    super::host::run_tool(
        WindowsTaskResumeCheck::MINGW,
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
    .expect("the task resume code compiles and links");
    if !WindowsTaskResumeCheck::wine() {
        eprintln!("skipping the run: wine64 is not installed");
        return;
    }
    let prefix = root.path().join("wine");
    let mut command = tokio::process::Command::new("wine64");
    command
        .arg(&executable)
        .env("WINEPREFIX", &prefix)
        .env("WINEDEBUG", "-all");
    let ran = crate::external_command::run(&mut command, std::time::Duration::from_mins(5), 65536)
        .await
        .expect("wine64 runs the harness");
    let printed = String::from_utf8_lossy(&ran.stdout);
    assert!(
        ran.status.success(),
        "{printed} {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(printed.contains("posted=2 resumed=2"), "{printed}");
}

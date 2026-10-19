#![allow(clippy::expect_used)]

use super::native_links::NativeLinks;

struct NativeDeepLinkTests;

impl NativeDeepLinkTests {
    const ENTRY: Option<(&'static str, &'static str)> = Some(("!", ""));
    const CASES: [(&'static str, Option<(&'static str, &'static str)>); 32] = [
        ("myapp://settings", Some(("settings", ""))),
        ("myapp:///settings/", Some(("settings", ""))),
        (
            "MYAPP://settings?tab=a#top",
            Some(("settings", "?tab=a#top")),
        ),
        ("myapp:settings", Some(("settings", ""))),
        ("myapp://", Some(("", ""))),
        ("myapp:", Some(("", ""))),
        ("myapp://items/42", Some(("items/42", ""))),
        ("other://items/42?x=1", Some(("items/42", "?x=1"))),
        ("myapp://missing/page?x=1", Self::ENTRY),
        ("myapp://Settings", Self::ENTRY),
        ("myapp://../settings", Self::ENTRY),
        ("https://example.com/settings", Some(("settings", ""))),
        (
            "https://EXAMPLE.COM/items/7?ref=mail",
            Some(("items/7", "?ref=mail")),
        ),
        ("HTTPS://example.com", Some(("", ""))),
        ("https://example.com?x=1", Some(("", "?x=1"))),
        ("https://example.com/#top", Some(("", "#top"))),
        ("https://example.org/settings", None),
        ("https://example.com:443/settings", None),
        ("https://user@example.com/settings", None),
        ("https://example.com.evil.test/", None),
        ("https:example.com/settings", None),
        ("http://example.com/settings", None),
        ("myappx://settings", None),
        ("mailto:someone@example.com", None),
        ("javascript:alert(1)", None),
        ("tachyon://app/settings/", None),
        ("", None),
        ("myapp", None),
        (":settings", None),
        ("myapp://set tings", None),
        ("myapp://a\"b", None),
        ("myapp://caf\u{e9}", None),
    ];
    const C_ORIGIN: &'static str = "tachyon://app";
    const C_MAIN: &'static str = r#"
int main(void) {
  (void)tachyon_bundle_path;
  (void)tachyon_payload_route_matches;
  (void)tachyon_external_link;
  int failures = 0;
  char target[TACHYON_PATH_LIMIT];
  for (size_t index = 0; index < sizeof(TACHYON_DEEP_LINK_CASES) / sizeof(TACHYON_DEEP_LINK_CASES[0]); index++) {
    const char *link = TACHYON_DEEP_LINK_CASES[index][0], *expected = TACHYON_DEEP_LINK_CASES[index][1];
    int opens = tachyon_deep_link_target(link, target, sizeof(target));
    if (opens != (expected != NULL) || (opens && strcmp(target, expected) != 0)) {
      fprintf(stderr, "%s opened %s, expected %s\n", link, opens ? target : "nothing", expected != NULL ? expected : "nothing");
      failures++;
    }
  }
  if (tachyon_deep_link_target(NULL, target, sizeof(target))) { fprintf(stderr, "a missing link opened\n"); failures++; }
  return failures == 0 ? 0 : 1;
}
"#;
    const SWIFT_MAIN: &'static str = r#"
var failures = 0
for (link, expected) in tachyonDeepLinkCases {
    let actual = TacDeepLink.target(link)
    if actual != expected {
        print("\(link) opened \(actual ?? "nothing"), expected \(expected ?? "nothing")")
        failures += 1
    }
}
exit(failures == 0 ? 0 : 1)
"#;
    const JAVA_HEAD: &'static str = r#"public final class DeepLinkCheck {
  private static final String ASSET_ORIGIN = "https://appassets.tachyon.local/";
  private static final String ENTRY_DOCUMENT = "index.html";

  private static String canonicalRoute(String path) {
    if (path == null || path.contains("..") || path.contains("\\")) return null;
    if (path.equals("/") || path.equals("/settings")) return path;
    String[] parts = path.split("/", -1);
    return parts.length == 3 && parts[1].equals("items") && !parts[2].isEmpty() ? "/items/_id" : null;
  }
"#;
    const JAVA_MAIN: &'static str = r#"
  public static void main(String[] arguments) {
    int failures = 0;
    for (String[] test : CASES) {
      String actual = linkTarget(test[0]);
      if (actual == null ? test[1] != null : !actual.equals(test[1])) {
        System.err.println(test[0] + " opened " + actual + ", expected " + test[1]);
        failures++;
      }
    }
    System.exit(failures == 0 ? 0 : 1);
  }
}
"#;

    fn links() -> NativeLinks {
        NativeLinks::declared(&["myapp", "other"], &["example.com"])
    }

    fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: [
                ("/", "index.html"),
                ("/settings", "settings/index.html"),
                ("/items/_id", "items/_id/index.html"),
            ]
            .into_iter()
            .map(|(route, document)| super::routes::NativeRouteIndexEntry {
                route: route.to_owned(),
                document: document.to_owned(),
            })
            .collect(),
        }
    }

    fn expected(
        case: Option<(&str, &str)>,
        known: &dyn Fn(&str, &str) -> String,
        entry: &str,
    ) -> Option<String> {
        case.map(|(path, suffix)| match path {
            "!" => String::from(entry),
            _ => known(path, suffix),
        })
    }

    fn c_harness() -> String {
        let cases = Self::CASES
            .iter()
            .map(|(link, case)| {
                let expected = Self::expected(
                    *case,
                    &|path, suffix| {
                        let slash = ["/", ""][usize::from(path.is_empty())];
                        format!("{}/{path}{slash}{suffix}", Self::C_ORIGIN)
                    },
                    "tachyon://app/",
                );
                format!(
                    "  {{\"{}\", {}}},\n",
                    super::host::c_string_escape(link),
                    expected.map_or_else(|| String::from("NULL"), |text| format!("\"{text}\""))
                )
            })
            .collect::<Vec<_>>()
            .concat();
        format!(
            "#include <stdio.h>\n#include <string.h>\n{}\n{}\n#define TACHYON_ENTRY_ROUTE \"/\"\n{}\nstatic const char *const TACHYON_DEEP_LINK_CASES[][2] = {{\n{cases}}};\n{}",
            super::routes::c_local_bundle(&Self::index(), &[], Self::C_ORIGIN),
            super::native_external_link::NativeExternalLink::C_HELPER,
            super::native_deep_link::NativeDeepLink::c_helper(&Self::links()),
            Self::C_MAIN
        )
    }

    fn swift_harness() -> String {
        let host = super::macos::host_source();
        let route = host
            .split("private func tachyonRoute(")
            .nth(1)
            .and_then(|rest| rest.split("private func tachyonDocument(").next())
            .expect("the macOS route matcher");
        let cases: Vec<String> = Self::CASES
            .iter()
            .map(|(link, case)| {
                let expected = Self::expected(
                    *case,
                    &|path, suffix| format!("tachyon-app://bundle/{path}{suffix}"),
                    "tachyon-app://bundle/index.html",
                );
                format!(
                    "(\"{}\", {})",
                    super::host::quoted_string_escape(link),
                    expected.map_or_else(|| String::from("nil"), |text| format!("\"{text}\""))
                )
            })
            .collect();
        format!(
            "import Foundation\nprivate let tachyonEntryDocument = \"index.html\"\nprivate let tachyonRouteDocuments: [String: String] = {}\nprivate func tachyonRoute({route}\n{}\nlet tachyonDeepLinkCases: [(String, String?)] = [{}]\n{}",
            super::routes::swift_routes(&Self::index()),
            super::native_deep_link::NativeDeepLink::swift_helper(&Self::links()),
            cases.join(",\n"),
            Self::SWIFT_MAIN
        )
    }

    fn java_harness() -> String {
        let cases: Vec<String> = Self::CASES
            .iter()
            .map(|(link, case)| {
                let expected = Self::expected(
                    *case,
                    &|path, suffix| format!("https://appassets.tachyon.local/{path}{suffix}"),
                    "https://appassets.tachyon.local/index.html",
                );
                format!(
                    "    {{\"{}\", {}}}",
                    super::host::quoted_string_escape(link),
                    expected.map_or_else(|| String::from("null"), |text| format!("\"{text}\""))
                )
            })
            .collect();
        format!(
            "{}\n{}\n  private static final String[][] CASES = {{\n{}\n  }};\n{}",
            Self::JAVA_HEAD,
            super::native_deep_link::NativeDeepLink::java_helper(&Self::links()),
            cases.join(",\n"),
            Self::JAVA_MAIN
        )
    }

    #[cfg(unix)]
    fn run(program: &str, arguments: &[&std::ffi::OsStr], directory: &std::path::Path) {
        let output = std::process::Command::new(program)
            .args(arguments)
            .current_dir(directory)
            .output()
            .expect("run the deep link check");
        assert!(
            output.status.success(),
            "{program}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn linked_application() -> super::config::NativeApplication {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.links = Self::links();
        application
    }
}

#[cfg(unix)]
#[test]
fn the_c_link_rule_opens_declared_schemes_and_hosts_at_their_route() {
    let directory = tempfile::tempdir().expect("deep link fixture");
    let source = directory.path().join("links.c");
    let binary = directory.path().join("links");
    std::fs::write(&source, NativeDeepLinkTests::c_harness()).expect("fixture");
    NativeDeepLinkTests::run(
        "cc",
        &[
            "-std=gnu17".as_ref(),
            "-Wall".as_ref(),
            "-Wextra".as_ref(),
            "-Werror".as_ref(),
            source.as_os_str(),
            "-o".as_ref(),
            binary.as_os_str(),
        ],
        directory.path(),
    );
    NativeDeepLinkTests::run(binary.to_str().expect("binary"), &[], directory.path());
}

#[cfg(unix)]
#[test]
#[ignore = "Requires javac and java; checks the Android link rule against the shared cases"]
fn the_android_link_rule_opens_declared_schemes_and_hosts_at_their_route() {
    let directory = tempfile::tempdir().expect("deep link fixture");
    let source = directory.path().join("DeepLinkCheck.java");
    std::fs::write(&source, NativeDeepLinkTests::java_harness()).expect("fixture");
    NativeDeepLinkTests::run(
        "javac",
        &["-encoding".as_ref(), "UTF-8".as_ref(), source.as_os_str()],
        directory.path(),
    );
    NativeDeepLinkTests::run(
        "java",
        &["-cp".as_ref(), ".".as_ref(), "DeepLinkCheck".as_ref()],
        directory.path(),
    );
}

#[cfg(unix)]
#[test]
#[ignore = "Requires swiftc; checks the Apple link rule with the macOS route matcher against the shared cases"]
fn the_apple_link_rule_opens_declared_schemes_and_hosts_at_their_route() {
    let directory = tempfile::tempdir().expect("deep link fixture");
    let source = directory.path().join("main.swift");
    let binary = directory.path().join("links");
    std::fs::write(&source, NativeDeepLinkTests::swift_harness()).expect("fixture");
    NativeDeepLinkTests::run(
        "swiftc",
        &[source.as_os_str(), "-o".as_ref(), binary.as_os_str()],
        directory.path(),
    );
    NativeDeepLinkTests::run(binary.to_str().expect("binary"), &[], directory.path());
}

#[test]
fn the_link_rule_harnesses_carry_every_case() {
    for harness in [
        NativeDeepLinkTests::c_harness(),
        NativeDeepLinkTests::swift_harness(),
        NativeDeepLinkTests::java_harness(),
    ] {
        assert!(!harness.contains("__LINK_"));
        assert!(harness.contains("example.com.evil.test"));
    }
}

#[test]
fn apple_hosts_register_the_schemes_and_open_links_at_their_route() {
    let application = NativeDeepLinkTests::linked_application();
    let index = NativeDeepLinkTests::index();
    let mac = super::macos::swift_source(&application, "tacNativeInvoke(payload)", &index);
    assert!(mac.contains("static let schemes: Set<String> = [\"myapp\", \"other\"]"));
    assert!(mac.contains(
        "forEventClass: AEEventClass(kInternetEventClass), andEventID: AEEventID(kAEGetURL))"
    ));
    assert!(mac.contains("let target = TacDeepLink.target(link)"));
    assert!(mac.contains("activity.activityType == NSUserActivityTypeBrowsingWeb"));
    assert!(mac.contains("return open(link: link)"));
    assert!(!mac.contains("__NATIVE_DEEP_LINKS__"));
    let ios = super::ios::swift_source(&application, "tacNativeInvoke(payload)", &index);
    assert!(ios.contains("static let hosts: Set<String> = [\"example.com\"]"));
    assert!(ios.contains("func application(_ application: UIApplication, open url: URL"));
    assert!(ios.contains("activity.activityType == NSUserActivityTypeBrowsingWeb"));
    assert!(ios.contains("if let link = options?[.url] as? URL"));
    assert!(!ios.contains("__NATIVE_DEEP_LINKS__"));
}

#[test]
fn android_claims_the_schemes_and_verified_hosts_and_opens_them_at_their_route() {
    let application = NativeDeepLinkTests::linked_application();
    let source = super::android::java_source(
        &application,
        "org.tachyon.check",
        false,
        &NativeDeepLinkTests::index(),
    );
    assert!(
        source.contains("private static final String[] LINK_SCHEMES = {\"myapp\", \"other\"};")
    );
    assert!(source.contains("protected void onNewIntent(Intent intent) {"));
    assert!(source.contains("String linked = intentTarget(getIntent());"));
    assert!(source.contains("String destination = org.json.JSONObject.quote(launchTarget);"));
    assert!(!source.contains("__NATIVE_DEEP_LINKS__"));
}

#[test]
fn desktop_hosts_register_the_schemes_and_forward_links_to_the_running_app() {
    let application = NativeDeepLinkTests::linked_application();
    let index = super::linux_host_tests::LinuxHostTests::index();
    let linux = super::linux::c_source(&application, &index);
    assert!(linux.contains(
        "static const char *const TACHYON_LINK_SCHEMES[] = {\"myapp\", \"other\", NULL};"
    ));
    assert!(linux.contains(
        "gtk_application_new(\"org.tachyon.linux_host_check\", G_APPLICATION_HANDLES_OPEN)"
    ));
    assert!(
        linux.contains("g_signal_connect(app, \"open\", G_CALLBACK(tachyon_open_links), NULL);")
    );
    assert!(!linux.contains("__DEEP_LINK_HELPER__"));
    let windows = super::windows::c_source(&application, &index, &[]);
    assert!(windows.contains("if (tachyon_forward_link()) return 0;"));
    assert!(windows.contains("RegSetKeyValueW(HKEY_CURRENT_USER, key, L\"URL Protocol\""));
    assert!(windows.contains("tachyon_links_listen(instance);"));
    assert!(windows.contains("FindWindowExW(HWND_MESSAGE, NULL, TACHYON_LINK_RECEIVER, bundle)"));
    assert!(!windows.contains("__WINDOWS_DEEP_LINKS__"));
    assert!(!windows.contains("__DEEP_LINK_HELPER__"));
}

#[test]
fn hosts_without_links_claim_no_scheme() {
    let application = super::linux_host_tests::LinuxHostTests::application(
        super::native_menu_config::NativeMenuConfig::default(),
    );
    let index = super::linux_host_tests::LinuxHostTests::index();
    let linux = super::linux::c_source(&application, &index);
    assert!(linux.contains("static const char *const TACHYON_LINK_SCHEMES[] = {NULL};"));
    let windows = super::windows::c_source(&application, &index, &[]);
    assert!(windows.contains("static const char *const TACHYON_LINK_HOSTS[] = {NULL};"));
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and Wine; runs the Windows scheme registration and link forwarding against the Win32 API, not WebView2 or MSVC"]
async fn a_second_windows_instance_forwards_its_link_to_the_first() {
    let root = tempfile::tempdir().expect("fixture");
    let source = root.path().join("links.c");
    let harness = format!(
        "#define WIN32_LEAN_AND_MEAN\n#include <windows.h>\n#include <shellapi.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n{}\n{}\n#define TACHYON_APP_NAME \"Links\"\n#define TACHYON_BUNDLE_ID \"org.tachyon.links\"\n#define TACHYON_ENTRY_ROUTE \"/\"\n{}\n{}\n{}\n{}\n{}",
        super::routes::c_local_bundle(&NativeDeepLinkTests::index(), &[], "https://tachyon.local"),
        super::native_external_link::NativeExternalLink::C_HELPER,
        super::native_deep_link::NativeDeepLink::c_helper(&NativeDeepLinkTests::links()),
        r"typedef struct ICoreWebView2 ICoreWebView2;
static ICoreWebView2 *g_webview = NULL;
static HWND g_window = NULL;
static wchar_t g_navigated[4096];
#define ICoreWebView2_Navigate(view, uri) ((void)(view), wcscpy_s(g_navigated, 4096, (uri)))
static void tachyon_record(const char *event) { (void)event; }
static wchar_t *tachyon_widen(const char *value) {
  int length = MultiByteToWideChar(CP_UTF8, 0, value, -1, NULL, 0);
  wchar_t *wide = length > 0 ? (wchar_t *)calloc((size_t)length, sizeof(wchar_t)) : NULL;
  if (wide != NULL) MultiByteToWideChar(CP_UTF8, 0, value, -1, wide, length);
  return wide;
}
static char *tachyon_narrow(const wchar_t *value) {
  int length = WideCharToMultiByte(CP_UTF8, 0, value, -1, NULL, 0, NULL, NULL);
  char *narrow = length > 0 ? (char *)calloc((size_t)length, sizeof(char)) : NULL;
  if (narrow != NULL) WideCharToMultiByte(CP_UTF8, 0, value, -1, narrow, length, NULL, NULL);
  return narrow;
}",
        super::windows::WindowsHostGenerator::PACKAGE_DATA,
        super::native_deep_link::NativeDeepLink::WINDOWS,
        r#"
int WINAPI wWinMain(HINSTANCE instance, HINSTANCE previous, PWSTR arguments, int show) {
  (void)previous; (void)arguments; (void)show;
  (void)tachyon_bundle_path; (void)tachyon_payload_route_matches; (void)tachyon_external_link;
  tachyon_read_launch_link();
  if (tachyon_forward_link()) return 0;
  tachyon_register_link_schemes();
  tachyon_links_listen(instance);
  wchar_t target[TACHYON_PATH_LIMIT];
  if (!tachyon_link_target(L"myapp://settings?tab=1", target) || wcscmp(target, L"https://tachyon.local/settings/?tab=1") != 0) return 2;
  if (tachyon_link_target(L"https://example.org/settings", target)) return 3;
  wchar_t command[1024];
  DWORD size = sizeof(command);
  if (RegGetValueW(HKEY_CURRENT_USER, L"Software\\Classes\\other\\shell\\open\\command", NULL, RRF_RT_REG_SZ, NULL, command, &size) != ERROR_SUCCESS ||
      wcsstr(command, L"links.exe\" \"%1\"") == NULL) return 4;
  wchar_t self[MAX_PATH], line[MAX_PATH + 64];
  GetModuleFileNameW(NULL, self, MAX_PATH);
  _snwprintf_s(line, MAX_PATH + 64, _TRUNCATE, L"\"%s\" \"other://items/7\"", self);
  STARTUPINFOW startup = {0};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process;
  if (!CreateProcessW(NULL, line, NULL, NULL, FALSE, 0, NULL, NULL, &startup, &process)) return 5;
  DWORD deadline = GetTickCount() + 30000;
  MSG message;
  while (g_launch_target[0] == L'\0' && GetTickCount() < deadline) {
    while (PeekMessageW(&message, NULL, 0, 0, PM_REMOVE)) DispatchMessageW(&message);
    Sleep(10);
  }
  DWORD second = 1;
  WaitForSingleObject(process.hProcess, 10000);
  GetExitCodeProcess(process.hProcess, &second);
  CloseHandle(process.hThread);
  CloseHandle(process.hProcess);
  if (second != 0) return 6;
  return wcscmp(g_launch_target, L"https://tachyon.local/items/7/") == 0 ? 0 : 7;
}
"#
    );
    std::fs::write(&source, harness).expect("harness");
    let executable = root.path().join("links.exe");
    super::host::run_tool(
        "x86_64-w64-mingw32-gcc",
        &[
            "-std=gnu17",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-municode",
            "-mwindows",
            source.to_str().expect("source"),
            "-ladvapi32",
            "-lshell32",
            "-o",
            executable.to_str().expect("executable"),
        ],
    )
    .await
    .expect("Windows link code compiles and links");
    super::host::run_tool("wine", &[executable.to_str().expect("executable")])
        .await
        .expect(
            "a second Windows instance hands its link to the first, which registered the schemes",
        );
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, a D-Bus session bus, an X11 display (Xvfb) and a working WebKit sandbox; run with dbus-run-session -- xvfb-run -a on the Linux lane"]
async fn the_linux_app_opens_links_at_their_route_and_forwards_later_links_to_the_running_app() {
    let root = tempfile::tempdir().expect("fixture");
    let source = super::native_menu_action_tests::MenuActionFixture::stage(root.path());
    let executable = super::linux_host_tests::LinuxHostTests::compile_application(
        root.path(),
        &source,
        &NativeDeepLinkTests::linked_application(),
        include_str!("linux_deep_link_fixture.c"),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &["myapp://help"])
        .await
        .expect("actual generated Linux host deep links");
    assert!(output.contains("linux deep links passed"), "{output}");
}

#[cfg(target_os = "macos")]
#[tokio::test]
#[ignore = "Requires Xcode's swiftc, AppKit and WebKit; hands universal-link activities to the generated macOS AppDelegate, not a signed app opened from a hosted domain"]
async fn the_macos_app_delegate_opens_universal_links_to_listed_hosts_at_their_route() {
    let root = tempfile::tempdir().expect("fixture");
    let bundle = root.path().join("HostLinkCheck.app/Contents");
    for (document, title) in [("index.html", "Home"), ("settings/index.html", "Settings")] {
        let path = bundle.join("Resources/WebBundle").join(document);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("web folder");
        std::fs::write(
            &path,
            format!("<!doctype html><html><head><title>{title}</title></head><body>{title}</body></html>"),
        )
        .expect("page");
    }
    std::fs::write(
        bundle.join("Info.plist"),
        "<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>HostLinkCheck</string><key>CFBundleIdentifier</key><string>org.tachyon.host-link-check</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>",
    )
    .expect("Info.plist");
    let host = super::macos::swift_source(
        &NativeDeepLinkTests::linked_application(),
        super::macos::companion_call(None, None),
        &NativeDeepLinkTests::index(),
    );
    let body = host.rsplit_once("\n@main\n").expect("generated main").0;
    let source = root.path().join("HostLinkCheck.swift");
    std::fs::write(
        &source,
        format!(
            "{body}\n{}",
            include_str!("NativeHostLinkCheck.fixture.swift")
        ),
    )
    .expect("host source");
    let executable = bundle.join("MacOS/HostLinkCheck");
    std::fs::create_dir_all(executable.parent().expect("binary folder")).expect("binary folder");
    super::host::run_tool(
        "/usr/bin/xcrun",
        &[
            "swiftc",
            "-parse-as-library",
            "-framework",
            "AppKit",
            "-framework",
            "WebKit",
            source.to_str().expect("source"),
            "-o",
            executable.to_str().expect("binary"),
        ],
    )
    .await
    .expect("generated macOS host compiles");
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("generated macOS host opens universal links");
    assert!(
        output.contains("macOS universal link delegate passed"),
        "{output}"
    );
}

#![allow(clippy::expect_used)]

use super::native_menu_config::NativeMenuConfig;

struct NativeExternalLinkTests;

impl NativeExternalLinkTests {
    const WINDOWS_ORIGIN: &'static str = "https://tachyon.local";
    const LINUX_ORIGIN: &'static str = "tachyon://app";
    const RULE_HARNESS: &'static str = r#"
typedef struct { const char *uri; int opens; } TachyonLinkCase;
static const TachyonLinkCase TACHYON_LINK_CASES[] = {
  {"https://example.com/", 1},
  {"HTTPS://Example.com/a?b=1#c", 1},
  {"http://example.org", 1},
  {"https://example.com/?q=a|b", 1},
  {"https://tachyon.local.example.com/", 1},
  {"mailto:someone@example.com", 1},
  {"MAILTO:someone@example.com?subject=Hi", 1},
  {"", 0},
  {"tachyon://app/", 0},
  {"file:///etc/passwd", 0},
  {"javascript:alert(1)", 0},
  {"data:text/html,x", 0},
  {"ftp://example.com/", 0},
  {"ms-settings:display", 0},
  {"httpsx://example.com/", 0},
  {"http:example.com", 0},
  {"mailto:", 0},
  {"https://", 0},
  {"https:///path", 0},
  {"https://?q", 0},
  {"https://user@example.com/", 0},
  {"https://good.example@evil.example/", 0},
  {"https://example.com/\"quoted", 0},
  {"https://example.com/a b", 0},
  {"https://example.com/\x01", 0},
  {"https://example.com/\x7f", 0},
  {"https://ex\xc3\xa9.com/", 0},
__LOCAL_ORIGIN_CASES__
};

int main(void) {
  (void)tachyon_local_path;
  (void)tachyon_bundle_path;
  (void)tachyon_payload_route_matches;
  int failures = 0;
  for (size_t index = 0; index < sizeof(TACHYON_LINK_CASES) / sizeof(TACHYON_LINK_CASES[0]); index++) {
    const TachyonLinkCase *test = &TACHYON_LINK_CASES[index];
    if (tachyon_external_link(test->uri) != test->opens) { fprintf(stderr, "wrong answer for %s\n", test->uri); failures++; }
  }
  if (tachyon_external_link(NULL)) { fprintf(stderr, "a missing link opened\n"); failures++; }
  static char longest[TACHYON_EXTERNAL_LINK_LIMIT + 2];
  memcpy(longest, "https://example.com/", strlen("https://example.com/"));
  memset(longest + strlen("https://example.com/"), 'a', TACHYON_EXTERNAL_LINK_LIMIT - strlen("https://example.com/"));
  if (!tachyon_external_link(longest)) { fprintf(stderr, "a link at the limit stayed closed\n"); failures++; }
  longest[TACHYON_EXTERNAL_LINK_LIMIT] = 'a';
  if (tachyon_external_link(longest)) { fprintf(stderr, "a link over the limit opened\n"); failures++; }
  return failures == 0 ? 0 : 1;
}
"#;
    const WINDOWS_LOCAL_CASES: &'static str = r#"  {"https://tachyon.local/", 0},
  {"https://tachyon.local", 0},
  {"https://TACHYON.LOCAL/help/", 0},
  {"https://tachyon.local:443/help/", 0},
  {"https://tachyon.local?x", 0},
  {"https://tachyon.local#x", 0},"#;
    const LINUX_LOCAL_CASES: &'static str = r#"  {"tachyon://app/help/", 0},
  {"TACHYON://APP/", 0},
  {"https://tachyon.local/", 1},"#;
    #[cfg(unix)]
    const MINGW: &'static str = "x86_64-w64-mingw32-gcc";
    #[cfg(unix)]
    const WINE: &'static str = "wine";
    #[cfg(unix)]
    const WINDOWS_HARNESS_HEAD: &'static str = r"#define UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shellapi.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
";
    #[cfg(unix)]
    const WINDOWS_HARNESS_STUBS: &'static str = r#"
static int g_failed_launches = 0;
static void tachyon_record(const char *event) { if (strcmp(event, "link.external.failed") == 0) g_failed_launches++; }
static char *tachyon_narrow(const wchar_t *value) {
  int length = WideCharToMultiByte(CP_UTF8, 0, value, -1, NULL, 0, NULL, NULL);
  char *narrow = length > 0 ? (char *)calloc((size_t)length, sizeof(char)) : NULL;
  if (narrow != NULL) WideCharToMultiByte(CP_UTF8, 0, value, -1, narrow, length, NULL, NULL);
  return narrow;
}
"#;
    #[cfg(unix)]
    const WINDOWS_HARNESS_MAIN: &'static str = r#"
static wchar_t g_opened[1024];
static void tachyon_fake_open(const wchar_t *uri) {
  if (g_opened[0] != L'\0') wcscat(g_opened, L" ");
  wcscat(g_opened, uri);
}

int main(void) {
  (void)tachyon_local_path;
  (void)tachyon_bundle_path;
  (void)tachyon_payload_route_matches;
  g_external_launch = tachyon_fake_open;
  const wchar_t *targets[] = {L"https://example.com/a?b=1", L"https://tachyon.local/help/", L"mailto:someone@example.com",
                              L"file:///C:/Windows/win.ini", L"ms-settings:display", L"https://example.com/caf\u00e9", L"http://example.org/"};
  for (size_t index = 0; index < sizeof(targets) / sizeof(targets[0]); index++) tachyon_open_external(targets[index]);
  int matches = wcscmp(g_opened, L"https://example.com/a?b=1 mailto:someone@example.com http://example.org/") == 0;
  if (!matches) fwprintf(stderr, L"opened: %ls\n", g_opened);
  return matches && g_failed_launches == 0 ? 0 : 1;
}
"#;
    const LINUX_SCRIPT: &'static str = r"const pause = () => new Promise((resolve) => setTimeout(resolve, 200));
for (const target of ['https://example.com/a?b=1', 'file:///etc/passwd', 'http://example.org/', 'ftp://example.com/', 'mailto:someone@example.com', 'tachyon://app/missing/', 'https://user@example.com/']) {
  location.href = target;
  await pause();
}
window.open('https://example.com/new', '_blank');
await pause();
window.open('ms-settings:display', '_blank');
await pause();
const link = document.createElement('a');
link.href = 'https://example.com/anchor';
link.target = '_blank';
document.body.append(link);
link.click();
await pause();
return location.href;";
    const LINUX_EXPECTED: &'static str = "https://example.com/a?b=1 http://example.org/ mailto:someone@example.com https://example.com/new https://example.com/anchor";

    fn index() -> super::routes::NativeRouteIndex {
        super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        }
    }

    fn rule_harness(origin: &str, local_cases: &str) -> String {
        let local_bundle = super::routes::c_local_bundle(&Self::index(), &[], origin);
        format!(
            "#include <stdio.h>\n#include <string.h>\n{local_bundle}\n{}\n{}",
            super::native_external_link::NativeExternalLink::C_HELPER,
            Self::RULE_HARNESS.replace("__LOCAL_ORIGIN_CASES__", local_cases)
        )
    }

    fn windows_host() -> String {
        super::windows::c_source(
            &super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default()),
            &super::linux_host_tests::LinuxHostTests::index(),
            &[],
        )
    }

    fn linux_host() -> String {
        super::linux::c_source(
            &super::linux_host_tests::LinuxHostTests::application(NativeMenuConfig::default()),
            &super::linux_host_tests::LinuxHostTests::index(),
        )
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).expect("generated function");
        body.split("\n}\n").next().expect("function body")
    }

    #[cfg(unix)]
    fn windows_harness() -> String {
        let host = Self::windows_host();
        let opener = host
            .split("typedef void (*TachyonExternalLaunch)(const wchar_t *);")
            .nth(1)
            .and_then(|rest| {
                rest.split("typedef struct { ICoreWebView2NavigationStartingEventHandlerVtbl")
                    .next()
            })
            .expect("Windows external link opener");
        format!(
            "{}\n{}\n{}\n{}\ntypedef void (*TachyonExternalLaunch)(const wchar_t *);\n{opener}\n{}",
            Self::WINDOWS_HARNESS_HEAD,
            super::routes::c_local_bundle(&Self::index(), &[], Self::WINDOWS_ORIGIN),
            super::native_external_link::NativeExternalLink::C_HELPER,
            Self::WINDOWS_HARNESS_STUBS,
            Self::WINDOWS_HARNESS_MAIN
        )
    }

    #[cfg(unix)]
    fn run_rules(origin: &str, local_cases: &str) {
        let directory = tempfile::tempdir().expect("external link fixture");
        let source = directory.path().join("links.c");
        let binary = directory.path().join("links");
        std::fs::write(&source, Self::rule_harness(origin, local_cases)).expect("fixture");
        let compile = std::process::Command::new("cc")
            .args(["-std=gnu17", "-Wall", "-Wextra", "-Werror"])
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .output()
            .expect("compile the external link rule");
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = std::process::Command::new(&binary)
            .output()
            .expect("run the external link rule");
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }

    fn linux_fixture() -> String {
        include_str!("linux_external_link_fixture.c")
            .replace(
                "__LINK_SCRIPT__",
                &super::host::c_string_escape(Self::LINUX_SCRIPT),
            )
            .replace("__LINK_EXPECTED__", Self::LINUX_EXPECTED)
    }
}

#[cfg(unix)]
#[test]
fn only_http_https_and_mailto_links_off_the_windows_origin_open_outside_the_app() {
    NativeExternalLinkTests::run_rules(
        NativeExternalLinkTests::WINDOWS_ORIGIN,
        NativeExternalLinkTests::WINDOWS_LOCAL_CASES,
    );
}

#[cfg(unix)]
#[test]
fn only_http_https_and_mailto_links_off_the_linux_origin_open_outside_the_app() {
    NativeExternalLinkTests::run_rules(
        NativeExternalLinkTests::LINUX_ORIGIN,
        NativeExternalLinkTests::LINUX_LOCAL_CASES,
    );
}

#[test]
fn the_windows_host_opens_external_links_in_the_default_browser() {
    let source = NativeExternalLinkTests::windows_host();
    assert!(source.contains("ShellExecuteW(NULL, L\"open\", uri, NULL, NULL, SW_SHOWNORMAL)"));
    assert!(source.contains("static int tachyon_external_link(const char *uri) {"));
    let opener = NativeExternalLinkTests::section(
        &source,
        "static void tachyon_open_external(const wchar_t *uri) {",
    );
    assert!(opener.contains("if (tachyon_external_link(text)) g_external_launch(uri);"));
    let navigation = NativeExternalLinkTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE navigation_invoke(",
    );
    assert!(navigation.contains(
        "ICoreWebView2NavigationStartingEventArgs_put_Cancel(arguments, TRUE);\n    if (((TachyonNavigationHandler *)self)->top_level) tachyon_open_external(uri);"
    ));
    let new_window = NativeExternalLinkTests::section(
        &source,
        "static HRESULT STDMETHODCALLTYPE new_window_invoke(",
    );
    assert!(
        new_window.contains("ICoreWebView2NewWindowRequestedEventArgs_get_Uri(arguments, &uri)")
    );
    assert!(
        new_window.find("tachyon_open_external(uri)").expect("open")
            < new_window
                .find("put_Handled(arguments, TRUE)")
                .expect("handled")
    );
    assert!(!source.contains("__EXTERNAL_LINK_HELPER__"));
}

#[test]
fn the_linux_host_opens_external_links_through_the_gtk_uri_launcher() {
    let source = NativeExternalLinkTests::linux_host();
    assert!(source.contains("static int tachyon_external_link(const char *uri) {"));
    assert!(source.contains(
        "gtk_uri_launcher_launch(launcher, g_native_window, NULL, tachyon_external_launched, NULL);"
    ));
    let policy =
        NativeExternalLinkTests::section(&source, "static gboolean tachyon_decide_policy(");
    assert_eq!(
        policy
            .matches("tachyon_open_external(action); webkit_policy_decision_ignore(decision); return TRUE;")
            .count(),
        2
    );
    assert!(
        source
            .contains("g_signal_connect(view, \"create\", G_CALLBACK(tachyon_create_view), NULL);")
    );
    let create =
        NativeExternalLinkTests::section(&source, "static GtkWidget *tachyon_create_view(");
    assert!(create.contains("return NULL;"));
    assert!(!source.contains("__EXTERNAL_LINK_HELPER__"));
}

#[test]
fn the_linux_link_fixture_drives_a_real_page_script() {
    let fixture = NativeExternalLinkTests::linux_fixture();
    assert!(fixture.contains("g_external_launch = tachyon_link_record;"));
    assert!(!fixture.contains("__LINK_"));
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and Wine; builds the Windows link opener against the Win32 headers and runs it"]
async fn the_windows_link_opener_compiles_against_win32_and_opens_only_external_links() {
    let root = tempfile::tempdir().expect("fixture");
    let source = root.path().join("links.c");
    std::fs::write(&source, NativeExternalLinkTests::windows_harness()).expect("harness");
    let executable = root.path().join("links.exe");
    super::host::run_tool(
        NativeExternalLinkTests::MINGW,
        &[
            "-std=gnu17",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
            source.to_str().expect("source"),
            "-lshell32",
            "-o",
            executable.to_str().expect("executable"),
        ],
    )
    .await
    .expect("Windows link opener compiles and links");
    super::host::run_tool(
        NativeExternalLinkTests::WINE,
        &[executable.to_str().expect("executable")],
    )
    .await
    .expect("Windows link opener opens only external links");
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, an X11 display (Xvfb) and a working WebKit sandbox; run explicitly on the Linux lane"]
async fn the_linux_host_hands_external_links_to_the_launcher_and_keeps_the_page() {
    let root = tempfile::tempdir().expect("fixture");
    let source = super::native_menu_action_tests::MenuActionFixture::stage(root.path());
    let executable = super::linux_host_tests::LinuxHostTests::compile_with(
        root.path(),
        &source,
        NativeMenuConfig::default(),
        &NativeExternalLinkTests::linux_fixture(),
    )
    .await;
    let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
        .await
        .expect("actual generated Linux host external links");
    assert!(output.contains("linux external links passed"), "{output}");
}

#[test]
fn apple_and_android_hosts_open_the_same_external_links() {
    for source in [super::macos::host_source(), super::ios::host_source()] {
        assert!(source.contains(
            "let web = [\"https\", \"http\"].contains(scheme) && url.host?.isEmpty == false && url.user == nil && url.password == nil"
        ));
        assert!(source.contains(
            "let mail = scheme == \"mailto\" && url.absoluteString.count > \"mailto:\".count"
        ));
        assert!(source.contains("if action.targetFrame?.isMainFrame != false && (web || mail) {"));
    }
    let android = super::android::host_source();
    assert!(
        android.contains("&& host != null && !host.isEmpty() && target.getUserInfo() == null;")
    );
    assert!(android.contains(
        "boolean mail = \"mailto\".equalsIgnoreCase(scheme) && address != null && !address.isEmpty();"
    ));
}

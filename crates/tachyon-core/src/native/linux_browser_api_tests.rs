#![allow(clippy::expect_used)]

use super::browser_api_permissions::BrowserApiPermissions;
use super::linux_browser_apis::LinuxBrowserApis;

struct LinuxBrowserApiTests;

impl LinuxBrowserApiTests {
    const ALL: [&'static str; 4] = ["clipboard", "geolocation", "notifications", "share"];
    const SCRIPT: &'static str = r"const post = (capability, values) => window.webkit.messageHandlers.tachyon.postMessage({capability, payload: JSON.stringify(values)});
const answer = async (capability, values) => {
  const reply = JSON.parse(await post(capability, values));
  return 'error' in reply ? 'error' : JSON.stringify(reply.value);
};
const clicked = (work) => new Promise((done) => addEventListener('click', () => done(work()), {once: true}));
const results = [];
results.push(await clicked(() => navigator.clipboard.readText().then((text) => 'read:' + text, (error) => 'read-refused:' + error.name)));
results.push(await clicked(() => navigator.clipboard.writeText('copied by the page').then(() => 'written', (error) => 'write-refused:' + error.name)));
results.push(await new Promise((done) => navigator.geolocation.getCurrentPosition(
  (position) => done('position:' + position.coords.latitude + ',' + position.coords.longitude),
  (error) => done('position-refused:' + error.code),
  {timeout: 5000, enableHighAccuracy: true})));
results.push(await answer('browserApi.notifications.requestPermission', {}));
results.push(await answer('browserApi.notifications.show', {title: 'Order', body: 'Shipped'}));
results.push(await answer('browserApi.notifications.show', {body: 'No title'}));
results.push(await answer('browserApi.share.open', {title: 'Catalog', text: 'Look at this', url: 'https://example.com/item'}));
results.push(await answer('browserApi.share.open', {}));
results.push(await answer('browserApi.clipboard.readText', {}));
return results.join(',');";
    const GRANTED: [&'static str; 3] = [
        "read:from another app,written,position:51.5,-0.125,\"granted\",null,error,\"clipboard\",error,error",
        "Order|Shipped",
        "https://example.com/item",
    ];
    const REFUSED: [&'static str; 3] = [
        "read-refused:NotAllowedError,written,position-refused:1,\"denied\",error,error,error,error,error",
        "",
        "copied by the page",
    ];

    fn permissions(granted: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    fn host(granted: &[&str]) -> String {
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::native_menu_config::NativeMenuConfig::default(),
        );
        application.permissions = Self::permissions(granted);
        super::linux::c_source(
            &application,
            &super::linux_host_tests::LinuxHostTests::index(),
        )
    }

    fn fixture(expected: [&str; 3], test_position: bool) -> String {
        let [summary, notified, clipboard] = expected;
        concat!(
            include_str!("linux_wayland_input.c"),
            include_str!("linux_browser_api_fixture.c")
        )
        .replace(
            "__BROWSER_TEST_POSITION__",
            ["FALSE", "TRUE"][usize::from(test_position)],
        )
        .replace(
            "__BROWSER_SCRIPT__",
            &super::host::c_string_escape(Self::SCRIPT),
        )
        .replace(
            "__BROWSER_EXPECTED__",
            &super::host::c_string_escape(summary),
        )
        .replace(
            "__BROWSER_NOTIFIED__",
            &super::host::c_string_escape(notified),
        )
        .replace(
            "__BROWSER_CLIPBOARD__",
            &super::host::c_string_escape(clipboard),
        )
    }

    #[cfg(target_os = "linux")]
    async fn run(granted: &[&str], expected: [&str; 3], test_position: bool) {
        let root = tempfile::tempdir().expect("fixture");
        let source = super::native_menu_action_tests::MenuActionFixture::stage(root.path());
        let mut application = super::linux_host_tests::LinuxHostTests::application(
            super::linux_host_tests::LinuxHostTests::menus(),
        );
        application.permissions = Self::permissions(granted);
        let executable = super::linux_host_tests::LinuxHostTests::compile_application(
            root.path(),
            &source,
            &application,
            &Self::fixture(expected, test_position),
        )
        .await;
        let output = super::host::run_tool(executable.to_str().expect("binary"), &[])
            .await
            .expect("actual generated Linux host browser APIs");
        assert!(output.contains("linux browser APIs passed"), "{output}");
        println!("{output}");
    }
}

#[test]
fn the_linux_host_checks_only_the_browser_apis_the_manifest_grants() {
    let nothing = LinuxBrowserApis::source(&LinuxBrowserApiTests::permissions(&[]));
    assert!(!nothing.contains("if (strcmp(api, "));
    assert!(!nothing.contains("__GRANTED_BROWSER_APIS__"));
    for api in LinuxBrowserApiTests::ALL {
        let source = LinuxBrowserApis::source(&LinuxBrowserApiTests::permissions(&[api]));
        assert!(source.contains(&format!("if (strcmp(api, \"{api}\") == 0) return TRUE;")));
        assert_eq!(source.matches("if (strcmp(api, ").count(), 1);
    }
    let unrelated =
        LinuxBrowserApis::source(&LinuxBrowserApiTests::permissions(&["preferenceStrings"]));
    assert!(!unrelated.contains("if (strcmp(api, "));
}

#[test]
fn the_linux_host_decides_web_view_permissions_and_answers_bridged_calls() {
    let source = LinuxBrowserApiTests::host(&LinuxBrowserApiTests::ALL);
    assert!(!source.contains("__NATIVE_BROWSER_APIS__"));
    assert!(source.contains(
        "g_signal_connect(view, \"permission-request\", G_CALLBACK(tachyon_permission_request), NULL);"
    ));
    assert!(source.contains("if (api != NULL && tachyon_trusted_view(view) && tachyon_browser_api_granted(api)) webkit_permission_request_allow(request);"));
    assert!(source.contains("else webkit_permission_request_deny(request);"));
    for (request, api) in [
        ("WEBKIT_IS_CLIPBOARD_PERMISSION_REQUEST", "clipboard"),
        ("WEBKIT_IS_GEOLOCATION_PERMISSION_REQUEST", "geolocation"),
        ("WEBKIT_IS_NOTIFICATION_PERMISSION_REQUEST", "notifications"),
    ] {
        assert!(source.contains(&format!("if ({request}(request)) return \"{api}\";")));
    }
    for (operation, answer) in [
        ("browserApi.share.open", "tachyon_share_open(payload)"),
        (
            "browserApi.notifications.requestPermission",
            "tachyon_notification_permission()",
        ),
        (
            "browserApi.notifications.show",
            "tachyon_notification_show(payload)",
        ),
    ] {
        assert!(source.contains(&format!(
            "if (g_strcmp0(capability, \"{operation}\") == 0) {{\n    answer = {answer};"
        )));
    }
    assert!(source.contains(
        "gdk_clipboard_set_text(gdk_display_get_clipboard(gdk_display_get_default()), shared);"
    ));
    assert!(source.contains(
        "g_application_send_notification(g_application_get_default(), id, notification);"
    ));
}

#[test]
fn the_browser_api_fixture_fills_every_placeholder() {
    let fixture = LinuxBrowserApiTests::fixture(LinuxBrowserApiTests::GRANTED, true);
    assert!(!fixture.contains("__BROWSER_"));
    assert!(fixture.contains("browserApi.share.open"));
    assert!(fixture.contains("XTestFakeButtonEvent"));
    assert!(fixture.contains("\"NotifyPointerButton\""));
    assert!(fixture.contains("if (TRUE) g_signal_connect(geolocation, \"start\""));
    let refused = LinuxBrowserApiTests::fixture(LinuxBrowserApiTests::REFUSED, true);
    assert!(refused.contains("read-refused:NotAllowedError"));
    let geoclue = LinuxBrowserApiTests::fixture(LinuxBrowserApiTests::GRANTED, false);
    assert!(geoclue.contains("if (FALSE) g_signal_connect(geolocation, \"start\""));
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, a session bus, and an X11 display (Xvfb) with libXtst or a headless mutter Wayland session (scripts/linux/compositor-test.sh); run explicitly on the Linux lane"]
async fn linux_browser_apis_follow_the_manifest_grants_in_the_real_web_view() {
    LinuxBrowserApiTests::run(
        &LinuxBrowserApiTests::ALL,
        LinuxBrowserApiTests::GRANTED,
        true,
    )
    .await;
    LinuxBrowserApiTests::run(&[], LinuxBrowserApiTests::REFUSED, true).await;
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires what the real web view test needs plus GeoClue on the system bus and a notification server on the session bus; run through scripts/linux/desktop-services-test.sh"]
async fn linux_browser_apis_reach_geoclue_and_the_notification_server() {
    LinuxBrowserApiTests::run(
        &LinuxBrowserApiTests::ALL,
        LinuxBrowserApiTests::GRANTED,
        false,
    )
    .await;
}

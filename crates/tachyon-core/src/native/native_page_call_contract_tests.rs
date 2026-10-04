use super::linux_host_tests::LinuxHostTests;
use super::native_menu_config::NativeMenuConfig;

struct NativePageCallContractTests;

impl NativePageCallContractTests {
    const OPERATIONS: [&'static str; 5] = ["init", "get", "set", "call", "dispose"];

    fn linux() -> String {
        super::linux::c_source(
            &LinuxHostTests::application(NativeMenuConfig::default()),
            &LinuxHostTests::index(),
        )
    }

    fn windows() -> String {
        super::windows::c_source(
            &LinuxHostTests::application(NativeMenuConfig::default()),
            &LinuxHostTests::index(),
            &[],
        )
    }

    fn macos() -> String {
        let call = super::macos::companion_call(None, None);
        super::macos::swift_source(
            &LinuxHostTests::application(NativeMenuConfig::default()),
            call,
            &LinuxHostTests::index(),
        ) + super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST
    }

    fn section<'s>(source: &'s str, start: &str) -> &'s str {
        let body = source.split(start).nth(1).unwrap_or_default();
        body.split("\n}\n").next().unwrap_or_default()
    }

    fn accepts_the_companion_operations(host: &str, source: &str) {
        for operation in Self::OPERATIONS {
            assert!(
                source.contains(&format!("\"op\", \"{operation}\""))
                    || source.contains(&format!("\"{operation}\",")),
                "{host} does not route op {operation}"
            );
        }
    }
}

#[test]
fn every_desktop_host_routes_the_same_companion_operations_with_a_64_kib_request_limit() {
    let linux = NativePageCallContractTests::linux();
    let windows = NativePageCallContractTests::windows();
    let macos = NativePageCallContractTests::macos();
    NativePageCallContractTests::accepts_the_companion_operations("linux", &linux);
    NativePageCallContractTests::accepts_the_companion_operations("windows", &windows);
    assert!(
        macos.contains("[\"init\", \"get\", \"set\", \"call\", \"dispose\"].contains(operation)")
    );
    for host in [&linux, &windows] {
        assert!(host.contains("#define TACHYON_MESSAGE_LIMIT 65536"));
        assert!(host.contains("tachyon_payload_route_matches(payload, route->route)"));
    }
    assert!(macos.contains("guard raw.utf8.count <= 65536 else { return nil }"));
    let linux_call =
        NativePageCallContractTests::section(&linux, "static gboolean tachyon_on_host_call(");
    assert!(linux_call.contains("strlen(payload) > TACHYON_MESSAGE_LIMIT"));
    assert!(windows.contains("payload.length>65536"));
}

#[test]
fn linux_and_windows_answer_an_oversized_reply_with_a_paging_hint_instead_of_an_opaque_error() {
    let linux = NativePageCallContractTests::linux();
    let windows = NativePageCallContractTests::windows();
    for host in [&linux, &windows] {
        assert!(!host.contains("Invalid native response."));
        assert!(host.contains("send it in pages of at most 24 KiB"));
        assert!(host.contains("TACHYON_EMPTY_REPLY"));
    }
    let invoke =
        NativePageCallContractTests::section(&linux, "static gchar *tachyon_companion_invoke(");
    assert!(invoke.contains("<= TACHYON_MESSAGE_LIMIT ? answer : TACHYON_REPLY_TOO_LARGE"));
    let handle = NativePageCallContractTests::section(&windows, "static char *tachyon_handle(");
    assert!(handle.contains("<= TACHYON_MESSAGE_LIMIT ? answer : TACHYON_REPLY_TOO_LARGE"));
}

#[test]
fn windows_answers_a_request_that_grows_past_the_limit_when_narrowed_instead_of_timing_out() {
    let windows = NativePageCallContractTests::windows();
    let message = NativePageCallContractTests::section(&windows, "static void tachyon_on_message(");
    assert!(message.contains(
        "strlen(payload) > TACHYON_MESSAGE_LIMIT ? _strdup(TACHYON_REQUEST_TOO_LARGE) : tachyon_handle(message, payload)"
    ));
    assert!(
        !message
            .contains("strlen(payload + 1) > TACHYON_MESSAGE_LIMIT) { free(message); return; }")
    );
}

#[test]
fn page_calls_are_answered_on_the_user_interface_thread_of_every_desktop_host() {
    let linux = NativePageCallContractTests::linux();
    assert!(
        linux.contains("g_signal_connect(manager, \"script-message-with-reply-received::tachyon\"")
    );
    let windows = NativePageCallContractTests::windows();
    assert!(windows.contains("ICoreWebView2_add_WebMessageReceived"));
    assert!(windows.contains("const result=chain.then(call,call)"));
    let macos = NativePageCallContractTests::macos();
    assert!(macos.contains("replyHandler(handle(capability, canonicalPayload), nil)"));
}

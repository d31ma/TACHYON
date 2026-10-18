#![allow(clippy::expect_used)]

use super::config::NativeApplication;
use super::routes::NativeRouteIndex;
use serde_json::{Value, json};

struct NativePolicyFixture;

impl NativePolicyFixture {
    const LISTED_SOURCES: &str = "connect-src 'self' https://api.example.com wss://api.example.com http://api.example.com ws://api.example.com https://*.cdn.example.com wss://*.cdn.example.com http://*.cdn.example.com ws://*.cdn.example.com;";
    const CLOSED_SOURCES: &str = "connect-src 'self';";

    fn application(outgoing: &Value) -> Result<NativeApplication, String> {
        Self::tac_application(&json!({ "outgoing": outgoing }))
    }

    fn tac_application(tac: &Value) -> Result<NativeApplication, String> {
        let root = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(root.path().join("client")).expect("client");
        let manifest = json!({ "name": "Catalog", "tac": tac });
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("manifest");
        NativeApplication::discover(root.path()).map_err(|failure| failure.to_string())
    }

    fn declared(outgoing: &Value) -> NativeApplication {
        Self::application(outgoing).expect("application")
    }

    fn index() -> NativeRouteIndex {
        NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: Vec::new(),
        }
    }

    fn host_sources(application: &NativeApplication) -> [(&'static str, String); 5] {
        let index = Self::index();
        let apple_call = super::macos::companion_call(None, None);
        [
            (
                "macos",
                super::macos::swift_source(application, apple_call, &index),
            ),
            (
                "ios",
                super::ios::swift_source(application, apple_call, &index),
            ),
            (
                "android",
                super::android::java_source(application, "dev.tachyon.catalog", false, &index),
            ),
            (
                "windows",
                super::windows::c_source(application, &index, &[]),
            ),
            ("linux", super::linux::c_source(application, &index)),
        ]
    }
}

#[test]
fn every_native_host_limits_page_requests_to_the_declared_outgoing_hosts() {
    let application = NativePolicyFixture::declared(&json!({
        "hosts": ["api.example.com", "*.CDN.example.com"]
    }));
    for (platform, source) in NativePolicyFixture::host_sources(&application) {
        assert!(
            source.contains(NativePolicyFixture::LISTED_SOURCES),
            "{platform} host does not limit connect-src to tac.outgoing.hosts"
        );
        assert!(
            !source.contains("connect-src 'self' https:;"),
            "{platform} allows every host"
        );
        assert!(
            !source.contains("__CONTENT_SECURITY_POLICY__"),
            "{platform} host kept the policy placeholder"
        );
    }
}

#[test]
fn every_native_host_lets_the_page_reach_the_declared_yon_origin() {
    let application = NativePolicyFixture::tac_application(&json!({
        "outgoing": { "hosts": ["api.example.com"] },
        "yon": { "origin": "https://yon.example.com:8443" }
    }))
    .expect("application");
    for (platform, source) in NativePolicyFixture::host_sources(&application) {
        assert!(
            source.contains(
                " ws://api.example.com https://yon.example.com:8443 wss://yon.example.com:8443;"
            ),
            "{platform} host does not let the page reach tac.yon.origin"
        );
    }
    let android = super::android::java_source(
        &application,
        "dev.tachyon.catalog",
        false,
        &NativePolicyFixture::index(),
    );
    assert!(android.contains(r#"OUTGOING_HOSTS = {"api.example.com", "yon.example.com"};"#));
}

#[test]
fn a_native_app_without_outgoing_hosts_connects_only_to_its_own_bundle() {
    let application = NativePolicyFixture::declared(&json!({}));
    for (platform, source) in NativePolicyFixture::host_sources(&application) {
        assert!(
            source.contains(NativePolicyFixture::CLOSED_SOURCES),
            "{platform} host allows requests that the manifest does not list"
        );
    }
}

#[test]
fn the_windows_host_has_room_for_the_whole_policy_header() {
    let hosts: Vec<String> = (0..40)
        .map(|number| format!("service-{number}.example.com"))
        .collect();
    let application = NativePolicyFixture::declared(&json!({ "hosts": hosts }));
    let policy = application.content_security_policy();
    let index = NativePolicyFixture::index();
    let source = super::windows::c_source(&application, &index, &[]);
    let capacity: usize = source
        .split_once("wchar_t headers[")
        .and_then(|(_, rest)| rest.split_once(']'))
        .and_then(|(number, _)| number.parse().ok())
        .expect("header capacity");
    assert!(capacity > policy.len() + 100, "{capacity} {}", policy.len());
    assert!(source.contains(&policy));
}

#[test]
fn an_outgoing_host_that_is_not_a_host_name_stops_the_native_build() {
    for hosts in [
        json!(["https://api.example.com"]),
        json!(["api.example.com:8443"]),
        json!(["api.example.com; script-src *"]),
        json!([7]),
        json!("api.example.com"),
    ] {
        let Err(text) = NativePolicyFixture::application(&json!({ "hosts": hosts })) else {
            panic!("{hosts} was accepted");
        };
        assert!(text.contains("TY1601"), "{text}");
        assert!(text.contains("tac.outgoing.hosts"), "{text}");
    }
}

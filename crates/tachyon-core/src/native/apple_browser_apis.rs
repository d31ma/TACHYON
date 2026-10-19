use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct AppleBrowserApis;

impl AppleBrowserApis {
    pub(super) const SWIFT: &'static str = concat!(
        include_str!("browser_apis/AppleBrowserApis.swift"),
        include_str!("browser_apis/AppleShareSheet.swift"),
        include_str!("browser_apis/AppleCurrentPosition.swift")
    );

    pub(super) fn swift_grants(permissions: &BrowserApiPermissions) -> String {
        serde_json::to_string(permissions.granted()).unwrap_or_else(|_| String::from("[]"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AppleBrowserApis;
    use crate::native::browser_api_permissions::BrowserApiPermissions;
    use crate::native::config::NativeApplication;
    use crate::native::routes::NativeRouteIndex;

    fn hosts(manifest: &str) -> [String; 2] {
        let root = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(root.path().join("client")).expect("client");
        std::fs::write(root.path().join("client/manifest.json"), manifest).expect("manifest");
        let application = NativeApplication::discover(root.path()).expect("application");
        let index = NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: Vec::new(),
        };
        let call = crate::native::macos::companion_call(None, None);
        [
            crate::native::macos::swift_source(&application, call, &index),
            crate::native::ios::swift_source(&application, call, &index),
        ]
    }

    #[test]
    fn both_apple_hosts_answer_browser_api_calls_for_the_granted_apis() {
        let granted = hosts(r#"{"tac":{"browserApis":["clipboard","share"]}}"#);
        let nothing = hosts("{}");
        for (source, empty) in granted.iter().zip(&nothing) {
            assert!(source.contains(r#"AppleBrowserApis.shared.granted = ["clipboard","share"]"#));
            assert!(empty.contains("AppleBrowserApis.shared.granted = []"));
            assert!(source.contains("final class AppleBrowserApis"));
            assert!(source.contains("final class AppleShareSheet"));
            assert!(source.contains("final class AppleCurrentPosition"));
            assert!(source.contains(
                "if capability.hasPrefix(\"browserApi.\") {\n            AppleBrowserApis.shared.call(capability, request, webView) { replyHandler($0, nil) }"
            ));
            assert!(source.contains("AppleBrowserApis.shared.prepare(view) {"));
            assert!(source.contains(
                "globalThis.__tachyonBrowserApiBridge = Object.freeze(['clipboard', 'share', 'notifications', 'geolocation'])"
            ));
            assert!(!source.contains("__NATIVE_BROWSER_API"));
        }
    }

    #[test]
    fn the_swift_grants_are_a_string_array() {
        let permissions = BrowserApiPermissions::declared(
            &[String::from("clipboard"), String::from("share")],
            &std::collections::BTreeMap::new(),
        );
        assert_eq!(
            AppleBrowserApis::swift_grants(&permissions),
            "[\"clipboard\",\"share\"]"
        );
        assert_eq!(
            AppleBrowserApis::swift_grants(&BrowserApiPermissions::default()),
            "[]"
        );
    }

    #[test]
    fn the_swift_answers_every_bridged_api_by_its_own_name() {
        for api in ["clipboard", "share", "notifications", "geolocation"] {
            assert!(
                AppleBrowserApis::SWIFT.contains(&format!("\"{api}\"")),
                "{api}"
            );
        }
        assert!(AppleBrowserApis::SWIFT.contains("#if os(macOS)"));
        for source in [
            AppleBrowserApis::SWIFT,
            include_str!("../template/tac-web.js"),
        ] {
            assert!(source.contains("globalThis.__tachyonNotificationPermission"));
        }
    }
}

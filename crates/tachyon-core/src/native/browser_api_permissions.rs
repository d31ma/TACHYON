use super::config::config_failure;
use super::host::xml_escape;
use crate::Failure;
use std::collections::BTreeMap;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct BrowserApiPermissions {
    granted: Vec<String>,
    usage: BTreeMap<String, String>,
}

impl BrowserApiPermissions {
    const ASKING_APIS: [&'static str; 2] = ["geolocation", "notifications"];
    const MOST_CHARACTERS: usize = 200;
    const REQUIRED: [(&'static str, NativeTarget); 4] = [
        ("geolocation", NativeTarget::Ios),
        ("geolocation", NativeTarget::Macos),
        ("geolocation", NativeTarget::Android),
        ("notifications", NativeTarget::Android),
    ];
    pub(super) const IOS_LOCATION_KEYS: [&'static str; 1] = ["NSLocationWhenInUseUsageDescription"];
    pub(super) const MACOS_LOCATION_KEYS: [&'static str; 2] = [
        "NSLocationUsageDescription",
        "NSLocationWhenInUseUsageDescription",
    ];

    pub(super) fn declared(granted: &[String], usage: &BTreeMap<String, String>) -> Self {
        Self {
            granted: granted.to_vec(),
            usage: usage.clone(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), Failure> {
        self.usage
            .iter()
            .try_for_each(|(api, text)| self.check_usage(api, text))
    }

    pub(super) fn validate_for(&self, target: NativeTarget) -> Result<(), Failure> {
        Self::REQUIRED
            .iter()
            .filter(|(_, needed_on)| *needed_on == target)
            .try_for_each(|(api, _)| self.require_reason(api, target))
    }

    pub(super) fn granted(&self) -> &[String] {
        &self.granted
    }

    pub(super) fn usage(&self, api: &str) -> Option<&str> {
        self.usage.get(api).map(String::as_str)
    }

    pub(super) fn location_plist_entries(&self, keys: &[&str]) -> String {
        self.usage("geolocation").map_or_else(String::new, |text| {
            Self::plist_entries(keys, &xml_escape(text))
        })
    }

    pub(super) fn android_strings(&self) -> String {
        Self::ASKING_APIS
            .iter()
            .filter_map(|api| self.usage(api).map(|text| Self::android_string(api, text)))
            .collect()
    }

    fn plist_entries(keys: &[&str], text: &str) -> String {
        keys.iter()
            .map(|key| ["\n<key>", key, "</key><string>", text, "</string>"].concat())
            .collect()
    }

    fn android_string(api: &str, text: &str) -> String {
        let text = Self::android_text(text);
        [
            "  <string name=\"tachyon_permission_",
            api,
            "\">",
            &text,
            "</string>\n",
        ]
        .concat()
    }

    fn android_text(text: &str) -> String {
        xml_escape(
            &text
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('"', "\\\"")
                .replace('@', "\\@")
                .replace('?', "\\?"),
        )
    }

    pub(super) fn is_granted(&self, api: &str) -> bool {
        self.granted.iter().any(|grant| grant == api)
    }

    fn require_reason(&self, api: &str, target: NativeTarget) -> Result<(), Failure> {
        let unexplained = [self.is_granted(api), !self.usage.contains_key(api)] == [true, true];
        Self::require(!unexplained, || Self::missing(api, target))
    }

    fn check_usage(&self, api: &str, text: &str) -> Result<(), Failure> {
        Self::require(Self::ASKING_APIS.contains(&api), || Self::not_asking(api))?;
        Self::require(self.is_granted(api), || Self::not_granted(api))?;
        Self::require(Self::is_usage_text(text), || Self::not_plain(api))
    }

    fn require(holds: bool, failure: impl FnOnce() -> Failure) -> Result<(), Failure> {
        holds.then_some(()).ok_or_else(failure)
    }

    fn is_usage_text(text: &str) -> bool {
        [
            text.trim().is_empty(),
            text.chars().count() > Self::MOST_CHARACTERS,
            text.chars().any(char::is_control),
        ] == [false, false, false]
    }

    fn not_asking(api: &str) -> Failure {
        config_failure(&format!(
            "'{api}' in tac.application.permissions is not a browser API that asks the user. Give a reason for any of: {}.",
            Self::ASKING_APIS.join(", ")
        ))
    }

    fn not_granted(api: &str) -> Failure {
        config_failure(&format!(
            "Manifest tac.application.permissions.{api} explains an API the app does not use. Add \"{api}\" to tac.browserApis, or remove the reason."
        ))
    }

    fn not_plain(api: &str) -> Failure {
        config_failure(&format!(
            "Manifest tac.application.permissions.{api} must be 1 to {} characters of plain text that tell the user why the app asks, such as \"Shows stores near you\".",
            Self::MOST_CHARACTERS
        ))
    }

    fn missing(api: &str, target: NativeTarget) -> Failure {
        let platform = super::compiler::native_target_directory(target);
        config_failure(&format!(
            "The {platform} app asks the user before it uses {api}, so it needs a reason. tac.browserApis grants \"{api}\": add tac.application.permissions.{api} to client/manifest.json, such as \"Shows stores near you\"."
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::BrowserApiPermissions;
    use std::collections::BTreeMap;
    use tachyon_contracts::NativeTarget;

    fn permissions(granted: &[&str], usage: &[(&str, &str)]) -> BrowserApiPermissions {
        let granted: Vec<String> = granted.iter().map(|grant| String::from(*grant)).collect();
        let usage: BTreeMap<String, String> = usage
            .iter()
            .map(|(api, text)| (String::from(*api), String::from(*text)))
            .collect();
        BrowserApiPermissions::declared(&granted, &usage)
    }

    fn message(failure: &crate::Failure) -> String {
        failure.to_string()
    }

    #[test]
    fn geolocation_needs_a_reason_on_apple_and_android_and_notifications_on_android() {
        let located = permissions(&["geolocation", "notifications"], &[]);
        for target in [
            NativeTarget::Ios,
            NativeTarget::Macos,
            NativeTarget::Android,
        ] {
            let error = message(&located.validate_for(target).expect_err("refused"));
            assert!(error.contains("TY1601"), "{error}");
            assert!(
                error.contains("tac.application.permissions.geolocation"),
                "{error}"
            );
            let platform = super::super::compiler::native_target_directory(target);
            assert!(error.contains(&format!("The {platform} app")), "{error}");
        }
        for target in [NativeTarget::Windows, NativeTarget::Linux] {
            located.validate_for(target).expect("no reason needed");
        }
        let notified = permissions(
            &["geolocation", "notifications"],
            &[("geolocation", "Shows stores near you")],
        );
        let error = message(
            &notified
                .validate_for(NativeTarget::Android)
                .expect_err("refused"),
        );
        assert!(
            error.contains("tac.application.permissions.notifications"),
            "{error}"
        );
        notified
            .validate_for(NativeTarget::Ios)
            .expect("Apple asks with its own text");
    }

    #[test]
    fn an_app_that_does_not_grant_an_asking_api_needs_no_reason() {
        let quiet = permissions(&["clipboard", "share"], &[]);
        for target in [
            NativeTarget::Ios,
            NativeTarget::Macos,
            NativeTarget::Android,
            NativeTarget::Windows,
            NativeTarget::Linux,
        ] {
            quiet.validate_for(target).expect("nothing to explain");
        }
    }

    #[test]
    fn reasons_name_an_asking_api_that_is_granted_in_plain_text() {
        for (granted, usage, expected) in [
            (
                vec!["clipboard"],
                vec![("clipboard", "Copies links")],
                "is not a browser API that asks the user",
            ),
            (
                vec![],
                vec![("geolocation", "Shows stores near you")],
                "explains an API the app does not use",
            ),
            (
                vec!["geolocation"],
                vec![("geolocation", "  ")],
                "must be 1 to 200 characters",
            ),
            (
                vec!["geolocation"],
                vec![("geolocation", "line\nbreak")],
                "must be 1 to 200 characters",
            ),
        ] {
            let error = message(
                &permissions(&granted, &usage)
                    .validate()
                    .expect_err("refused"),
            );
            assert!(error.contains("TY1601"), "{error}");
            assert!(error.contains(expected), "{error}");
        }
        let long = "x".repeat(201);
        assert!(
            permissions(&["notifications"], &[("notifications", &long)])
                .validate()
                .is_err()
        );
        permissions(
            &["geolocation", "notifications"],
            &[
                ("geolocation", "Shows stores near you"),
                ("notifications", "Order updates"),
            ],
        )
        .validate()
        .expect("valid reasons");
    }

    #[test]
    fn hosts_get_the_reasons_escaped_for_their_files() {
        let declared = permissions(
            &["geolocation", "notifications"],
            &[
                ("geolocation", "Stores <near> you & \"yours\""),
                ("notifications", "Order's updates? @once"),
            ],
        );
        assert_eq!(
            declared.location_plist_entries(&BrowserApiPermissions::IOS_LOCATION_KEYS),
            "\n<key>NSLocationWhenInUseUsageDescription</key><string>Stores &lt;near&gt; you &amp; &quot;yours&quot;</string>"
        );
        assert_eq!(
            declared
                .location_plist_entries(&BrowserApiPermissions::MACOS_LOCATION_KEYS)
                .matches("<key>")
                .count(),
            2
        );
        let strings = declared.android_strings();
        assert!(
            strings.contains("<string name=\"tachyon_permission_geolocation\">Stores &lt;near&gt; you &amp; \\&quot;yours\\&quot;</string>"),
            "{strings}"
        );
        assert!(
            strings.contains("<string name=\"tachyon_permission_notifications\">Order\\&apos;s updates\\? \\@once</string>"),
            "{strings}"
        );
        let silent = permissions(&["geolocation"], &[]);
        assert_eq!(
            silent.location_plist_entries(&BrowserApiPermissions::IOS_LOCATION_KEYS),
            ""
        );
        assert_eq!(silent.android_strings(), "");
    }
}

use tachyon_contracts::NativeTarget;

pub(super) struct BrowserApiBridge;

impl BrowserApiBridge {
    #[cfg(test)]
    const OPERATIONS: [(&'static str, &'static [&'static str]); 8] = [
        ("clipboard", &["writeText", "readText"]),
        ("share", &["open"]),
        ("notifications", &["requestPermission", "show"]),
        ("geolocation", &["currentPosition"]),
        ("codeScanner", &["scan", "cancel"]),
        (
            "crypto",
            &[
                "check",
                "makeKeyPair",
                "publicKey",
                "sign",
                "verify",
                "agree",
                "deriveKey",
                "importKey",
                "fromCode",
                "encrypt",
                "decrypt",
                "hkdf",
                "hash",
                "forget",
                "forgetAll",
                "frameKey",
            ],
        ),
        ("media", &["callActive", "speakers", "chooseSpeaker"]),
        ("recorder", &["open", "write", "close"]),
    ];
    #[cfg(test)]
    const EVERY_HOST: [&'static str; 2] = [
        "browserApi.storageSpace.estimate",
        "browserApi.storageSpace.persist",
    ];
    const BRIDGED: [(NativeTarget, &'static [&'static str]); 5] = [
        (
            NativeTarget::Macos,
            &[
                "clipboard",
                "share",
                "notifications",
                "geolocation",
                "codeScanner",
                "crypto",
                "recorder",
            ],
        ),
        (
            NativeTarget::Ios,
            &[
                "clipboard",
                "share",
                "notifications",
                "geolocation",
                "codeScanner",
                "crypto",
                "media",
            ],
        ),
        (
            NativeTarget::Android,
            &["clipboard", "share", "notifications", "media"],
        ),
        (
            NativeTarget::Windows,
            &["clipboard", "share", "notifications"],
        ),
        (NativeTarget::Linux, &["share", "notifications", "crypto"]),
    ];

    const FRAME_KEYS: [NativeTarget; 2] = [NativeTarget::Macos, NativeTarget::Ios];

    pub(super) fn frame_keys(target: NativeTarget) -> String {
        Self::FRAME_KEYS.contains(&target).to_string()
    }

    pub(super) fn bridged(target: NativeTarget) -> &'static [&'static str] {
        Self::BRIDGED
            .iter()
            .find(|(host, _)| *host == target)
            .map_or(&[], |(_, apis)| *apis)
    }

    pub(super) fn page_list(target: NativeTarget) -> String {
        let quoted: Vec<String> = Self::bridged(target)
            .iter()
            .map(|api| format!("'{api}'"))
            .collect();
        format!("[{}]", quoted.join(", "))
    }

    #[cfg(test)]
    fn operations(target: NativeTarget) -> Vec<String> {
        Self::OPERATIONS
            .iter()
            .filter(|(api, _)| Self::bridged(target).contains(api))
            .flat_map(|(api, calls)| {
                calls
                    .iter()
                    .map(move |call| format!("browserApi.{api}.{call}"))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::BrowserApiBridge;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn every_target_lists_only_known_browser_apis_once() {
        for target in [
            NativeTarget::Macos,
            NativeTarget::Ios,
            NativeTarget::Android,
            NativeTarget::Windows,
            NativeTarget::Linux,
        ] {
            assert_eq!(
                BrowserApiBridge::BRIDGED
                    .iter()
                    .filter(|(host, _)| *host == target)
                    .count(),
                1
            );
            let bridged = BrowserApiBridge::bridged(target);
            for api in bridged {
                assert!(
                    BrowserApiBridge::OPERATIONS
                        .iter()
                        .any(|(known, _)| known == api),
                    "{api}"
                );
                assert_eq!(bridged.iter().filter(|other| *other == api).count(), 1);
            }
        }
    }

    #[test]
    fn every_bridged_operation_is_answered_by_its_host() {
        let apple = super::super::apple_browser_apis::AppleBrowserApis::SWIFT;
        for (target, source) in [
            (
                NativeTarget::Macos,
                [
                    super::super::macos::host_source(),
                    apple,
                    super::super::apple_browser_apis::AppleBrowserApis::MACOS_SWIFT,
                ]
                .concat(),
            ),
            (
                NativeTarget::Ios,
                [super::super::ios::host_source(), apple].concat(),
            ),
            (
                NativeTarget::Android,
                super::super::android::host_source().to_owned(),
            ),
            (
                NativeTarget::Windows,
                [
                    super::super::windows::host_source(),
                    super::super::windows_browser_apis::WindowsBrowserApis::SOURCE,
                ]
                .concat(),
            ),
            (
                NativeTarget::Linux,
                [
                    super::super::linux::host_source(),
                    super::super::linux_crypto::LinuxCrypto::SOURCE,
                ]
                .concat(),
            ),
        ] {
            let answered = BrowserApiBridge::operations(target)
                .into_iter()
                .chain(BrowserApiBridge::EVERY_HOST.map(String::from));
            for operation in answered {
                assert!(
                    source.contains(&operation),
                    "the {target:?} host does not answer {operation}"
                );
            }
        }
    }

    #[test]
    fn only_the_apple_hosts_hand_native_frame_keys_to_the_media_worker() {
        for (target, frame_keys) in [
            (NativeTarget::Macos, "true"),
            (NativeTarget::Ios, "true"),
            (NativeTarget::Linux, "false"),
            (NativeTarget::Android, "false"),
            (NativeTarget::Windows, "false"),
        ] {
            assert_eq!(
                BrowserApiBridge::frame_keys(target),
                frame_keys,
                "{target:?}"
            );
        }
    }

    #[test]
    fn the_page_list_is_a_javascript_array_of_names() {
        assert_eq!(
            BrowserApiBridge::page_list(NativeTarget::Windows),
            "['clipboard', 'share', 'notifications']"
        );
        assert_eq!(BrowserApiBridge::operations(NativeTarget::Windows).len(), 5);
        assert_eq!(
            BrowserApiBridge::page_list(NativeTarget::Linux),
            "['share', 'notifications', 'crypto']"
        );
        let linux = BrowserApiBridge::operations(NativeTarget::Linux);
        assert_eq!(
            linux[..3],
            [
                "browserApi.share.open",
                "browserApi.notifications.requestPermission",
                "browserApi.notifications.show"
            ]
        );
        assert_eq!(linux.len(), 19);
        assert_eq!(linux[3], "browserApi.crypto.check");
    }

    #[test]
    fn the_apple_hosts_bridge_every_browser_api() {
        assert_eq!(
            BrowserApiBridge::page_list(NativeTarget::Macos),
            "['clipboard', 'share', 'notifications', 'geolocation', 'codeScanner', 'crypto', 'recorder']"
        );
        assert_eq!(BrowserApiBridge::operations(NativeTarget::Macos).len(), 27);
        assert_eq!(
            BrowserApiBridge::page_list(NativeTarget::Ios),
            "['clipboard', 'share', 'notifications', 'geolocation', 'codeScanner', 'crypto', 'media']"
        );
        assert_eq!(BrowserApiBridge::operations(NativeTarget::Ios).len(), 27);
    }
}

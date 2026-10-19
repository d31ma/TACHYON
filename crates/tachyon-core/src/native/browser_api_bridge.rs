use tachyon_contracts::NativeTarget;

pub(super) struct BrowserApiBridge;

impl BrowserApiBridge {
    #[cfg(test)]
    const OPERATIONS: [(&'static str, &'static [&'static str]); 4] = [
        ("clipboard", &["writeText", "readText"]),
        ("share", &["open"]),
        ("notifications", &["requestPermission", "show"]),
        ("geolocation", &["currentPosition"]),
    ];
    const BRIDGED: [(NativeTarget, &'static [&'static str]); 5] = [
        (
            NativeTarget::Macos,
            &["clipboard", "share", "notifications", "geolocation"],
        ),
        (
            NativeTarget::Ios,
            &["clipboard", "share", "notifications", "geolocation"],
        ),
        (
            NativeTarget::Android,
            &["clipboard", "share", "notifications"],
        ),
        (NativeTarget::Windows, &[]),
        (NativeTarget::Linux, &["share", "notifications"]),
    ];

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
                [super::super::macos::host_source(), apple].concat(),
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
                super::super::windows::host_source().to_owned(),
            ),
            (
                NativeTarget::Linux,
                super::super::linux::host_source().to_owned(),
            ),
        ] {
            for operation in BrowserApiBridge::operations(target) {
                assert!(
                    source.contains(&operation),
                    "the {target:?} host does not answer {operation}"
                );
            }
        }
    }

    #[test]
    fn the_page_list_is_a_javascript_array_of_names() {
        assert_eq!(BrowserApiBridge::page_list(NativeTarget::Windows), "[]");
        assert!(BrowserApiBridge::operations(NativeTarget::Windows).is_empty());
        assert_eq!(
            BrowserApiBridge::page_list(NativeTarget::Linux),
            "['share', 'notifications']"
        );
        assert_eq!(
            BrowserApiBridge::operations(NativeTarget::Linux),
            [
                "browserApi.share.open",
                "browserApi.notifications.requestPermission",
                "browserApi.notifications.show"
            ]
        );
    }

    #[test]
    fn the_apple_hosts_bridge_every_browser_api() {
        for target in [NativeTarget::Macos, NativeTarget::Ios] {
            assert_eq!(
                BrowserApiBridge::page_list(target),
                "['clipboard', 'share', 'notifications', 'geolocation']"
            );
            assert_eq!(BrowserApiBridge::operations(target).len(), 6);
        }
    }
}

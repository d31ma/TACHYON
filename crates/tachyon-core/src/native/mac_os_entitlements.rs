use super::browser_api_permissions::BrowserApiPermissions;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MacOsEntitlements;

impl MacOsEntitlements {
    pub(super) const FILE_NAME: &'static str = "TachyonHost.entitlements";
    pub(super) const LOCATION: &'static str = "com.apple.security.personal-information.location";
    const HEADER: &'static str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n<dict>\n";
    const FOOTER: &'static str = "</dict>\n</plist>\n";

    pub(super) fn plist(permissions: &BrowserApiPermissions) -> String {
        let granted = [("geolocation", Self::LOCATION)];
        let entries: String = granted
            .iter()
            .filter(|(api, _)| permissions.is_granted(api))
            .map(|(_, entitlement)| ["<key>", entitlement, "</key><true/>\n"].concat())
            .collect();
        [Self::HEADER, &entries, Self::FOOTER].concat()
    }
}

#[cfg(test)]
mod tests {
    use super::MacOsEntitlements;
    use crate::native::browser_api_permissions::BrowserApiPermissions;
    use std::collections::BTreeMap;

    #[test]
    fn an_app_without_grants_asks_for_no_entitlement() {
        let plist = MacOsEntitlements::plist(&BrowserApiPermissions::default());
        assert!(plist.contains("<dict>\n</dict>"), "{plist}");
    }

    #[test]
    fn a_geolocation_grant_adds_the_hardened_runtime_location_entitlement() {
        let mut usage = BTreeMap::new();
        usage.insert(
            String::from("geolocation"),
            String::from("Shows stores near you"),
        );
        let permissions = BrowserApiPermissions::declared(
            &[String::from("geolocation"), String::from("clipboard")],
            &usage,
        );
        let plist = MacOsEntitlements::plist(&permissions);
        assert!(
            plist.contains("<key>com.apple.security.personal-information.location</key><true/>"),
            "{plist}"
        );
        assert!(!plist.contains("app-sandbox"), "{plist}");
        assert!(!plist.contains("Shows stores near you"), "{plist}");
    }
}

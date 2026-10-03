pub(crate) struct WebApiClients;

impl WebApiClients {
    pub(crate) const TYPE_NAMES: [&'static str; 10] = [
        "Clock",
        "SecureRandom",
        "Connectivity",
        "PageVisibility",
        "Preferences",
        "Clipboard",
        "Share",
        "Notifications",
        "Geolocation",
        "WindowControls",
    ];

    const PRIVATE_CLIENTS: [&'static str; 3] = ["FilePicker", "PreferenceStrings", "DocumentTitle"];
    const OWNER_RESOURCES: [&'static str; 3] = [
        "PreferenceStrings",
        "DocumentTitle",
        "ApplicationEventPublisher",
    ];

    pub(crate) fn browser_resources() -> std::collections::BTreeSet<String> {
        Self::TYPE_NAMES
            .iter()
            .chain(Self::OWNER_RESOURCES.iter())
            .map(|name| String::from(*name))
            .collect()
    }

    pub(crate) fn is_client_type(declared: &str) -> bool {
        Self::client_type(declared)
    }

    fn client_type(declared: &str) -> bool {
        Self::PRIVATE_CLIENTS.contains(&declared) || Self::TYPE_NAMES.contains(&declared)
    }
}

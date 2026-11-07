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

    pub(crate) const NATIVE_CAPABILITIES: [&'static str; 2] = ["FilePicker", "WindowControls"];

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

    pub(crate) fn companion_resources() -> std::collections::BTreeSet<String> {
        let mut resources = Self::browser_resources();
        resources.extend(Self::NATIVE_CAPABILITIES.map(String::from));
        resources
    }

    pub(crate) fn native_capabilities() -> std::collections::BTreeSet<String> {
        Self::NATIVE_CAPABILITIES.map(String::from).into()
    }

    pub(crate) fn client_type_names() -> impl Iterator<Item = &'static str> {
        Self::PRIVATE_CLIENTS.into_iter().chain(Self::TYPE_NAMES)
    }

    pub(crate) fn is_client_type(declared: &str) -> bool {
        Self::client_type(declared)
    }

    fn client_type(declared: &str) -> bool {
        Self::PRIVATE_CLIENTS.contains(&declared) || Self::TYPE_NAMES.contains(&declared)
    }
}

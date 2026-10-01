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

    const PRIVATE_CLIENTS: [&'static str; 2] = ["FilePicker", "PreferenceStrings"];

    pub(crate) fn is_client_type(declared: &str) -> bool {
        Self::client_type(declared)
    }

    fn client_type(declared: &str) -> bool {
        Self::PRIVATE_CLIENTS.contains(&declared) || Self::TYPE_NAMES.contains(&declared)
    }
}

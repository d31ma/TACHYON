pub(crate) struct WebApiClients;

impl WebApiClients {
    pub(crate) const TYPE_NAMES: [&'static str; 9] = [
        "Clock",
        "SecureRandom",
        "Connectivity",
        "PageVisibility",
        "Preferences",
        "Clipboard",
        "Share",
        "Notifications",
        "Geolocation",
    ];

    pub(crate) fn is_client_type(declared: &str) -> bool {
        Self::TYPE_NAMES.contains(&declared)
    }
}

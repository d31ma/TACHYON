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

    pub(crate) fn is_client_type(declared: &str) -> bool {
        declared == "FilePicker" || Self::TYPE_NAMES.contains(&declared)
    }
}

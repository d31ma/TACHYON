use std::path::{Path, PathBuf};

pub(super) struct JavaWebApis;

impl JavaWebApis {
    const HOST: (&'static str, &'static str) =
        ("TacWebHost", include_str!("prelude/TacWebHost.java"));
    const CLIENTS: [(&'static str, &'static str); 9] = [
        ("Clock", include_str!("../web_apis/Clock.java")),
        (
            "SecureRandom",
            include_str!("../web_apis/SecureRandom.java"),
        ),
        (
            "Connectivity",
            include_str!("../web_apis/Connectivity.java"),
        ),
        (
            "PageVisibility",
            include_str!("../web_apis/PageVisibility.java"),
        ),
        ("Preferences", include_str!("../web_apis/Preferences.java")),
        ("Clipboard", include_str!("../web_apis/Clipboard.java")),
        ("Share", include_str!("../web_apis/Share.java")),
        (
            "Notifications",
            include_str!("../web_apis/Notifications.java"),
        ),
        ("Geolocation", include_str!("../web_apis/Geolocation.java")),
    ];
    const EVENTS_FILE: &'static str = "tachyon/TacWebEvents.java";
    const EXTENSION: &'static str = "java";

    pub(super) fn clients(folder: &Path, authored: &str) -> Vec<(PathBuf, String)> {
        let used: Vec<&(&str, &str)> = Self::CLIENTS
            .iter()
            .filter(|(name, _)| authored.contains(name))
            .collect();
        if used.is_empty() {
            return Vec::new();
        }
        std::iter::once(&Self::HOST)
            .chain(used)
            .map(|(name, text)| Self::file(folder, name, text))
            .collect()
    }

    pub(super) fn events(authored: &str) -> Option<(PathBuf, String)> {
        let records = crate::route_types::WebEventSources::java_used_by(authored);
        (!records.is_empty()).then(|| (PathBuf::from(Self::EVENTS_FILE), records))
    }

    fn file(folder: &Path, name: &str, text: &str) -> (PathBuf, String) {
        let path = folder.join(name).with_extension(Self::EXTENSION);
        (path, String::from(text))
    }
}

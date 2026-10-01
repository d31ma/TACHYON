use crate::route_types::WebEventSources;

pub(super) struct CsharpWebApis;

impl CsharpWebApis {
    pub(super) const SOURCE: &'static str = concat!(
        include_str!("web_apis/TacWebHost.cs"),
        include_str!("web_apis/Clock.cs"),
        include_str!("web_apis/SecureRandom.cs"),
        include_str!("web_apis/Connectivity.cs"),
        include_str!("web_apis/PageVisibility.cs"),
        include_str!("web_apis/Preferences.cs"),
        include_str!("web_apis/Clipboard.cs"),
        include_str!("web_apis/Share.cs"),
        include_str!("web_apis/Notifications.cs"),
        include_str!("web_apis/Geolocation.cs"),
        include_str!("web_apis/WindowControls.cs"),
        include_str!("capabilities/FilePickerStatus.cs"),
        include_str!("capabilities/FilePickerResult.cs"),
        include_str!("capabilities/FilePickerOptions.cs"),
        include_str!("capabilities/NativeCapabilityOwner.cs"),
        include_str!("capabilities/NativeCapabilities.cs"),
        include_str!("capabilities/FilePicker.cs"),
        include_str!("capabilities/NativeCapabilityTransport.cs")
    );
    const LIBRARY_USINGS: &'static str = "using System.Text.Json.Nodes;\n\n";

    pub(super) fn with_events() -> String {
        format!("{}{}", Self::SOURCE, WebEventSources::csharp())
    }

    pub(super) fn in_web_services(fsharp: &str) -> String {
        let own = fsharp.is_empty().then(Self::with_events);
        own.unwrap_or_default()
    }

    pub(super) fn native() -> String {
        let source = Self::SOURCE.replace(
            include_str!("web_apis/WindowControls.cs"),
            include_str!("capabilities/WindowControls.cs"),
        );
        format!("{source}{}", WebEventSources::csharp())
    }

    pub(super) fn library() -> String {
        format!("{}{}", Self::LIBRARY_USINGS, Self::SOURCE)
    }
}

#[cfg(test)]
mod tests {
    use super::CsharpWebApis;

    #[test]
    fn web_services_keep_their_own_browser_apis_only_without_fsharp() {
        let own = CsharpWebApis::in_web_services("");
        assert!(own.contains("public static class TacWebHost"));
        assert!(own.contains("public sealed record ConnectivityChanged(bool Online);"));
        assert!(CsharpWebApis::in_web_services("namespace rec TachyonService0\n").is_empty());
    }

    #[test]
    fn the_library_carries_the_browser_apis_without_the_event_records() {
        let library = CsharpWebApis::library();
        assert!(library.starts_with("using System.Text.Json.Nodes;"));
        assert!(library.contains("public sealed class Clock"));
        assert!(library.contains("public sealed class Geolocation"));
        assert!(library.contains("public sealed class WindowControls"));
        assert!(!library.contains("ConnectivityChanged"));
    }
}

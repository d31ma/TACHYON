use super::file_picker_sources::FilePickerSources;
use crate::route_types::WebEventSources;

pub(crate) struct CsharpWebApis;

impl CsharpWebApis {
    const BROWSER: &'static str = concat!(
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
        include_str!("web_apis/WindowControls.cs")
    );

    const LIBRARY_USINGS: &'static str = "using System.Text.Json.Nodes;\n\n";
    const BROWSER_FAKES: &'static str = include_str!("web_apis/BrowserFakes.cs");

    pub(super) fn with_events() -> String {
        format!(
            "{}{}{}",
            Self::BROWSER,
            FilePickerSources::csharp(),
            WebEventSources::csharp()
        )
    }

    pub(super) fn in_web_services(fsharp: &str) -> String {
        let own = fsharp.is_empty().then(Self::with_events);
        own.unwrap_or_default()
    }

    pub(super) fn native() -> String {
        let source = Self::BROWSER.replace(
            include_str!("web_apis/WindowControls.cs"),
            include_str!("capabilities/WindowControls.cs"),
        );
        format!(
            "{source}{}{}",
            FilePickerSources::csharp_native(),
            WebEventSources::csharp()
        )
    }

    pub(super) fn library() -> String {
        format!(
            "{}{}{}",
            Self::LIBRARY_USINGS,
            Self::BROWSER,
            FilePickerSources::csharp()
        )
    }

    pub(crate) fn test_unit_library() -> String {
        format!("{}{}", Self::library(), Self::BROWSER_FAKES)
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

    #[test]
    fn a_test_unit_library_answers_with_browser_fakes() {
        let library = CsharpWebApis::test_unit_library();
        assert!(library.contains("public sealed class Clock"));
        assert!(library.contains("public static class BrowserFakes"));
    }
}

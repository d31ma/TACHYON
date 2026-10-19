use super::config::WindowConfiguration;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MacOsTitleBarSource;

impl MacOsTitleBarSource {
    const SWIFT_TYPES: &str = concat!(
        include_str!("title_bar/MacOsTitleBarSettings.swift"),
        include_str!("title_bar/MacOsTitleBarInsets.swift"),
        include_str!("title_bar/MacOsDragRegions.swift"),
        include_str!("title_bar/MacOsWindowButtons.swift"),
        include_str!("title_bar/MacOsTitleBarWebView.swift"),
        include_str!("title_bar/MacOsTitleBar.swift"),
    );
    const PAGE_SCRIPT: &str = include_str!("title_bar/tac-title-bar.js");
    const PAGE_SCRIPT_MARKER: &str = "__NATIVE_TITLE_BAR_PAGE_SCRIPT__";
    const BRIDGE_CASE: &str = "        case \"window.dragRegions\":\n            return titleBar?.updateRegions(payload) ?? \"{\\\"error\\\":\\\"Native title bar unavailable.\\\"}\"\n";

    pub(super) fn swift_types() -> String {
        Self::SWIFT_TYPES.replace(Self::PAGE_SCRIPT_MARKER, Self::PAGE_SCRIPT)
    }

    pub(super) fn settings(window: &WindowConfiguration) -> String {
        let inset = window.title_bar_button_inset.map_or_else(
            || String::from("nil"),
            |inset| format!("NSPoint(x: {}, y: {})", inset.leading, inset.top),
        );
        let integrated = window.title_bar.is_integrated();
        format!(
            "MacOsTitleBarSettings(integrated: {integrated}, buttonInset: {inset}, applicationName: \"__APP_NAME__\")"
        )
    }

    pub(super) fn bridge_case(window: &WindowConfiguration) -> &'static str {
        Self::bridge_case_for(window.title_bar.is_integrated())
    }

    fn bridge_case_for(integrated: bool) -> &'static str {
        if integrated { Self::BRIDGE_CASE } else { "" }
    }
}

#[cfg(test)]
mod tests {
    use super::MacOsTitleBarSource;
    use crate::native::config::{NativeApplication, WindowConfiguration};
    use crate::native::native_title_bar::NativeTitleBar;
    use crate::native::native_title_bar_button_inset::NativeTitleBarButtonInset;

    fn application(window: WindowConfiguration) -> NativeApplication {
        NativeApplication {
            name: String::from("Title Bar Check"),
            executable_name: String::from("TitleBarCheck"),
            application_id: String::from("dev.tachyon.title-bar-check"),
            version: String::from("1.0.0"),
            build: 1,
            entry_route: String::from("/"),
            icons: Vec::new(),
            window,
            native_capabilities: Vec::new(),
            menus: crate::native::native_menu_config::NativeMenuConfig::default(),
            rust: crate::native::native_rust_project::NativeRustProject::default(),
            outgoing: crate::native::native_outgoing_hosts::NativeOutgoingHosts::default(),
            permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
            minimum_system: crate::native::minimum_system::MinimumSystem::default(),
            description: String::new(),
        }
    }

    fn host(window: WindowConfiguration) -> String {
        let index = crate::native::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: Vec::new(),
        };
        crate::native::macos::swift_source(&application(window), "tacNativeInvoke(payload)", &index)
    }

    #[test]
    fn a_standard_window_keeps_the_standard_title_bar_and_no_drag_region_call() {
        let window = WindowConfiguration::default();
        assert_eq!(
            MacOsTitleBarSource::settings(&window),
            "MacOsTitleBarSettings(integrated: false, buttonInset: nil, applicationName: \"__APP_NAME__\")"
        );
        let source = host(window);
        assert!(source.contains(
            "MacOsTitleBarSettings(integrated: false, buttonInset: nil, applicationName: \"Title Bar Check\")"
        ));
        assert!(!source.contains("case \"window.dragRegions\""));
        assert!(source.contains("styleMask: titleBar.styleMask"));
    }

    #[test]
    fn an_integrated_window_embeds_the_inset_the_page_script_and_the_drag_region_call() {
        let window = WindowConfiguration {
            title_bar: NativeTitleBar::Integrated,
            title_bar_button_inset: Some(NativeTitleBarButtonInset {
                leading: 20,
                top: 35,
            }),
            ..WindowConfiguration::default()
        };
        let source = host(window);
        assert!(source.contains(
            "MacOsTitleBarSettings(integrated: true, buttonInset: NSPoint(x: 20, y: 35), applicationName: \"Title Bar Check\")"
        ));
        assert!(source.contains(
            "case \"window.dragRegions\":\n            return titleBar?.updateRegions(payload)"
        ));
        assert!(source.contains("document.querySelectorAll('[data-tac-drag-region]')"));
        assert!(source.contains("--tac-titlebar-leading-inset"));
        assert!(source.contains(
            "let view = MacOsTitleBarWebView(frame: .zero, configuration: configuration)"
        ));
        assert!(source.contains("mask.insert(.fullSizeContentView)"));
        assert!(!source.contains("__NATIVE_TITLE_BAR"));
    }

    #[test]
    fn the_page_script_fits_in_a_swift_raw_string() {
        assert!(!MacOsTitleBarSource::PAGE_SCRIPT.contains("\"\"\"#"));
        assert!(MacOsTitleBarSource::SWIFT_TYPES.contains(MacOsTitleBarSource::PAGE_SCRIPT_MARKER));
    }
}

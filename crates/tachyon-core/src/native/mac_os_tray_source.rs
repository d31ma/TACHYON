use super::config::NativeApplication;
use super::host::quoted_string_escape;
use super::native_menu_entry::NativeMenuEntry;
use super::native_tray::NativeTray;

pub(super) struct MacOsTraySource;

impl MacOsTraySource {
    const POLICIES: [&'static str; 2] = [".accessory", ".regular"];
    const HIDES: [&'static str; 2] = ["false", "true"];
    const AGENT: [&'static str; 2] = ["\n<key>LSUIElement</key><true/>", ""];

    pub(super) fn install(application: &NativeApplication) -> String {
        application
            .tray
            .as_ref()
            .map_or_else(String::new, |tray| Self::declared(tray, &application.name))
    }

    pub(super) fn hides_window(application: &NativeApplication) -> &'static str {
        Self::HIDES[usize::from(
            application
                .tray
                .as_ref()
                .is_some_and(|tray| tray.close_hides_window),
        )]
    }

    pub(super) fn activation_policy(application: &NativeApplication) -> &'static str {
        Self::POLICIES[usize::from(Self::shows_dock_icon(application))]
    }

    pub(super) fn plist_entry(application: &NativeApplication) -> &'static str {
        Self::AGENT[usize::from(Self::shows_dock_icon(application))]
    }

    fn shows_dock_icon(application: &NativeApplication) -> bool {
        application
            .tray
            .as_ref()
            .is_none_or(NativeTray::shows_dock_icon)
    }

    fn declared(tray: &NativeTray, name: &str) -> String {
        let items = tray
            .items
            .iter()
            .map(Self::item)
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "tray = MacOsTray(\"{}\", \"{}\", {}, \"{}\", [{items}], tachyonMenuReady, tachyonMenuPerform, {{ [weak self] (route: String?) -> Void in self?.showWindow(route) }}, {{ [weak self] () -> Bool in self?.webView != nil }})",
            quoted_string_escape(name),
            quoted_string_escape(tray.icon_path()),
            tray.template,
            quoted_string_escape(tray.tooltip_or(name)),
        )
    }

    fn item(entry: &NativeMenuEntry) -> String {
        format!(
            "MacOsTrayItem(id: \"{}\", label: \"{}\", route: \"{}\", event: \"{}\")",
            quoted_string_escape(&entry.id),
            quoted_string_escape(&entry.label),
            quoted_string_escape(entry.route.as_deref().unwrap_or_default()),
            entry
                .action
                .as_ref()
                .and_then(|action| action.event.as_deref())
                .unwrap_or_default(),
        )
    }
}

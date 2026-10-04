use super::native_menu_config::NativeMenuConfig;
use super::native_menu_entry::NativeMenuEntry;

pub(super) struct MacOsMenuSource;

impl MacOsMenuSource {
    pub(super) fn declarations(config: &NativeMenuConfig) -> String {
        Self::listed(config, |group, entry| {
            entry
                .route
                .as_deref()
                .map(|route| Self::route(group, entry, route))
        })
    }

    pub(super) fn actions(config: &NativeMenuConfig) -> String {
        Self::listed(config, |group, entry| {
            entry.action.as_ref().map(|action| {
                format!(
                    "MacOsMenuAction(id: \"{}\", label: \"{}\", group: .{group}, event: \"{}\", shortcut: {})",
                    super::host::quoted_string_escape(&entry.id),
                    super::host::quoted_string_escape(&entry.label),
                    action.event.as_deref().unwrap_or_default(),
                    Self::shortcut(entry)
                )
            })
        })
    }

    fn listed(
        config: &NativeMenuConfig,
        declaration: impl Fn(&str, &NativeMenuEntry) -> Option<String>,
    ) -> String {
        let entries = config
            .groups()
            .into_iter()
            .flat_map(|(group, entries)| entries.iter().map(move |entry| (group, entry)))
            .filter_map(|(group, entry)| declaration(group, entry))
            .collect::<Vec<_>>();
        format!("[{}]", entries.join(",\n"))
    }

    fn route(group: &str, entry: &NativeMenuEntry, route: &str) -> String {
        format!(
            "MacOsMenuRoute(id: \"{}\", label: \"{}\", destination: MacOsMenuDestination(group: .{group}, route: \"{}\"), shortcut: {})",
            super::host::quoted_string_escape(&entry.id),
            super::host::quoted_string_escape(&entry.label),
            super::host::quoted_string_escape(route),
            Self::shortcut(entry)
        )
    }

    fn shortcut(entry: &NativeMenuEntry) -> String {
        entry.shortcut.as_ref().map_or_else(
            || "MacOsMenuShortcut(key: \"\", modifiers: [])".to_owned(),
            |shortcut| {
                format!(
                    "MacOsMenuShortcut(key: \"{}\", modifiers: [{}])",
                    shortcut.key,
                    shortcut
                        .modifiers
                        .iter()
                        .map(|modifier| format!(".{modifier}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        )
    }
}

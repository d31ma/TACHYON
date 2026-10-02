use super::native_menu_config::NativeMenuConfig;
use super::native_menu_entry::NativeMenuEntry;

pub(super) struct MacOsMenuSource;

impl MacOsMenuSource {
    pub(super) fn declarations(config: &NativeMenuConfig) -> String {
        let items = [
            ("file", &config.file),
            ("view", &config.view),
            ("help", &config.help),
        ];
        let entries = items
            .into_iter()
            .flat_map(|(group, entries)| entries.iter().map(move |entry| Self::entry(group, entry)))
            .collect::<Vec<_>>();
        format!("[{}]", entries.join(",\n"))
    }

    fn entry(group: &str, entry: &NativeMenuEntry) -> String {
        let shortcut = Self::shortcut(entry);
        format!(
            "MacOsMenuRoute(id: \"{}\", label: \"{}\", destination: MacOsMenuDestination(group: .{group}, route: \"{}\"), shortcut: {shortcut})",
            super::host::quoted_string_escape(&entry.id),
            super::host::quoted_string_escape(&entry.label),
            super::host::quoted_string_escape(&entry.route)
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

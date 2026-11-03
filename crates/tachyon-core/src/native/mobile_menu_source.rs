use super::native_menu_config::NativeMenuConfig;
use super::native_menu_entry::NativeMenuEntry;

pub(super) struct MobileMenuSource;

impl MobileMenuSource {
    pub(super) fn swift_routes(config: &NativeMenuConfig) -> String {
        format!("[{}]", Self::routes(config, Self::swift_route).join(",\n"))
    }

    pub(super) fn java_routes(config: &NativeMenuConfig) -> String {
        format!("{{{}}}", Self::routes(config, Self::java_route).join(",\n"))
    }

    fn routes(
        config: &NativeMenuConfig,
        declaration: fn(&str, &NativeMenuEntry) -> String,
    ) -> Vec<String> {
        config
            .groups()
            .into_iter()
            .flat_map(|(group, entries)| entries.iter().map(move |entry| declaration(group, entry)))
            .collect()
    }

    fn swift_route(group: &str, entry: &NativeMenuEntry) -> String {
        format!(
            "IosMenuRoute(id: \"{}\", label: \"{}\", group: \"{group}\", route: \"{}\", key: \"{}\", modifiers: [{}]{})",
            super::host::quoted_string_escape(&entry.id),
            super::host::quoted_string_escape(&entry.label),
            super::host::quoted_string_escape(entry.route.as_deref().unwrap_or_default()),
            Self::key(entry),
            Self::modifiers(entry, Self::swift_modifier).join(", "),
            Self::swift_action(entry),
        )
    }

    fn swift_action(entry: &NativeMenuEntry) -> String {
        entry
            .action
            .as_ref()
            .map(|action| {
                format!(
                    ", event: \"{}\", action: true",
                    super::host::quoted_string_escape(action.event.as_deref().unwrap_or_default())
                )
            })
            .unwrap_or_default()
    }

    fn java_route(group: &str, entry: &NativeMenuEntry) -> String {
        let mut names = Self::modifiers(entry, Self::java_modifier);
        names.dedup();
        let none = ["", "0"][usize::from(names.is_empty())];
        let (kind, destination) = Self::java_destination(entry);
        format!(
            "{kind}({}, {}, \"{group}\", {}, \"{}\", {}{none})",
            Self::java_string(&entry.id),
            Self::java_string(&entry.label),
            Self::java_string(destination),
            Self::key(entry),
            names.join(" | "),
        )
    }

    fn java_destination(entry: &NativeMenuEntry) -> (&'static str, &str) {
        entry.action.as_ref().map_or(
            ("new MenuRoute", entry.route.as_deref().unwrap_or_default()),
            |action| {
                (
                    "MenuRoute.action",
                    action.event.as_deref().unwrap_or_default(),
                )
            },
        )
    }

    fn java_string(value: &str) -> String {
        serde_json::to_string(value).unwrap_or_else(|_| String::from("\"\""))
    }

    fn key(entry: &NativeMenuEntry) -> &str {
        entry
            .shortcut
            .as_ref()
            .map_or("", |shortcut| shortcut.key.as_str())
    }

    fn modifiers(entry: &NativeMenuEntry, spelling: fn(&str) -> &'static str) -> Vec<&'static str> {
        let mut modifiers = entry
            .shortcut
            .iter()
            .flat_map(|shortcut| shortcut.modifiers.iter())
            .map(|modifier| spelling(modifier))
            .collect::<Vec<_>>();
        modifiers.sort_unstable();
        modifiers
    }

    fn swift_modifier(modifier: &str) -> &'static str {
        match modifier {
            "control" => ".control",
            "option" => ".alternate",
            "shift" => ".shift",
            _ => ".command",
        }
    }

    fn java_modifier(modifier: &str) -> &'static str {
        match modifier {
            "option" => "KeyEvent.META_ALT_ON",
            "shift" => "KeyEvent.META_SHIFT_ON",
            _ => "KeyEvent.META_CTRL_ON",
        }
    }
}

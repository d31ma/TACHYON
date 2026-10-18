use super::native_menu_config::NativeMenuConfig;
use super::native_menu_entry::NativeMenuEntry;
use super::native_menu_shortcut::NativeMenuShortcut;

pub(super) struct DesktopMenuItems;

impl DesktopMenuItems {
    const ITEM_TYPE: &'static str = "#define TACHYON_MENU_FILE 0
#define TACHYON_MENU_VIEW 1
#define TACHYON_MENU_HELP 2
#define TACHYON_MENU_PRIMARY 1
#define TACHYON_MENU_ALT 2
#define TACHYON_MENU_SHIFT 4
#define TACHYON_MENU_LIMIT 64

typedef struct {
  const char *id;
  const char *label;
  int group;
  const char *route;
  const char *event;
  char key;
  int modifiers;
} TachyonMenuItem;
";
    const EMPTY_ROW: &'static str = "{NULL, NULL, TACHYON_MENU_FILE, NULL, NULL, 0, 0}";
    const PRIMARY: u8 = 1;
    const ALT: u8 = 2;
    const SHIFT: u8 = 4;

    pub(super) fn c_table(config: &NativeMenuConfig, mnemonic: char) -> String {
        let rows = config
            .groups()
            .into_iter()
            .flat_map(|(group, entries)| entries.iter().map(move |entry| (group, entry)))
            .map(|(group, entry)| Self::row(group, entry, mnemonic))
            .collect::<Vec<_>>();
        let count = rows.len();
        let listed = Some(rows.join(",\n  "))
            .filter(|listed| !listed.is_empty())
            .unwrap_or_else(|| Self::EMPTY_ROW.to_owned());
        format!(
            "{}\nstatic const int TACHYON_MENU_ITEM_COUNT = {count};\nstatic const TachyonMenuItem TACHYON_MENU_ITEMS[] = {{\n  {listed}\n}};\n",
            Self::ITEM_TYPE
        )
    }

    fn row(group: &str, entry: &NativeMenuEntry, mnemonic: char) -> String {
        let label = entry
            .label
            .replace(mnemonic, &format!("{mnemonic}{mnemonic}"));
        let event = entry
            .action
            .as_ref()
            .map(|action| Self::quoted(action.event.as_deref().unwrap_or_default()));
        let (key, modifiers) = entry
            .shortcut
            .as_ref()
            .map_or((String::from("0"), 0), |shortcut| {
                (format!("'{}'", shortcut.key), Self::modifiers(shortcut))
            });
        format!(
            "{{{}, {}, TACHYON_MENU_{}, {}, {}, {key}, {modifiers}}}",
            Self::quoted(&entry.id),
            Self::quoted(&label),
            group.to_ascii_uppercase(),
            entry
                .route
                .as_deref()
                .map_or_else(|| String::from("NULL"), Self::quoted),
            event.unwrap_or_else(|| String::from("NULL")),
        )
    }

    fn modifiers(shortcut: &NativeMenuShortcut) -> u8 {
        shortcut
            .modifiers
            .iter()
            .map(|modifier| Self::modifier_bit(modifier))
            .fold(0, |bits, bit| bits | bit)
    }

    fn modifier_bit(modifier: &str) -> u8 {
        match modifier {
            "command" | "control" => Self::PRIMARY,
            "option" => Self::ALT,
            "shift" => Self::SHIFT,
            _ => 0,
        }
    }

    fn quoted(value: &str) -> String {
        let text = super::host::c_string_escape(value)
            .bytes()
            .map(Self::portable_byte)
            .collect::<String>();
        format!("\"{text}\"")
    }

    fn portable_byte(byte: u8) -> String {
        if byte.is_ascii() {
            char::from(byte).to_string()
        } else {
            format!("\\{byte:03o}")
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::DesktopMenuItems;
    use crate::native::native_menu_config::NativeMenuConfig;
    use serde_json::json;

    fn menus() -> NativeMenuConfig {
        serde_json::from_value(json!({
            "file": [
                {"id":"refresh","label":"Refresh & Reload","action":{"service":"SectionMenuService","method":"refresh"}}
            ],
            "view": [
                {"id":"show_running","label":"What's_Running","action":{"service":"SectionMenuService","method":"show_running","event":"SectionChosen"},"shortcut":{"key":"1","modifiers":["command","shift"]}},
                {"id":"full","label":"Caf\u{e9}","route":"/full","shortcut":{"key":"k","modifiers":["command","control"]}}
            ],
            "help": [
                {"id":"guide","label":"Guide","route":"/help","shortcut":{"key":"g","modifiers":["command","option"]}}
            ]
        }))
        .expect("menu declarations")
    }

    #[test]
    fn every_declared_item_becomes_one_c_row_in_its_group() {
        let table = DesktopMenuItems::c_table(&menus(), '_');
        assert!(table.contains("static const int TACHYON_MENU_ITEM_COUNT = 4;"));
        assert!(
            table.contains(
                "{\"refresh\", \"Refresh & Reload\", TACHYON_MENU_FILE, NULL, \"\", 0, 0}"
            )
        );
        assert!(table.contains(
            "{\"show_running\", \"What's__Running\", TACHYON_MENU_VIEW, NULL, \"SectionChosen\", '1', 5}"
        ));
        assert!(
            table.contains("{\"guide\", \"Guide\", TACHYON_MENU_HELP, \"/help\", NULL, 'g', 3}")
        );
    }

    #[test]
    fn a_shortcut_with_control_uses_ctrl_and_labels_stay_portable_c() {
        let table = DesktopMenuItems::c_table(&menus(), '&');
        assert!(
            table.contains(
                "{\"full\", \"Caf\\303\\251\", TACHYON_MENU_VIEW, \"/full\", NULL, 'k', 1}"
            )
        );
        assert!(table.contains("\"Refresh && Reload\""));
        assert!(table.is_ascii());
    }

    #[test]
    fn no_declared_items_keep_one_unused_row_so_the_c_array_is_valid() {
        let table = DesktopMenuItems::c_table(&NativeMenuConfig::default(), '_');
        assert!(table.contains("static const int TACHYON_MENU_ITEM_COUNT = 0;"));
        assert!(table.contains("{NULL, NULL, TACHYON_MENU_FILE, NULL, NULL, 0, 0}"));
    }
}

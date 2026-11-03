use super::native_menu_config::NativeMenuConfig;
use super::native_menu_entry::NativeMenuEntry;
use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct NativeTray {
    pub(super) icon: String,
    #[serde(default)]
    pub(super) tooltip: Option<String>,
    #[serde(default)]
    pub(super) template: bool,
    #[serde(default)]
    pub(super) items: Vec<NativeMenuEntry>,
    #[serde(default)]
    pub(super) close_hides_window: bool,
    #[serde(default)]
    pub(super) dock: Option<bool>,
}

impl NativeTray {
    const MAX_ITEMS: usize = 32;
    const MAX_TOOLTIP: usize = 127;

    pub(super) fn validate(&self, menus: &NativeMenuConfig) -> Result<(), Failure> {
        self.require_icon()?;
        self.require_tooltip()?;
        self.require_count()?;
        self.items.iter().try_for_each(NativeMenuEntry::validate)?;
        self.items.iter().try_for_each(Self::require_no_shortcut)?;
        self.require_ids(menus)
    }

    pub(super) fn shows_dock_icon(&self) -> bool {
        self.dock.unwrap_or(true)
    }

    pub(super) fn tooltip_or<'a>(&'a self, name: &'a str) -> &'a str {
        self.tooltip.as_deref().unwrap_or(name)
    }

    pub(super) fn icon_path(&self) -> &str {
        self.icon.trim_start_matches('/')
    }

    fn require_icon(&self) -> Result<(), Failure> {
        let valid = [
            super::config::valid_icon_source(&self.icon),
            self.icon.to_ascii_lowercase().ends_with(".png"),
            self.icon.len() <= 512,
        ] == [true; 3];
        valid.then_some(()).ok_or_else(|| {
            super::config::config_failure(
                "Manifest tac.application.tray.icon must name a PNG file in the client bundle, such as /shared/assets/tray.png.",
            )
        })
    }

    fn require_tooltip(&self) -> Result<(), Failure> {
        self.tooltip
            .as_deref()
            .is_none_or(Self::valid_tooltip)
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(
                    "Manifest tac.application.tray.tooltip must contain 1 to 127 UTF-8 bytes without control characters.",
                )
            })
    }

    fn valid_tooltip(tooltip: &str) -> bool {
        [
            !tooltip.trim().is_empty(),
            tooltip.len() <= Self::MAX_TOOLTIP,
            !tooltip.chars().any(char::is_control),
        ] == [true; 3]
    }

    fn require_count(&self) -> Result<(), Failure> {
        (self.items.len() <= Self::MAX_ITEMS)
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(
                    "Manifest tac.application.tray.items may declare at most 32 items.",
                )
            })
    }

    fn require_no_shortcut(item: &NativeMenuEntry) -> Result<(), Failure> {
        item.shortcut.is_none().then_some(()).ok_or_else(|| {
            super::config::config_failure(&format!(
                "Tray item '{}' cannot have a shortcut: a tray menu has no keyboard shortcuts.",
                item.id
            ))
        })
    }

    fn require_ids(&self, menus: &NativeMenuConfig) -> Result<(), Failure> {
        let ids = self
            .items
            .iter()
            .chain(menus.items())
            .map(|item| &item.id)
            .collect::<std::collections::BTreeSet<_>>();
        (ids.len() == self.items.len() + menus.items().count())
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(
                    "Tray item ids must be unique and must differ from the ids of the File, View and Help menu items.",
                )
            })
    }
}

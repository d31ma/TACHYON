use super::native_menu_entry::NativeMenuEntry;
use super::routes::NativeRouteIndex;
use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeMenuConfig {
    #[serde(default)]
    pub(super) file: Vec<NativeMenuEntry>,
    #[serde(default)]
    pub(super) view: Vec<NativeMenuEntry>,
    #[serde(default)]
    pub(super) help: Vec<NativeMenuEntry>,
}

impl NativeMenuConfig {
    const MAX_ITEMS: usize = 64;

    pub(super) fn validate(&self) -> Result<(), Failure> {
        self.require_count()?;
        self.items().try_for_each(NativeMenuEntry::validate)?;
        self.require_ids()?;
        self.require_shortcuts()
    }

    const DESKTOP_TARGETS: [tachyon_contracts::NativeTarget; 3] = [
        tachyon_contracts::NativeTarget::Macos,
        tachyon_contracts::NativeTarget::Linux,
        tachyon_contracts::NativeTarget::Windows,
    ];

    const CTRL_TARGETS: [tachyon_contracts::NativeTarget; 2] = [
        tachyon_contracts::NativeTarget::Linux,
        tachyon_contracts::NativeTarget::Windows,
    ];

    pub(super) fn validate_for(
        &self,
        target: tachyon_contracts::NativeTarget,
        index: &NativeRouteIndex,
    ) -> Result<(), Failure> {
        self.require_target(target)?;
        self.require_desktop_shortcuts(target)?;
        self.items()
            .try_for_each(|item| Self::require_document(item, index))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.items().next().is_none()
    }

    pub(super) fn groups(&self) -> [(&'static str, &[NativeMenuEntry]); 3] {
        [
            ("file", &self.file),
            ("view", &self.view),
            ("help", &self.help),
        ]
    }

    pub(super) fn items(&self) -> impl Iterator<Item = &NativeMenuEntry> {
        self.file.iter().chain(&self.view).chain(&self.help)
    }

    fn require_count(&self) -> Result<(), Failure> {
        (self.items().count() <= Self::MAX_ITEMS)
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure("Native menus may declare at most 64 items in total.")
            })
    }

    fn require_ids(&self) -> Result<(), Failure> {
        let ids = self
            .items()
            .map(|item| &item.id)
            .collect::<std::collections::BTreeSet<_>>();
        (ids.len() == self.items().count())
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(
                    "Native menu ids must be unique across File, View and Help.",
                )
            })
    }

    fn require_shortcuts(&self) -> Result<(), Failure> {
        let shortcuts = self
            .items()
            .filter_map(|item| item.shortcut.as_ref())
            .map(super::native_menu_shortcut::NativeMenuShortcut::identity)
            .collect::<Vec<_>>();
        let unique = shortcuts.iter().collect::<std::collections::BTreeSet<_>>();
        (unique.len() == shortcuts.len())
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(
                    "Native menu shortcuts must be unique across File, View and Help.",
                )
            })
    }

    fn require_desktop_shortcuts(
        &self,
        target: tachyon_contracts::NativeTarget,
    ) -> Result<(), Failure> {
        if !Self::CTRL_TARGETS.contains(&target) {
            return Ok(());
        }
        let shortcuts = self
            .items()
            .filter_map(|item| item.shortcut.as_ref())
            .collect::<Vec<_>>();
        shortcuts
            .iter()
            .try_for_each(|shortcut| shortcut.require_desktop_available(target))?;
        let unique = shortcuts
            .iter()
            .map(|shortcut| shortcut.desktop_identity())
            .collect::<std::collections::BTreeSet<_>>();
        (unique.len() == shortcuts.len())
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(&format!(
                    "Native menu shortcuts must stay unique on {target:?}, where control is Ctrl like command; two shortcuts that differ only by control and command get the same key, so change one of them.",
                ))
            })
    }

    fn require_target(&self, target: tachyon_contracts::NativeTarget) -> Result<(), Failure> {
        (self.is_empty() || Self::DESKTOP_TARGETS.contains(&target))
            .then_some(())
            .ok_or_else(|| {
                super::config::config_failure(
                    "Declared native menus are supported only on macOS, Linux and Windows; iOS and Android have no menu bar.",
                )
            })
    }

    fn require_document(item: &NativeMenuEntry, index: &NativeRouteIndex) -> Result<(), Failure> {
        item.route
            .as_deref()
            .map_or(Ok(()), |route| Self::require_page(&item.id, route, index))
    }

    fn require_page(id: &str, route: &str, index: &NativeRouteIndex) -> Result<(), Failure> {
        let valid = index.routes.iter().any(|entry| entry.route == route)
            && !route.split('/').any(|segment| segment.starts_with('_'));
        valid.then_some(()).ok_or_else(|| super::config::config_failure(&format!(
            "Native menu '{id}' destination '{route}' must exactly identify a compiled static page route.",
        )))
    }
}

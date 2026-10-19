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
    #[serde(skip)]
    pub(super) tray: Vec<NativeMenuEntry>,
}

impl NativeMenuConfig {
    const MAX_ITEMS: usize = 64;

    pub(super) fn validate(&self) -> Result<(), Failure> {
        self.require_count()?;
        self.items().try_for_each(NativeMenuEntry::validate)?;
        self.require_ids()?;
        self.require_shortcuts()
    }

    const MOBILE_TARGETS: [tachyon_contracts::NativeTarget; 2] = [
        tachyon_contracts::NativeTarget::Ios,
        tachyon_contracts::NativeTarget::Android,
    ];

    const CTRL_TARGETS: [tachyon_contracts::NativeTarget; 3] = [
        tachyon_contracts::NativeTarget::Linux,
        tachyon_contracts::NativeTarget::Windows,
        tachyon_contracts::NativeTarget::Android,
    ];

    pub(super) fn validate_for(
        &self,
        target: tachyon_contracts::NativeTarget,
        index: &NativeRouteIndex,
    ) -> Result<(), Failure> {
        self.require_mobile_routes(target)?;
        self.require_desktop_shortcuts(target)?;
        self.items()
            .try_for_each(|item| Self::require_document(item, index))
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.groups().iter().all(|(_, entries)| entries.is_empty())
    }

    pub(super) fn groups(&self) -> [(&'static str, &[NativeMenuEntry]); 3] {
        [
            ("file", &self.file),
            ("view", &self.view),
            ("help", &self.help),
        ]
    }

    pub(super) fn table_groups(&self) -> [(&'static str, &[NativeMenuEntry]); 4] {
        let [file, view, help] = self.groups();
        [file, view, help, ("tray", &self.tray)]
    }

    pub(super) fn items(&self) -> impl Iterator<Item = &NativeMenuEntry> {
        self.file
            .iter()
            .chain(&self.view)
            .chain(&self.help)
            .chain(&self.tray)
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
                    "Native menu ids must be unique across File, View, Help and the tray.",
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

    fn require_mobile_routes(
        &self,
        target: tachyon_contracts::NativeTarget,
    ) -> Result<(), Failure> {
        let mobile = Self::MOBILE_TARGETS.contains(&target);
        let first_action = self.items().find(|item| item.action.is_some());
        first_action.filter(|_| mobile).map_or(Ok(()), |item| {
            Err(Self::mobile_action_failure(&item.id, target))
        })
    }

    fn mobile_action_failure(id: &str, target: tachyon_contracts::NativeTarget) -> Failure {
        super::config::config_failure(&format!(
            "Native menu '{id}' calls a service action, which {target:?} does not support yet: menu actions come with the application lifetime, which {target:?} does not have yet (LT1). Use a route item, or a link or button in the page.",
        ))
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

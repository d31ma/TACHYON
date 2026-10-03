mod path_change;
mod source_action;
mod source_watcher;

pub(crate) use source_action::SourceAction;
pub(crate) use source_watcher::SourceWatcher;

use notify::{Event, EventKind};
use path_change::PathChange;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceChanges {
    paths: BTreeSet<String>,
    force_reload: bool,
}

impl SourceChanges {
    const MAX_CHANGED_PATHS: usize = 4_096;

    pub(crate) fn new() -> Self {
        Self {
            paths: BTreeSet::new(),
            force_reload: false,
        }
    }

    pub(crate) fn record_event(&mut self, project_root: &Path, event: Event) {
        let paths = Self::relevant_paths(event);
        self.record_relevant(project_root, paths);
    }

    pub(crate) fn force_reload(&mut self) {
        self.force_reload = true;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.nothing_changed()
    }

    pub(crate) fn paths(&self) -> Vec<String> {
        self.paths.iter().cloned().collect()
    }

    pub(crate) fn action(&self) -> SourceAction {
        self.reload_action()
            .or_else(|| self.css_action())
            .unwrap_or_else(|| self.island_action())
    }

    fn nothing_changed(&self) -> bool {
        self.paths.is_empty() && !self.force_reload
    }

    fn reload_action(&self) -> Option<SourceAction> {
        let reload = self.force_reload || self.paths.is_empty();
        reload.then_some(SourceAction::Reload)
    }

    fn css_action(&self) -> Option<SourceAction> {
        let only_stylesheets = self.paths.iter().all(|path| Self::is_stylesheet(path));
        only_stylesheets.then_some(SourceAction::Css)
    }

    fn island_action(&self) -> SourceAction {
        let boundaries: Option<BTreeSet<String>> = self
            .paths
            .iter()
            .map(|path| Self::island_boundary(path))
            .collect();
        boundaries.map_or(SourceAction::Reload, |boundaries| SourceAction::Island {
            boundaries: boundaries.into_iter().collect(),
        })
    }

    fn record_relevant(&mut self, project_root: &Path, paths: Option<Vec<PathBuf>>) {
        if let Some(paths) = paths {
            self.record_paths(project_root, &paths);
        }
    }

    fn record_paths(&mut self, project_root: &Path, paths: &[PathBuf]) {
        self.force_reload |= paths.is_empty();
        for path in paths {
            self.record_path(project_root, path);
        }
    }

    fn record_path(&mut self, project_root: &Path, path: &Path) {
        let change = path
            .strip_prefix(project_root)
            .map_or(PathChange::Reload, |relative| {
                self.relative_change(relative)
            });
        self.apply(change);
    }

    fn relative_change(&self, relative: &Path) -> PathChange {
        if !Self::is_source_path(relative) {
            return PathChange::Ignore;
        }
        self.source_change(relative)
    }

    fn source_change(&self, relative: &Path) -> PathChange {
        Self::portable_path(relative)
            .filter(|_| self.paths.len() < Self::MAX_CHANGED_PATHS)
            .map_or(PathChange::Reload, PathChange::Record)
    }

    fn apply(&mut self, change: PathChange) {
        match change {
            PathChange::Ignore => {}
            PathChange::Record(portable) => {
                self.paths.insert(portable);
            }
            PathChange::Reload => self.force_reload = true,
        }
    }

    fn relevant_paths(event: Event) -> Option<Vec<PathBuf>> {
        let access = matches!(event.kind, EventKind::Access(_));
        (!access).then_some(event.paths)
    }

    fn is_source_path(path: &Path) -> bool {
        let mut components = path.components();
        let first = components.next().and_then(Self::normal_name);
        let single = components.next().is_none();
        first.is_some_and(|name| Self::is_source_name(name, single))
    }

    fn is_source_name(name: &OsStr, single: bool) -> bool {
        let checks = [
            Self::is_source_folder(name),
            Self::is_root_file(name, single),
        ];
        checks.contains(&true)
    }

    fn is_source_folder(name: &OsStr) -> bool {
        name == "client" || name == "server"
    }

    fn is_root_file(name: &OsStr, single: bool) -> bool {
        single && name.to_str().is_some_and(Self::is_root_configuration)
    }

    fn is_root_configuration(name: &str) -> bool {
        name == ".tachyonrc" || name.starts_with("middleware.")
    }

    fn normal_name(component: Component<'_>) -> Option<&OsStr> {
        match component {
            Component::Normal(name) => Some(name),
            _ => None,
        }
    }

    fn portable_path(path: &Path) -> Option<String> {
        let parts: Option<Vec<&str>> = path
            .components()
            .map(|component| Self::normal_name(component).and_then(OsStr::to_str))
            .collect();
        Some(parts?.join("/"))
    }

    fn is_stylesheet(path: &str) -> bool {
        path.starts_with("client/")
            && Path::new(path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("css"))
    }

    fn island_boundary(path: &str) -> Option<String> {
        let file = path.strip_prefix("client/services/")?;
        let (stem, extension) = file.rsplit_once('.')?;
        let checks = [
            Self::is_script(extension),
            stem.ends_with("Service"),
            !file.contains('/'),
        ];
        checks
            .iter()
            .all(|passed| *passed)
            .then(|| format!("service:{stem}"))
    }

    fn is_script(extension: &str) -> bool {
        matches!(extension, "js" | "ts")
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceAction, SourceChanges};
    use notify::{Event, EventKind};
    use std::path::Path;

    fn action(paths: &[&str]) -> SourceAction {
        changes(paths).action()
    }

    fn changes(paths: &[&str]) -> SourceChanges {
        let root = Path::new("/project");
        let mut changes = SourceChanges::new();
        changes.record_event(
            root,
            Event {
                kind: EventKind::Any,
                paths: paths.iter().map(|path| root.join(path)).collect(),
                attrs: notify::event::EventAttributes::new(),
            },
        );
        changes
    }

    #[test]
    fn stylesheets_are_the_only_css_hot_update_boundary() {
        assert_eq!(
            action(&["client/components/card/tac.css"]),
            SourceAction::Css
        );
        assert_eq!(
            action(&["client/components/card/tac.css", "client/shared/theme.css"]),
            SourceAction::Css
        );
        assert_eq!(
            action(&["client/components/card/tac.css", "client/pages/tac.html"]),
            SourceAction::Reload
        );
    }

    #[test]
    fn browser_service_modules_map_to_service_boundaries() {
        assert_eq!(
            action(&[
                "client/services/UserListService.ts",
                "client/services/BasketService.js"
            ]),
            SourceAction::Island {
                boundaries: vec![
                    String::from("service:BasketService"),
                    String::from("service:UserListService")
                ]
            }
        );
        assert_eq!(
            action(&["client/components/product/card/tac.html"]),
            SourceAction::Reload
        );
        assert_eq!(
            action(&[
                "client/services/BasketService.js",
                "client/components/product/card/tac.html"
            ]),
            SourceAction::Reload
        );
    }

    #[test]
    fn only_browser_services_receive_hot_updates() {
        for path in [
            "client/services/BasketService.swift",
            "client/services/BasketService.kt",
            "client/services/BasketService.cs",
            "client/services/BasketService.rs",
            "client/services/basket.js",
            "client/services/Basket.js",
            "client/services/nested/DeepService.js",
            "client/components/card/tac.js",
            "client/components/card/tac.ts",
            "client/components/card/tachyon-island.js",
            "client/components/card/tac.rs",
            "client/components/card/tac.swift",
            "client/pages/tac.js",
            "client/pages/tac.rs",
            "client/stores/BasketStore.js",
            "client/helpers/PricingHelper.js",
            "client/remotes/ProductsClient.js",
            "client/entities/ProductEntity.js",
            "client/setups/ApiSetup.js",
            "server/services/OrdersService.js",
            "server/helpers/AuditHelper.js",
            "server/controllers/OrdersController.js",
        ] {
            let changed = changes(&[path]);
            assert!(!changed.is_empty(), "{path}");
            assert_eq!(changed.action(), SourceAction::Reload, "{path}");
        }
    }

    #[test]
    fn every_supported_root_configuration_invalidates_the_dev_build() {
        for name in [".tachyonrc", "client/manifest.json"] {
            let changed = changes(&[name]);
            assert_eq!(changed.paths(), vec![String::from(name)]);
            assert_eq!(changed.action(), SourceAction::Reload);
        }
    }

    #[test]
    fn generated_output_and_dependency_events_are_ignored() {
        let changes = changes(&[
            "dist/index.html",
            "target/debug/ty",
            "node_modules/x.js",
            ".env",
            "application-production.json",
        ]);
        assert!(changes.is_empty());
    }
}

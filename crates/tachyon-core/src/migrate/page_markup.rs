use super::page_component::PageComponent;
use super::{MigrationFinding, MigrationStatus, portable_path, view_sources};
use crate::compiler::PageUpgrade;
use crate::template::ComponentRegistry;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct PageMarkup;

impl PageMarkup {
    pub(super) fn findings(root: &Path) -> Vec<MigrationFinding> {
        let names = Self::component_names(root);
        view_sources(&root.join(PageComponent::PAGES))
            .iter()
            .filter_map(|path| Self::finding(root, path, &names))
            .collect()
    }

    pub(super) fn refused(source: &str, relative: &str, names: &BTreeSet<String>) -> bool {
        !PageUpgrade::markup_diagnostics(source, relative, names).is_empty()
    }

    pub(super) fn component_names(root: &Path) -> BTreeSet<String> {
        ComponentRegistry::discover(root)
            .map_or_else(|_| BTreeSet::new(), |registry| registry.names())
    }

    fn finding(root: &Path, path: &Path, names: &BTreeSet<String>) -> Option<MigrationFinding> {
        let relative = portable_path(path.strip_prefix(root).unwrap_or(path));
        let source = fs::read_to_string(path).ok()?;
        let diagnostics = PageUpgrade::markup_diagnostics(&source, &relative, names);
        let first = diagnostics.first()?;
        Some(Self::refusal(&relative, first))
    }

    fn refusal(relative: &str, diagnostic: &Diagnostic) -> MigrationFinding {
        let folder = relative.rsplit_once('/').map_or("", |(folder, _)| folder);
        let action = PageComponent::of(folder).map_or_else(
            || String::from("Move the markup into a component under client/components/<name>/ and place <name /> in the page (ADR 0029)."),
            |component| format!(
                "Run 'ty migrate fix --write' to move the page's markup into {} and place <{} /> in the page (ADR 0029).",
                component.view(),
                component.tag()
            ),
        );
        MigrationFinding::new(
            relative,
            "view.page_markup",
            MigrationStatus::Changed,
            &format!(
                "ty bundle refuses this page (TY1309): {}",
                diagnostic.message
            ),
            Some(&action),
        )
    }
}

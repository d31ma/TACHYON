use super::super::page_component::PageComponent;
use super::super::page_markup::PageMarkup;
use super::{ManualStep, MigrationFix};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) struct PageSources {
    pub(super) component: PageComponent,
    pub(super) view: String,
    pub(super) source: String,
    pub(super) refused: bool,
    pub(super) style: Option<String>,
    pub(super) script: bool,
}

impl PageSources {
    pub(super) const STYLE: &'static str = "tac.css";
    pub(super) const SCRIPT: &'static str = "tac.js";

    pub(super) fn read(
        root: &Path,
        view: &str,
        names: &BTreeSet<String>,
    ) -> Result<Option<Self>, ManualStep> {
        let folder = view.rsplit_once('/').map_or("", |(folder, _)| folder);
        let source = fs::read_to_string(root.join(view)).unwrap_or_default();
        let refused = PageMarkup::refused(&source, view, names);
        let style = Self::small_text(root, &format!("{folder}/{}", Self::STYLE));
        let script = Self::small_text(root, &format!("{folder}/{}", Self::SCRIPT)).is_some();
        let needed = [refused, style.is_some(), script].contains(&true);
        let component = PageComponent::of(folder);
        let unnamed = [needed, component.is_none()] == [true, true];
        (!unnamed)
            .then_some(())
            .ok_or_else(|| Self::unnamed_step(view))?;
        Ok(component.filter(|_| needed).map(|component| Self {
            component,
            view: String::from(view),
            source,
            refused,
            style,
            script,
        }))
    }

    fn small_text(root: &Path, relative: &str) -> Option<String> {
        let path = root.join(relative);
        let small = MigrationFix::is_small_file(&path).unwrap_or(false);
        small.then(|| fs::read_to_string(path).ok()).flatten()
    }

    fn unnamed_step(view: &str) -> ManualStep {
        ManualStep::new(
            view,
            String::from(
                "A page only composes components (TY1309), and this page's folder names are not component tag names, so the fix cannot name a component for it.",
            ),
            String::from(
                "Move the page's markup and companions into a component under client/components/<name>/ by hand, and place <name /> in the page.",
            ),
        )
    }
}

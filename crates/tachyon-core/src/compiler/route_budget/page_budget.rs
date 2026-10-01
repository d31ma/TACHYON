use super::{BudgetLimit, LoadedFiles, RouteBudgetSettings, RouteSize};
use crate::Failure;
use crate::tac_python::{RuntimeInventory, RuntimeProfile};
use std::collections::{BTreeMap, BTreeSet};
use tachyon_diagnostics::Diagnostic;

pub(super) struct PageBudget<'a> {
    settings: RouteBudgetSettings,
    profile: Option<RuntimeProfile>,
    sent_bytes: &'a dyn Fn(&str) -> Option<u64>,
}

impl<'a> PageBudget<'a> {
    pub(super) fn new(
        settings: RouteBudgetSettings,
        profile: Option<RuntimeProfile>,
        sent_bytes: &'a dyn Fn(&str) -> Option<u64>,
    ) -> Self {
        Self {
            settings,
            profile,
            sent_bytes,
        }
    }

    pub(super) fn diagnostics(
        &self,
        page: &str,
        files: &BTreeMap<String, &[u8]>,
        sizes: &BTreeMap<String, u64>,
    ) -> Result<(Vec<Diagnostic>, Vec<String>), Failure> {
        let mut loaded = LoadedFiles::of(page, files);
        let inventory = RuntimeInventory::reachable(&loaded, files, self.sent_bytes)?;
        self.require_profile(&inventory)?;
        loaded.extend(inventory.iter().map(|(name, _)| name.clone()));
        let settings = self.settings.for_python_route(!inventory.is_empty());
        let route = Self::route(page);
        let diagnostics = Self::over_budget(&route, &loaded, sizes, settings);
        let notes = Self::python_notes(&route, &inventory);
        Ok((diagnostics, notes))
    }

    fn over_budget(
        route: &str,
        loaded: &BTreeSet<String>,
        sizes: &BTreeMap<String, u64>,
        settings: RouteBudgetSettings,
    ) -> Vec<Diagnostic> {
        [settings.script, settings.web_assembly]
            .into_iter()
            .map(|limit| RouteSize::new(route.to_owned(), limit, Self::sizes(loaded, sizes, limit)))
            .filter(RouteSize::is_over)
            .map(|size| size.diagnostic())
            .collect()
    }

    fn require_profile(&self, inventory: &[(String, u64)]) -> Result<(), Failure> {
        if self.profile.is_none() && !inventory.is_empty() {
            return Err(RuntimeInventory::profile_missing());
        }
        Ok(())
    }

    fn python_notes(route: &str, inventory: &[(String, u64)]) -> Vec<String> {
        if inventory.is_empty() {
            return Vec::new();
        }
        Self::python_note(route, inventory)
    }

    fn python_note(route: &str, inventory: &[(String, u64)]) -> Vec<String> {
        let total: u64 = inventory.iter().map(|(_, bytes)| bytes).sum();
        let auxiliary: u64 = inventory
            .iter()
            .filter(|(name, _)| Self::auxiliary(name))
            .map(|(_, bytes)| bytes)
            .sum();
        vec![format!(
            "Route '{route}' pinned Python runtime transfer: {total} bytes across all five dependencies, including {auxiliary} auxiliary ZIP/lock bytes. Script and WebAssembly limits retain their own categories."
        )]
    }

    fn auxiliary(name: &str) -> bool {
        name.ends_with(".zip") || name.ends_with(".json")
    }

    fn sizes(
        loaded: &BTreeSet<String>,
        sizes: &BTreeMap<String, u64>,
        limit: BudgetLimit,
    ) -> Vec<(String, u64)> {
        loaded
            .iter()
            .filter(|path| Self::within_category(path, limit))
            .map(|path| (path.clone(), sizes.get(path).copied().unwrap_or_default()))
            .collect()
    }

    fn within_category(path: &str, limit: BudgetLimit) -> bool {
        limit
            .extensions
            .iter()
            .any(|extension| path.ends_with(extension))
    }

    fn route(page: &str) -> String {
        let folder = page.trim_end_matches("index.html").trim_end_matches('/');
        format!("/{folder}")
    }
}

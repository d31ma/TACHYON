use super::{BudgetLimit, LoadedFiles, RouteBudgetSettings, RouteRuntimes, RouteSize};
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
        let without_runtimes = self.settings.for_python_route(!inventory.is_empty());
        let runtimes = RouteRuntimes::of(&loaded);
        let settings = without_runtimes.for_runtimes(&runtimes);
        let route = Self::route(page);
        let basis = Self::basis(without_runtimes, settings, &runtimes);
        let script = Self::route_size(&route, &loaded, sizes, settings.script).explained(basis);
        let web_assembly = Self::route_size(&route, &loaded, sizes, settings.web_assembly);
        let diagnostics = Self::over_budget([script, web_assembly]);
        let notes = Self::python_notes(&route, &inventory);
        Ok((diagnostics, notes))
    }

    fn route_size(
        route: &str,
        loaded: &BTreeSet<String>,
        sizes: &BTreeMap<String, u64>,
        limit: BudgetLimit,
    ) -> RouteSize {
        RouteSize::new(route.to_owned(), limit, Self::sizes(loaded, sizes, limit))
    }

    fn over_budget(route_sizes: [RouteSize; 2]) -> Vec<Diagnostic> {
        route_sizes
            .into_iter()
            .filter(RouteSize::is_over)
            .map(|size| size.diagnostic())
            .collect()
    }

    fn basis(
        without_runtimes: RouteBudgetSettings,
        settings: RouteBudgetSettings,
        runtimes: &RouteRuntimes,
    ) -> String {
        if settings.script == without_runtimes.script {
            return String::new();
        }
        runtimes.basis(RouteBudgetSettings::SCRIPT.bytes)
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
        std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| extension == "zip" || extension == "json")
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

use super::{BudgetLimit, LoadedFiles, RouteBudgetSettings, RouteSize};
use crate::Failure;
use crate::compiler::PortablePath;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

pub(crate) struct BudgetCheck;

impl BudgetCheck {
    const PAGE_FILE: &'static str = "index.html";
    const SEPARATOR: char = '/';

    #[cfg(test)]
    pub(crate) fn verify(
        project_root: &Path,
        files: &[(PathBuf, Vec<u8>)],
        release: bool,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<(), Failure> {
        Self::check(project_root, files, release, sent_bytes).map(|_| ())
    }

    pub(crate) fn check(
        project_root: &Path,
        files: &[(PathBuf, Vec<u8>)],
        release: bool,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<Vec<String>, Failure> {
        let profile = crate::tac_python::RuntimeProfile::for_project(project_root)?;
        let settings = RouteBudgetSettings::read(project_root);
        let checked = Self::checked(files, settings.check & release);
        let (diagnostics, notes) = Self::diagnostics(checked, settings, profile, sent_bytes)?;
        Self::result(diagnostics)?;
        Ok(notes)
    }

    pub(crate) fn has_python_roots(files: &[(PathBuf, Vec<u8>)]) -> bool {
        let by_path: BTreeMap<String, &[u8]> = files
            .iter()
            .map(|(path, bytes)| (PortablePath::of(path), bytes.as_slice()))
            .collect();
        by_path
            .keys()
            .filter(|path| Self::is_page(path))
            .any(|page| {
                LoadedFiles::of(page, &by_path)
                    .contains(crate::tac_python::RuntimeInventory::ENGINE)
            })
    }

    fn checked(files: &[(PathBuf, Vec<u8>)], check: bool) -> &[(PathBuf, Vec<u8>)] {
        if check { files } else { &[] }
    }

    fn diagnostics(
        files: &[(PathBuf, Vec<u8>)],
        settings: RouteBudgetSettings,
        profile: Option<crate::tac_python::RuntimeProfile>,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<(Vec<Diagnostic>, Vec<String>), Failure> {
        let by_path: BTreeMap<String, &[u8]> = files
            .iter()
            .map(|(path, bytes)| (PortablePath::of(path), bytes.as_slice()))
            .collect();
        let sizes: BTreeMap<String, u64> = by_path
            .iter()
            .map(|(path, bytes)| (path.clone(), Self::size(path, bytes, sent_bytes)))
            .collect();
        let mut diagnostics = Vec::new();
        let mut notes = Vec::new();
        for page in by_path.keys().filter(|path| Self::is_page(path)) {
            let (page_diagnostics, page_notes) =
                Self::page_diagnostics(page, &by_path, &sizes, settings, profile, sent_bytes)?;
            diagnostics.extend(page_diagnostics);
            notes.extend(page_notes);
        }
        Ok((diagnostics, notes))
    }

    fn page_diagnostics(
        page: &str,
        files: &BTreeMap<String, &[u8]>,
        sizes: &BTreeMap<String, u64>,
        settings: RouteBudgetSettings,
        profile: Option<crate::tac_python::RuntimeProfile>,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<(Vec<Diagnostic>, Vec<String>), Failure> {
        let mut loaded = LoadedFiles::of(page, files);
        let inventory = crate::tac_python::RuntimeInventory::reachable(&loaded, files, sent_bytes)?;
        Self::require_python_profile(profile, &inventory)?;
        loaded.extend(inventory.iter().map(|(name, _)| name.clone()));
        let settings = settings.for_python_route(!inventory.is_empty());
        let route = Self::route(page);
        let diagnostics = [settings.script, settings.web_assembly]
            .into_iter()
            .map(|limit| RouteSize::new(route.clone(), limit, Self::sizes(&loaded, sizes, limit)))
            .filter(RouteSize::is_over)
            .map(|size| size.diagnostic())
            .collect();
        let notes = Self::python_notes(&route, &inventory);
        Ok((diagnostics, notes))
    }

    fn require_python_profile(
        profile: Option<crate::tac_python::RuntimeProfile>,
        inventory: &[(String, u64)],
    ) -> Result<(), Failure> {
        if [profile.is_none(), !inventory.is_empty()].contains(&false) {
            return Ok(());
        }
        Err(crate::tac_python::RuntimeInventory::profile_missing())
    }

    fn python_notes(route: &str, inventory: &[(String, u64)]) -> Vec<String> {
        if inventory.is_empty() {
            return Vec::new();
        }
        let total: u64 = inventory.iter().map(|(_, bytes)| bytes).sum();
        let auxiliary: u64 = inventory
            .iter()
            .filter(|(name, _)| name.ends_with(".zip") || name.ends_with(".json"))
            .map(|(_, bytes)| bytes)
            .sum();
        vec![format!(
            "Route '{route}' pinned Python runtime transfer: {total} bytes across all five dependencies, including {auxiliary} auxiliary ZIP/lock bytes. Script and WebAssembly limits retain their own categories."
        )]
    }

    fn sizes(
        loaded: &BTreeSet<String>,
        sizes: &BTreeMap<String, u64>,
        limit: BudgetLimit,
    ) -> Vec<(String, u64)> {
        loaded
            .iter()
            .filter(|path| {
                limit
                    .extensions
                    .iter()
                    .any(|extension| path.ends_with(extension))
            })
            .map(|path| (path.clone(), sizes.get(path).copied().unwrap_or_default()))
            .collect()
    }

    fn size(path: &str, bytes: &[u8], sent_bytes: &dyn Fn(&str) -> Option<u64>) -> u64 {
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        sent_bytes(path).unwrap_or(length)
    }

    fn is_page(path: &str) -> bool {
        let file = path.rsplit(Self::SEPARATOR).next().unwrap_or_default();
        file == Self::PAGE_FILE
    }

    fn route(page: &str) -> String {
        let folder = page
            .trim_end_matches(Self::PAGE_FILE)
            .trim_end_matches(Self::SEPARATOR);
        format!("/{folder}")
    }

    fn result(diagnostics: Vec<Diagnostic>) -> Result<(), Failure> {
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(Failure::new(diagnostics))
        }
    }
}

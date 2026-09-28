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

    pub(crate) fn verify(
        project_root: &Path,
        files: &[(PathBuf, Vec<u8>)],
        release: bool,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<(), Failure> {
        let settings = RouteBudgetSettings::read(project_root);
        let checked = Self::checked(files, settings.check & release);
        Self::result(Self::diagnostics(checked, settings, sent_bytes))
    }

    fn checked(files: &[(PathBuf, Vec<u8>)], check: bool) -> &[(PathBuf, Vec<u8>)] {
        if check { files } else { &[] }
    }

    fn diagnostics(
        files: &[(PathBuf, Vec<u8>)],
        settings: RouteBudgetSettings,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Vec<Diagnostic> {
        let by_path: BTreeMap<String, &[u8]> = files
            .iter()
            .map(|(path, bytes)| (PortablePath::of(path), bytes.as_slice()))
            .collect();
        let sizes: BTreeMap<String, u64> = by_path
            .iter()
            .map(|(path, bytes)| (path.clone(), Self::size(path, bytes, sent_bytes)))
            .collect();
        by_path
            .keys()
            .filter(|path| Self::is_page(path))
            .flat_map(|page| Self::page_diagnostics(page, &by_path, &sizes, settings))
            .collect()
    }

    fn page_diagnostics(
        page: &str,
        files: &BTreeMap<String, &[u8]>,
        sizes: &BTreeMap<String, u64>,
        settings: RouteBudgetSettings,
    ) -> Vec<Diagnostic> {
        let loaded = LoadedFiles::of(page, files);
        let route = Self::route(page);
        [settings.script, settings.web_assembly]
            .into_iter()
            .map(|limit| RouteSize::new(route.clone(), limit, Self::sizes(&loaded, sizes, limit)))
            .filter(RouteSize::is_over)
            .map(|size| size.diagnostic())
            .collect()
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

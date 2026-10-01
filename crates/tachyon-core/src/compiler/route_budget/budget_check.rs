use super::{LoadedFiles, PageBudget, RouteBudgetSettings};
use crate::Failure;
use crate::compiler::PortablePath;
use std::collections::BTreeMap;
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
        Self::python_roots(&by_path)
    }

    fn python_roots(files: &BTreeMap<String, &[u8]>) -> bool {
        files
            .keys()
            .filter(|path| Self::is_page(path))
            .any(|page| Self::page_loads_python(page, files))
    }

    fn page_loads_python(page: &str, files: &BTreeMap<String, &[u8]>) -> bool {
        LoadedFiles::of(page, files).contains(crate::tac_python::RuntimeInventory::ENGINE)
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
        let budget = PageBudget::new(settings, profile, sent_bytes);
        let mut diagnostics = Vec::new();
        let mut notes = Vec::new();
        for page in by_path.keys().filter(|path| Self::is_page(path)) {
            let (page_diagnostics, page_notes) = budget.diagnostics(page, &by_path, &sizes)?;
            diagnostics.extend(page_diagnostics);
            notes.extend(page_notes);
        }
        Ok((diagnostics, notes))
    }

    fn size(path: &str, bytes: &[u8], sent_bytes: &dyn Fn(&str) -> Option<u64>) -> u64 {
        let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        sent_bytes(path).unwrap_or(length)
    }

    fn is_page(path: &str) -> bool {
        let file = path.rsplit(Self::SEPARATOR).next().unwrap_or_default();
        file == Self::PAGE_FILE
    }

    fn result(diagnostics: Vec<Diagnostic>) -> Result<(), Failure> {
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(Failure::new(diagnostics))
        }
    }
}

#![allow(clippy::needless_bitwise_bool)]

use super::super::{Failure, Input};
use super::Tokens;
use crate::migrate::{MigrationFinding, MigrationStatus};
use body::BodyScan;
use braced::BracedMembers;
use ended::EndedMembers;
use indented::IndentedMembers;
use layout::Layout;
pub(super) use method::Method;
use scanned::Scanned;
pub(super) use spelling::Spelling;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;
use verdict::Verdict;
pub(super) use words::Words;

mod body;
mod braced;
mod container;
mod ended;
mod finding;
mod head;
mod indented;
mod layout;
mod method;
mod scanned;
mod spelling;
mod utility_shape;
mod verdict;
mod word;
mod words;

#[cfg(test)]
mod tests;

use super::rule_switch::RuleSwitch;
pub(super) use utility_shape::RustConstants;
pub(crate) use utility_shape::UtilityShape;

pub(crate) struct CohesionCheck;

impl CohesionCheck {
    const FOLDERS: &'static [&'static str] = &[
        "client/services",
        "server/services",
        "server/controllers",
        "client/shared/utils",
        "server/utils",
    ];
    const FEATURE: &'static str = "cohesion.method";

    pub(crate) fn check_file(
        path: &Path,
        bytes: &[u8],
        settings: RuleSwitch,
    ) -> Result<(), Failure> {
        let text = std::str::from_utf8(bytes).unwrap_or_default();
        crate::fsharp_source::FsharpView::either(path, text, |path, text| {
            Self::check_text(path, text.as_bytes(), settings)
        })
    }

    fn check_text(path: &Path, bytes: &[u8], settings: RuleSwitch) -> Result<(), Failure> {
        let source = Self::checked_source(path, bytes, settings);
        let problems = Self::problems_in(path, source)?;
        Self::refuse(problems)
    }

    pub(crate) fn problems(path: &Path, source: &str) -> Result<Vec<Diagnostic>, Failure> {
        let view = crate::fsharp_source::FsharpView::of(path, source);
        view.map_or_else(
            || Self::language_problems(path, source),
            |view| Self::fsharp_problems(&view),
        )
    }

    fn fsharp_problems(
        view: &crate::fsharp_source::FsharpView,
    ) -> Result<Vec<Diagnostic>, Failure> {
        let problems = Self::language_problems(view.path(), view.text())?;
        Ok(problems
            .iter()
            .map(|problem| view.diagnostic(problem))
            .collect())
    }

    fn language_problems(path: &Path, source: &str) -> Result<Vec<Diagnostic>, Failure> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Self::problems_with(path, source, Spelling::of(extension))
    }

    pub(crate) fn migration_findings(root: &Path) -> Vec<MigrationFinding> {
        Self::FOLDERS
            .iter()
            .flat_map(|folder| Self::folder_findings(root, folder))
            .collect()
    }

    fn checked_source<'s>(path: &Path, bytes: &'s [u8], settings: RuleSwitch) -> Option<&'s str> {
        let wanted = settings.covers(path) & Self::in_scope(path);
        wanted.then(|| std::str::from_utf8(bytes).ok()).flatten()
    }

    fn in_scope(path: &Path) -> bool {
        let folder = path
            .parent()
            .map(Path::to_string_lossy)
            .unwrap_or_default()
            .replace('\\', "/");
        Self::FOLDERS.contains(&folder.as_str())
    }

    fn problems_in(path: &Path, source: Option<&str>) -> Result<Vec<Diagnostic>, Failure> {
        let Some(source) = source else {
            return Ok(Vec::new());
        };
        Self::problems(path, source)
    }

    fn refuse(problems: Vec<Diagnostic>) -> Result<(), Failure> {
        if problems.is_empty() {
            return Ok(());
        }
        Err(Failure::new(problems))
    }

    fn problems_with(
        path: &Path,
        source: &str,
        spelling: Option<Spelling>,
    ) -> Result<Vec<Diagnostic>, Failure> {
        let Some(spelling) = spelling else {
            return Ok(Vec::new());
        };
        Self::scan(path, source, &spelling)
    }

    fn scan(path: &Path, source: &str, spelling: &Spelling) -> Result<Vec<Diagnostic>, Failure> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input)?;
        tokens.check_complete(&input)?;
        let words = Words::read(&tokens, source);
        let body_scan = BodyScan::new(&words, spelling);
        let scanned = Self::methods(&words, spelling)
            .into_iter()
            .map(|method| Scanned::new(method.clone(), body_scan.findings(&method)))
            .collect();
        let portable = path.to_string_lossy().replace('\\', "/");
        Ok(Verdict::new(&portable, &words, spelling, scanned).diagnostics())
    }

    pub(super) fn methods(words: &Words<'_>, spelling: &Spelling) -> Vec<Method> {
        match spelling.layout() {
            Layout::Braced => BracedMembers::read(words, spelling),
            Layout::Indented => IndentedMembers::read(words),
            Layout::Ended => EndedMembers::read(words),
        }
    }

    fn folder_findings(root: &Path, folder: &str) -> Vec<MigrationFinding> {
        Self::file_names(&root.join(folder))
            .iter()
            .flat_map(|name| Self::file_findings(root, &format!("{folder}/{name}")))
            .collect()
    }

    fn file_names(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn file_findings(root: &Path, relative: &str) -> Vec<MigrationFinding> {
        let source = std::fs::read_to_string(root.join(relative)).unwrap_or_default();
        Self::problems(Path::new(relative), &source)
            .unwrap_or_default()
            .iter()
            .map(|problem| Self::finding(relative, &source, problem))
            .collect()
    }

    fn finding(relative: &str, source: &str, problem: &Diagnostic) -> MigrationFinding {
        let start = problem.spans.first().map_or(0, |span| span.start);
        let before = source
            .get(..usize::try_from(start).unwrap_or(0))
            .unwrap_or("");
        let line = before.matches('\n').count() + 1;
        MigrationFinding {
            source: relative.to_owned(),
            feature: Self::FEATURE.to_owned(),
            status: MigrationStatus::Changed,
            detail: format!("Line {line}: {}", problem.message),
            action: problem.help.clone(),
        }
    }
}

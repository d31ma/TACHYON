#![allow(clippy::needless_bitwise_bool)]

use super::super::Input;
use super::Tokens;
use super::rule_switch::RuleSwitch;
use crate::migrate::MigrationFinding;
use code_reader::CodeReader;
use duplicate_report::DuplicateReport;
pub(super) use duplication_scope::DuplicationScope;
pub(super) use import_words::ImportWords;
pub(super) use project_files::ProjectFiles;
pub(super) use project_source::ProjectSource;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;
use tachyon_duplication::{Duplicate, DuplicateFinder, SourceFile};

mod code_reader;
mod duplicate_report;
mod duplication_scope;
mod import_words;
mod project_files;
mod project_source;
mod rust_declarations;
mod utility_name;

#[cfg(test)]
mod tests;

pub(crate) struct DuplicationCheck;

impl DuplicationCheck {
    pub(crate) fn diagnostics<'p>(
        files: impl Iterator<Item = (&'p Path, &'p [u8])>,
        switch: RuleSwitch,
    ) -> Vec<Diagnostic> {
        let sources = ProjectSource::switched_on(files, switch);
        Self::duplicates(&sources)
            .iter()
            .map(|duplicate| DuplicateReport::new(duplicate, &sources).diagnostic())
            .collect()
    }

    pub(crate) fn migration_findings(root: &Path) -> Vec<MigrationFinding> {
        let sources = ProjectFiles::read(root);
        Self::duplicates(&sources)
            .iter()
            .map(|duplicate| DuplicateReport::new(duplicate, &sources).finding())
            .collect()
    }

    fn duplicates(sources: &[ProjectSource]) -> Vec<Duplicate> {
        let files: Vec<SourceFile> = sources
            .iter()
            .filter(|source| DuplicationScope::covers(Path::new(&source.path)))
            .filter_map(CodeReader::read)
            .collect();
        DuplicateFinder::find(&files)
    }
}

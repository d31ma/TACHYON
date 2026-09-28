use super::CaptureProblem;
use crate::compiler::PortablePath;
use cap_std::fs::{Dir, Metadata};
use std::io::ErrorKind;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct EntryMetadata;

impl EntryMetadata {
    pub(super) fn existing(
        directory: &Dir,
        name: &Path,
        shown_path: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Metadata> {
        directory
            .symlink_metadata(name)
            .map_err(|error| Self::report_unless_missing(&error, shown_path, diagnostics))
            .ok()
    }

    fn report_unless_missing(
        error: &std::io::Error,
        shown_path: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if error.kind() == ErrorKind::NotFound {
            return;
        }
        diagnostics.push(CaptureProblem::error(format!(
            "Cannot inspect '{}': {error}",
            PortablePath::of(shown_path)
        )));
    }
}

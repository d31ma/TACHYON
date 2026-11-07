use crate::compiler::PortablePath;
use crate::failure::{diagnostic, source_span};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct CaptureProblem;

impl CaptureProblem {
    pub(super) fn error(message: String) -> Diagnostic {
        diagnostic(1001, message, None, None)
    }

    pub(super) fn check(
        problem: Option<Diagnostic>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<()> {
        problem.map_or(Some(()), |problem| {
            diagnostics.push(problem);
            None
        })
    }

    pub(super) fn unsafe_symlink(relative: &Path) -> Diagnostic {
        let path = PortablePath::of(relative);
        diagnostic(
            1004,
            format!("Symlinked project source '{path}' is not allowed."),
            Some(String::from(
                "Replace the symlink with a regular file or directory.",
            )),
            source_span(&path, 0, path.len()),
        )
    }

    pub(super) fn oversized_source(relative: &Path) -> Diagnostic {
        Self::error(format!(
            "Source '{}' exceeds the 16 MiB snapshot limit.",
            PortablePath::of(relative)
        ))
    }

    pub(super) fn unreadable_directory(relative: &Path, error: &std::io::Error) -> Diagnostic {
        Self::error(format!(
            "Cannot read source directory '{}': {error}",
            PortablePath::of(relative)
        ))
    }
}

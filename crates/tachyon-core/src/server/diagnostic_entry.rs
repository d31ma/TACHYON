use super::SourceLocation;
use crate::Failure;
use serde::Serialize;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(in crate::server) struct DiagnosticEntry {
    code: String,
    message: String,
    locations: Vec<String>,
    help: Option<String>,
}

impl DiagnosticEntry {
    pub(in crate::server) fn list(project_root: &Path, failure: &Failure) -> Vec<Self> {
        failure
            .diagnostics()
            .iter()
            .map(|diagnostic| Self::from_diagnostic(project_root, diagnostic))
            .collect()
    }

    pub(in crate::server) fn text(entries: &[Self]) -> String {
        entries
            .iter()
            .map(Self::entry_text)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn from_diagnostic(project_root: &Path, diagnostic: &Diagnostic) -> Self {
        Self {
            code: diagnostic.code.to_string(),
            message: diagnostic.message.clone(),
            locations: diagnostic
                .spans
                .iter()
                .map(|span| SourceLocation::describe(project_root, span))
                .collect(),
            help: diagnostic.help.clone(),
        }
    }

    fn entry_text(&self) -> String {
        let mut lines = vec![format!("{}: {}", self.code, self.message)];
        lines.extend(
            self.locations
                .iter()
                .map(|location| format!("  at {location}")),
        );
        lines.extend(self.help.iter().map(|help| format!("  help: {help}")));
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::DiagnosticEntry;
    use crate::Failure;
    use crate::failure::{diagnostic, source_span};

    #[test]
    fn entries_carry_the_message_the_line_and_the_help() {
        let project = tempfile::tempdir().expect("project");
        std::fs::write(project.path().join("tac.html"), "<main>\n<p>\n").expect("source");
        let failure = Failure::one(diagnostic(
            1002,
            "The paragraph is not closed.",
            Some(String::from("Close it with </p>.")),
            source_span("tac.html", 7, 10),
        ));
        let entries = DiagnosticEntry::list(project.path(), &failure);
        assert_eq!(
            DiagnosticEntry::text(&entries),
            "TY1002: The paragraph is not closed.\n  at tac.html:2\n  help: Close it with </p>."
        );
    }
}

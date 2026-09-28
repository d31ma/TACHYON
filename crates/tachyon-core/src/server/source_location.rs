use std::path::Path;
use tachyon_diagnostics::SourceSpan;

pub(in crate::server) struct SourceLocation;

impl SourceLocation {
    pub(in crate::server) fn describe(project_root: &Path, span: &SourceSpan) -> String {
        Self::line_number(project_root, span)
            .map_or_else(|| span.file.clone(), |line| format!("{}:{line}", span.file))
    }

    fn line_number(project_root: &Path, span: &SourceSpan) -> Option<usize> {
        let source = std::fs::read(project_root.join(&span.file)).ok()?;
        let start = usize::try_from(span.start).ok()?.min(source.len());
        Some(source[..start].split(|byte| *byte == b'\n').count())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::SourceLocation;
    use tachyon_diagnostics::SourceSpan;

    #[test]
    fn a_span_is_shown_as_file_and_line() {
        let project = tempfile::tempdir().expect("project");
        std::fs::write(project.path().join("page.html"), "one\ntwo\nthree\n").expect("source");
        let span = SourceSpan::new("page.html", 8, 13).expect("span");
        assert_eq!(
            SourceLocation::describe(project.path(), &span),
            "page.html:3"
        );
    }

    #[test]
    fn a_missing_file_is_shown_by_name() {
        let project = tempfile::tempdir().expect("project");
        let span = SourceSpan::new("gone.html", 4, 5).expect("span");
        assert_eq!(SourceLocation::describe(project.path(), &span), "gone.html");
    }
}

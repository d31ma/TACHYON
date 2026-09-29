use super::{FsharpShadow, FsharpSource};
use crate::Failure;
use std::path::{Path, PathBuf};
use tachyon_diagnostics::{Diagnostic, SourceSpan};

pub(crate) struct FsharpView {
    shadow_path: PathBuf,
    shadow_name: String,
    source_name: String,
    shadow: FsharpShadow,
}

impl FsharpView {
    const ANALYSIS_EXTENSION: &'static str = "cs";
    const WORDS: [(&'static str, &'static str); 2] =
        [("`switch`", "`match`"), ("`foreach`", "`for`")];
    const USINGS: &'static str = "using System;\nusing System.Collections.Generic;\nusing System.Net.Http;\nusing Microsoft.FSharp.Core;\nusing Microsoft.FSharp.Collections;\n";

    pub(crate) fn of(path: &Path, source: &str) -> Option<Self> {
        FsharpSource::is_fsharp(path).then(|| Self::new(path, source))
    }

    pub(crate) fn new(path: &Path, source: &str) -> Self {
        let shadow_path = path.with_extension(Self::ANALYSIS_EXTENSION);
        Self {
            shadow_name: Self::portable(&shadow_path),
            source_name: Self::portable(path),
            shadow_path,
            shadow: FsharpShadow::of(source),
        }
    }

    pub(crate) fn tac(path: &Path, source: &str) -> Self {
        let view = Self::new(path, source);
        Self {
            shadow: view.shadow.without_constructor_needs(),
            ..view
        }
    }

    pub(crate) fn either<T>(
        path: &Path,
        source: &str,
        check: impl Fn(&Path, &str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        Self::run(path, source, &check).unwrap_or_else(|| check(path, source))
    }

    pub(crate) fn run<T>(
        path: &Path,
        source: &str,
        check: impl FnOnce(&Path, &str) -> Result<T, Failure>,
    ) -> Option<Result<T, Failure>> {
        let view = Self::of(path, source)?;
        Some(check(view.path(), view.text()).map_err(|failure| view.failure(&failure)))
    }

    fn portable(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.shadow_path
    }

    pub(crate) fn text(&self) -> &str {
        self.shadow.text()
    }

    pub(crate) fn source_line(&self, shadow_line: usize) -> Option<usize> {
        self.shadow.source_line(shadow_line)
    }

    pub(crate) fn usings(&self) -> String {
        self.using_lines()
    }

    pub(crate) fn authored_usings(&self) -> String {
        let mut usings = String::new();
        for line in self.text().lines() {
            Self::add_using(&mut usings, line.trim());
        }
        usings
    }

    fn using_lines(&self) -> String {
        format!("{}{}", Self::USINGS, self.authored_usings())
    }

    fn add_using(usings: &mut String, line: &str) {
        if line.starts_with("using ") {
            usings.push_str(line);
            usings.push('\n');
        }
    }

    pub(crate) fn failure(&self, failure: &Failure) -> Failure {
        let diagnostics = failure
            .diagnostics()
            .iter()
            .map(|diagnostic| self.diagnostic(diagnostic))
            .collect();
        Failure::new(diagnostics)
    }

    pub(crate) fn diagnostic(&self, diagnostic: &Diagnostic) -> Diagnostic {
        Diagnostic {
            message: self.renamed(&diagnostic.message),
            help: diagnostic.help.as_deref().map(|help| self.renamed(help)),
            spans: diagnostic
                .spans
                .iter()
                .map(|span| self.span(span))
                .collect(),
            ..diagnostic.clone()
        }
    }

    fn renamed(&self, text: &str) -> String {
        let mut renamed = text.replace(&self.shadow_name, &self.source_name);
        for (csharp, fsharp) in Self::WORDS {
            renamed = renamed.replace(csharp, fsharp);
        }
        renamed
    }

    fn span(&self, span: &SourceSpan) -> SourceSpan {
        let ours = span.file == self.shadow_name;
        let mapped = ours.then(|| self.mapped_span(span));
        mapped.unwrap_or_else(|| span.clone())
    }

    fn mapped_span(&self, span: &SourceSpan) -> SourceSpan {
        let start = self.source_offset(span.start);
        let end = self.source_offset(span.end).max(start);
        SourceSpan {
            file: self.source_name.clone(),
            start,
            end,
        }
    }

    fn source_offset(&self, offset: u64) -> u64 {
        let offset = usize::try_from(offset)
            .unwrap_or(usize::MAX)
            .min(self.text().len());
        u64::try_from(self.shadow.source_offset(offset)).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpView;
    use crate::Failure;
    use crate::failure::{diagnostic, source_span};
    use std::path::Path;

    #[test]
    fn a_failure_on_the_shadow_points_at_the_fsharp_file() {
        let path = Path::new("server/services/TallyService.fs");
        let source = "[<Service>]\ntype TallyService() =\n    member this.Count() : int = 1\n";
        let view = FsharpView::of(path, source).unwrap_or_else(|| panic!("fsharp"));
        assert_eq!(view.path(), Path::new("server/services/TallyService.cs"));
        let at = view
            .text()
            .find("Count")
            .unwrap_or_else(|| panic!("member"));
        let failure = Failure::one(diagnostic(
            1002,
            "Bad member in 'server/services/TallyService.cs'",
            None,
            source_span("server/services/TallyService.cs", at, at + 5),
        ));
        let mapped = view.failure(&failure);
        let mapped = &mapped.diagnostics()[0];
        assert_eq!(
            mapped.message,
            "Bad member in 'server/services/TallyService.fs'"
        );
        assert_eq!(mapped.spans[0].file, "server/services/TallyService.fs");
        let start =
            usize::try_from(mapped.spans[0].start).unwrap_or_else(|error| panic!("start: {error}"));
        let line_start = source[..start].rfind('\n').map_or(0, |at| at + 1);
        assert!(source[line_start..].starts_with("    member this.Count"));
    }

    #[test]
    fn other_languages_have_no_view() {
        assert!(FsharpView::of(Path::new("server/services/A.cs"), "class A {}").is_none());
    }
}

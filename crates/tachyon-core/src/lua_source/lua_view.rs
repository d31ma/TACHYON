use super::{LuaShadow, LuaSource};
use crate::Failure;
use std::path::{Path, PathBuf};
use tachyon_diagnostics::{Diagnostic, SourceSpan};

pub(crate) struct LuaView {
    shadow_path: PathBuf,
    shadow_name: String,
    source_name: String,
    shadow: LuaShadow,
    line_starts: Vec<usize>,
}

impl LuaView {
    const ANALYSIS_EXTENSION: &'static str = "rb";
    const WORDS: [(&'static str, &'static str); 12] = [
        (".rb'", ".lua'"),
        ("`#[", "`-- @"),
        ("]`", "`"),
        ("# @param", "---@param"),
        ("# @return", "---@return"),
        ("attr_reader :", "field "),
        ("attr_reader", "a field"),
        ("`def ", "`function Name:"),
        ("YARD", "LuaCATS"),
        ("Ruby annotations", "Lua annotations"),
        ("Ruby class", "Lua class table"),
        ("a Ruby ", "a Lua "),
    ];

    pub(crate) fn of(path: &Path, source: &str) -> Option<Self> {
        LuaSource::is_lua(path).then(|| Self::joined(path, source))
    }

    pub(crate) fn joined(path: &Path, source: &str) -> Self {
        let shadow_path = path.with_extension(Self::ANALYSIS_EXTENSION);
        Self {
            shadow_name: Self::portable(&shadow_path),
            source_name: Self::portable(path),
            shadow_path,
            shadow: LuaShadow::joined(source),
            line_starts: Self::line_starts(source),
        }
    }

    pub(crate) fn analysis(path: &Path, source: &str) -> (PathBuf, String) {
        Self::of(path, source).map_or_else(
            || (path.to_path_buf(), source.to_owned()),
            |view| (view.shadow_path.clone(), view.text().to_owned()),
        )
    }

    pub(crate) fn either<T>(
        path: &Path,
        source: &str,
        check: impl Fn(&Path, &str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        Self::of(path, source).map_or_else(
            || crate::fsharp_source::FsharpView::either(path, source, &check),
            |view| view.checked(&check),
        )
    }

    pub(crate) fn path(&self) -> &Path {
        &self.shadow_path
    }

    pub(crate) fn text(&self) -> &str {
        self.shadow.text()
    }

    pub(crate) fn source_name(&self) -> &str {
        &self.source_name
    }

    pub(crate) fn line_start(&self, source_line: usize) -> usize {
        self.line_starts
            .get(source_line)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn failure(&self, failure: &Failure) -> Failure {
        Failure::new(self.diagnostics(failure.diagnostics()))
    }

    fn checked<T>(&self, check: &impl Fn(&Path, &str) -> Result<T, Failure>) -> Result<T, Failure> {
        check(self.path(), self.text()).map_err(|failure| self.failure(&failure))
    }

    fn line_starts(source: &str) -> Vec<usize> {
        std::iter::once(0)
            .chain(source.match_indices('\n').map(|(at, _)| at + 1))
            .collect()
    }

    fn portable(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn diagnostics(&self, diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
        diagnostics
            .iter()
            .map(|diagnostic| self.in_lua(diagnostic))
            .collect()
    }

    fn in_lua(&self, diagnostic: &Diagnostic) -> Diagnostic {
        let mut mapped = diagnostic.clone();
        mapped.message = self.renamed(&diagnostic.message);
        mapped.help = self.renamed_help(diagnostic.help.as_deref());
        mapped.spans = self.spans(&diagnostic.spans);
        mapped
    }

    fn renamed_help(&self, help: Option<&str>) -> Option<String> {
        help.map(|help| self.renamed(help))
    }

    fn spans(&self, spans: &[SourceSpan]) -> Vec<SourceSpan> {
        spans.iter().map(|span| self.span(span)).collect()
    }

    fn renamed(&self, text: &str) -> String {
        Self::WORDS.iter().fold(
            text.replace(&self.shadow_name, &self.source_name),
            |renamed, (ruby, lua)| renamed.replace(ruby, lua),
        )
    }

    fn span(&self, span: &SourceSpan) -> SourceSpan {
        if span.file != self.shadow_name {
            return span.clone();
        }
        let start = self.mapped(span.start);
        SourceSpan {
            file: self.source_name.clone(),
            start,
            end: self.mapped(span.end).max(start),
        }
    }

    fn mapped(&self, offset: u64) -> u64 {
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        u64::try_from(self.source_offset(offset)).unwrap_or(u64::MAX)
    }

    pub(crate) fn source_offset(&self, shadow_offset: usize) -> usize {
        let shadow_offset = shadow_offset.min(self.text().len());
        let shadow_line = self.text()[..shadow_offset].matches('\n').count();
        let source_line = self.shadow.source_line(shadow_line).unwrap_or_default();
        self.line_start(source_line)
    }
}

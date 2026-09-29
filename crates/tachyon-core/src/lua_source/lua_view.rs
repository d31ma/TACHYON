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
        ("a Ruby", "a Lua"),
        ("Ruby", "Lua"),
    ];

    pub(crate) fn of(path: &Path, source: &str) -> Option<Self> {
        LuaSource::is_lua(path).then(|| Self::with_shadow(path, source, LuaShadow::of(source)))
    }

    pub(crate) fn joined(path: &Path, source: &str) -> Self {
        Self::with_shadow(path, source, LuaShadow::joined(source))
    }

    fn with_shadow(path: &Path, source: &str, shadow: LuaShadow) -> Self {
        let shadow_path = path.with_extension(Self::ANALYSIS_EXTENSION);
        let mut line_starts = vec![0];
        line_starts.extend(source.match_indices('\n').map(|(at, _)| at + 1));
        Self {
            shadow_name: Self::portable(&shadow_path),
            source_name: Self::portable(path),
            shadow_path,
            shadow,
            line_starts,
        }
    }

    pub(crate) fn either<T>(
        path: &Path,
        source: &str,
        check: impl Fn(&Path, &str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        let Some(view) = Self::of(path, source) else {
            return crate::fsharp_source::FsharpView::either(path, source, check);
        };
        check(view.path(), view.text()).map_err(|failure| view.failure(&failure))
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

    pub(crate) fn shadow(&self) -> &LuaShadow {
        &self.shadow
    }

    pub(crate) fn source_name(&self) -> &str {
        &self.source_name
    }

    pub(crate) fn source_offset(&self, shadow_offset: usize) -> usize {
        let shadow_offset = shadow_offset.min(self.text().len());
        let shadow_line = self.text()[..shadow_offset].matches('\n').count();
        let source_line = self.shadow.source_line(shadow_line).unwrap_or_default();
        self.line_start(source_line)
    }

    pub(crate) fn line_start(&self, source_line: usize) -> usize {
        self.line_starts.get(source_line).copied().unwrap_or_default()
    }

    pub(crate) fn failure(&self, failure: &Failure) -> Failure {
        let diagnostics = failure
            .diagnostics()
            .iter()
            .map(|diagnostic| self.diagnostic(diagnostic))
            .collect();
        Failure::new(diagnostics)
    }

    fn diagnostic(&self, diagnostic: &Diagnostic) -> Diagnostic {
        Diagnostic {
            message: self.renamed(&diagnostic.message),
            help: diagnostic.help.as_deref().map(|help| self.renamed(help)),
            spans: diagnostic.spans.iter().map(|span| self.span(span)).collect(),
            ..diagnostic.clone()
        }
    }

    fn renamed(&self, text: &str) -> String {
        let mut renamed = text.replace(&self.shadow_name, &self.source_name);
        for (ruby, lua) in Self::WORDS {
            renamed = renamed.replace(ruby, lua);
        }
        renamed
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
}

use super::ManualStep;
use super::edits::TextEdit;
use super::scan::SourceText;
use crate::behavior::BehaviorsImport;
use std::path::Path;

#[derive(Default)]
pub(super) struct ImportRemoval {
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) manual_steps: Vec<ManualStep>,
}

impl ImportRemoval {
    const GLOBAL_NAMES: &str =
        "Tachyon annotations and types are global in every language (ADR 0046).";

    pub(super) fn plan(relative: &str, source: &str) -> Self {
        let path = Path::new(relative);
        let positions = BehaviorsImport::specifier_positions(path, source);
        let mut removal = Self::default();
        if positions.is_empty() {
            return removal;
        }
        let extension = path.extension().and_then(|value| value.to_str());
        let Some(text) = SourceText::read(extension.unwrap_or(""), source) else {
            return removal;
        };
        let mut ranges = Vec::new();
        for at in positions {
            let statement = ImportStatement { text: &text, at };
            match statement.removal_range() {
                Ok(range) => {
                    let line = text.line_number(range.0);
                    removal.notes.push(format!(
                        "Deleted the import from '{}' on line {line}.",
                        BehaviorsImport::SPECIFIER
                    ));
                    ranges.push(range);
                }
                Err(problem) => removal.manual_steps.push(problem.step(relative, &text, at)),
            }
        }
        for (start, end) in Self::tidy(&text, ranges) {
            removal.edits.push(TextEdit::delete(start, end));
        }
        removal
    }

    fn tidy(text: &SourceText<'_>, ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in ranges {
            match merged.last_mut() {
                Some(last) if last.1 == start => last.1 = end,
                _ => merged.push((start, end)),
            }
        }
        for range in &mut merged {
            if Self::leaves_double_blank_line(text, *range) {
                range.1 = text.line_end(range.1);
            }
        }
        merged
    }

    fn leaves_double_blank_line(text: &SourceText<'_>, (start, end): (usize, usize)) -> bool {
        let whole_lines = text.line_start(start) == start && text.line_start(end) == end;
        if !whole_lines || !text.is_blank_line(end) {
            return false;
        }
        match text.previous_line_start(start) {
            None => true,
            Some(previous) => text.is_blank_line(previous),
        }
    }
}

struct ImportStatement<'t, 'a> {
    text: &'t SourceText<'a>,
    at: usize,
}

enum ImportProblem {
    ReExport,
    DynamicImport,
    LocalNames(String),
    TextAfterImport,
    NoStatement,
}

impl ImportStatement<'_, '_> {
    fn removal_range(&self) -> Result<(usize, usize), ImportProblem> {
        let quote_at = self.at - 1;
        let (keyword_start, keyword_end) = self.keyword_before(quote_at)?;
        let keyword = &self.text.source[keyword_start..keyword_end];
        if keyword == "export" {
            return Err(ImportProblem::ReExport);
        }
        let after_keyword = self.text.next_code(keyword_end);
        if after_keyword.is_some_and(|at| self.text.bytes[at] == b'(') {
            return Err(ImportProblem::DynamicImport);
        }
        self.check_names(keyword_end, quote_at)?;
        let statement_end = self.statement_end();
        self.range(keyword_start, statement_end)
    }

    fn keyword_before(&self, quote_at: usize) -> Result<(usize, usize), ImportProblem> {
        let text = self.text;
        let mut at = quote_at;
        while at > 0 {
            at -= 1;
            if !text.is_code(at) {
                continue;
            }
            if text.bytes[at] == b';' {
                break;
            }
            if !text.is_word_byte(at) {
                continue;
            }
            let end = at + 1;
            at = text.word_start(end);
            if matches!(&text.source[at..end], "import" | "export") {
                return Ok((at, end));
            }
        }
        Err(ImportProblem::NoStatement)
    }

    fn check_names(&self, keyword_end: usize, quote_at: usize) -> Result<(), ImportProblem> {
        let mut clause = String::new();
        for at in keyword_end..quote_at {
            if self.text.is_code(at) {
                clause.push(char::from(self.text.bytes[at]));
            } else {
                clause.push(' ');
            }
        }
        let clause = clause.trim();
        let clause = clause.strip_suffix("from").unwrap_or(clause).trim();
        if clause.is_empty() || Self::is_plain_name_list(clause) {
            return Ok(());
        }
        Err(ImportProblem::LocalNames(
            clause.split_whitespace().collect::<Vec<_>>().join(" "),
        ))
    }

    fn is_plain_name_list(clause: &str) -> bool {
        let clause = clause.strip_prefix("type").map_or(clause, str::trim_start);
        let Some(inside) = clause
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
        else {
            return false;
        };
        for item in inside.split(',') {
            let item = item.trim();
            let name = item.strip_prefix("type ").map_or(item, str::trim_start);
            let is_name = name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'));
            if !is_name {
                return false;
            }
        }
        true
    }

    fn statement_end(&self) -> usize {
        let text = self.text;
        let mut end = self.at + BehaviorsImport::SPECIFIER.len() + 1;
        end = Self::skip_spaces(text, end);
        if text.bytes.get(end) == Some(&b';') {
            end += 1;
        }
        end
    }

    fn range(&self, keyword_start: usize, end: usize) -> Result<(usize, usize), ImportProblem> {
        let text = self.text;
        let rest = Self::skip_spaces(text, end);
        let line_ends = matches!(text.bytes.get(rest), None | Some(b'\n' | b'\r'));
        if !line_ends && text.is_word_byte(rest) {
            return Err(ImportProblem::TextAfterImport);
        }
        let line_start = text.line_start(keyword_start);
        let starts_line = text.source[line_start..keyword_start].trim().is_empty();
        if starts_line && line_ends {
            return Ok((line_start, text.line_end(rest)));
        }
        if line_ends {
            return Ok((keyword_start, end));
        }
        Ok((keyword_start, rest))
    }

    fn skip_spaces(text: &SourceText<'_>, from: usize) -> usize {
        let mut at = from;
        while matches!(text.bytes.get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
        at
    }
}

impl ImportProblem {
    fn step(&self, relative: &str, text: &SourceText<'_>, at: usize) -> ManualStep {
        let specifier = BehaviorsImport::SPECIFIER;
        let global = ImportRemoval::GLOBAL_NAMES;
        let (problem, action) = match self {
            Self::ReExport => (
                format!("This file re-exports names from '{specifier}'."),
                format!(
                    "{global} Delete this export. A file that imports these names from this file can delete that import too."
                ),
            ),
            Self::DynamicImport => (
                format!("This file loads '{specifier}' with import()."),
                format!("{global} Use the global names directly, then delete this import()."),
            ),
            Self::LocalNames(clause) => (
                format!("This import from '{specifier}' gives the names a local name: {clause}"),
                format!(
                    "{global} Replace each use of the local name with the global name (for example Behavior.Service becomes Service), then delete this import."
                ),
            ),
            Self::TextAfterImport | Self::NoStatement => (
                format!(
                    "This import from '{specifier}' has a form that is not changed automatically."
                ),
                format!("{global} Delete this import by hand."),
            ),
        };
        ManualStep::new(
            &format!("{relative}:{}", text.line_number(at)),
            problem,
            action,
        )
    }
}

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
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = (!positions.is_empty())
            .then(|| SourceText::read(extension, source))
            .flatten();
        text.map(|text| Self::remove(relative, &text, positions))
            .unwrap_or_default()
    }

    fn remove(relative: &str, text: &SourceText<'_>, positions: Vec<usize>) -> Self {
        let mut removal = Self::default();
        let mut ranges = Vec::new();
        for at in positions {
            removal.add_statement(relative, text, at, &mut ranges);
        }
        removal.edits = Self::tidy(text, ranges)
            .into_iter()
            .map(|(start, end)| TextEdit::delete(start, end))
            .collect();
        removal
    }

    fn add_statement(
        &mut self,
        relative: &str,
        text: &SourceText<'_>,
        at: usize,
        ranges: &mut Vec<(usize, usize)>,
    ) {
        let statement = ImportStatement { text, at };
        let removed = statement.removal_range();
        let problem = removed.as_ref().err();
        let step = problem.map(|problem| problem.step(relative, text, at));
        self.manual_steps.extend(step);
        let range = removed.ok();
        let note = range.map(|(start, _)| {
            format!(
                "Deleted the import from '{}' on line {}.",
                BehaviorsImport::SPECIFIER,
                text.line_number(start)
            )
        });
        self.notes.extend(note);
        ranges.extend(range);
    }

    fn tidy(text: &SourceText<'_>, ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        Self::merged(ranges)
            .into_iter()
            .map(|range| Self::without_double_blank_line(text, range))
            .collect()
    }

    fn merged(ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        let mut merged = Vec::new();
        for (start, end) in ranges {
            Self::merge(&mut merged, start, end);
        }
        merged
    }

    fn merge(merged: &mut Vec<(usize, usize)>, start: usize, end: usize) {
        match merged.last_mut() {
            Some(last) if last.1 == start => last.1 = end,
            _ => merged.push((start, end)),
        }
    }

    fn without_double_blank_line(text: &SourceText<'_>, range: (usize, usize)) -> (usize, usize) {
        if Self::leaves_double_blank_line(text, range) {
            return (range.0, text.line_end(range.1));
        }
        range
    }

    fn leaves_double_blank_line(text: &SourceText<'_>, (start, end): (usize, usize)) -> bool {
        let whole_lines = [text.line_start(start) == start, text.line_start(end) == end];
        let blank_after = text.is_blank_line(end);
        let previous = text.previous_line_start(start);
        let blank_before = previous.is_none_or(|previous| text.is_blank_line(previous));
        [whole_lines == [true, true], blank_after, blank_before] == [true; 3]
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
    const KEYWORDS: [&'static str; 2] = ["import", "export"];
    const SPACES: &'static [u8] = b" \t";
    const LINE_ENDS: &'static [u8] = b"\n\r";

    fn removal_range(&self) -> Result<(usize, usize), ImportProblem> {
        let quote_at = self.at - 1;
        let (keyword_start, keyword_end) = self.keyword_before(quote_at)?;
        let keyword = &self.text.source[keyword_start..keyword_end];
        Self::refuse(keyword == "export", ImportProblem::ReExport)?;
        let after_keyword = self.text.next_code(keyword_end);
        let dynamic = after_keyword.is_some_and(|at| self.text.bytes[at] == b'(');
        Self::refuse(dynamic, ImportProblem::DynamicImport)?;
        self.check_names(keyword_end, quote_at)?;
        let statement_end = self.statement_end();
        self.range(keyword_start, statement_end)
    }

    fn refuse(found: bool, problem: ImportProblem) -> Result<(), ImportProblem> {
        (!found).then_some(()).ok_or(problem)
    }

    fn keyword_before(&self, quote_at: usize) -> Result<(usize, usize), ImportProblem> {
        let first = self.previous_word(quote_at);
        let mut words = std::iter::successors(first, |(start, _)| self.previous_word(*start));
        let keyword = words.find(|word| self.is_keyword(*word));
        keyword.ok_or(ImportProblem::NoStatement)
    }

    fn previous_word(&self, before: usize) -> Option<(usize, usize)> {
        let mark = (0..before).rev().find(|at| self.is_mark(*at))?;
        self.text.is_word_byte(mark).then_some(())?;
        let end = mark + 1;
        Some((self.text.word_start(end), end))
    }

    fn is_mark(&self, at: usize) -> bool {
        let semicolon = self.text.bytes[at] == b';';
        let mark = [semicolon, self.text.is_word_byte(at)].contains(&true);
        [self.text.is_code(at), mark] == [true, true]
    }

    fn is_keyword(&self, (start, end): (usize, usize)) -> bool {
        Self::KEYWORDS.contains(&&self.text.source[start..end])
    }

    fn check_names(&self, keyword_end: usize, quote_at: usize) -> Result<(), ImportProblem> {
        let clause: String = (keyword_end..quote_at)
            .map(|at| self.code_character(at))
            .collect();
        let clause = clause.trim();
        let clause = clause.strip_suffix("from").unwrap_or(clause).trim();
        let plain = [clause.is_empty(), Self::is_plain_name_list(clause)].contains(&true);
        plain.then_some(()).ok_or_else(|| {
            ImportProblem::LocalNames(clause.split_whitespace().collect::<Vec<_>>().join(" "))
        })
    }

    fn code_character(&self, at: usize) -> char {
        if self.text.is_code(at) {
            return char::from(self.text.bytes[at]);
        }
        ' '
    }

    fn is_plain_name_list(clause: &str) -> bool {
        let clause = clause.strip_prefix("type").map_or(clause, str::trim_start);
        let inside = clause
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'));
        inside.is_some_and(|inside| inside.split(',').all(Self::is_plain_name))
    }

    fn is_plain_name(item: &str) -> bool {
        let item = item.trim();
        let name = item.strip_prefix("type ").map_or(item, str::trim_start);
        name.bytes().all(Self::is_name_byte)
    }

    fn is_name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'$'].contains(&true)
    }

    fn statement_end(&self) -> usize {
        let after = Self::skip_spaces(self.text, self.at + BehaviorsImport::SPECIFIER.len() + 1);
        let semicolon = self.text.bytes.get(after) == Some(&b';');
        after + usize::from(semicolon)
    }

    fn range(&self, keyword_start: usize, end: usize) -> Result<(usize, usize), ImportProblem> {
        let text = self.text;
        let rest = Self::skip_spaces(text, end);
        let next = text.bytes.get(rest).copied();
        let line_ends = next.is_none_or(|byte| Self::LINE_ENDS.contains(&byte));
        let text_follows = [line_ends, text.is_word_byte(rest)] == [false, true];
        Self::refuse(text_follows, ImportProblem::TextAfterImport)?;
        let line_start = text.line_start(keyword_start);
        let starts_line = text.source[line_start..keyword_start].trim().is_empty();
        match (starts_line, line_ends) {
            (true, true) => Ok((line_start, text.line_end(rest))),
            (false, true) => Ok((keyword_start, end)),
            (_, false) => Ok((keyword_start, rest)),
        }
    }

    fn skip_spaces(text: &SourceText<'_>, from: usize) -> usize {
        let rest = text.bytes.get(from..).unwrap_or_default();
        let spaces = rest
            .iter()
            .take_while(|byte| Self::SPACES.contains(byte))
            .count();
        from + spaces
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

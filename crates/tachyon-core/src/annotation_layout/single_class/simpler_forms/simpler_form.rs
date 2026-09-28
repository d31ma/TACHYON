#![allow(clippy::needless_bitwise_bool)]

use super::form_pattern::FormPattern;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SimplerForm {
    pub(crate) pattern: FormPattern,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) replacement: String,
}

impl SimplerForm {
    pub(super) fn new(pattern: FormPattern, start: usize, end: usize, replacement: String) -> Self {
        Self {
            pattern,
            start,
            end,
            replacement,
        }
    }

    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        (self.start < other.end) & (other.start < self.end)
    }

    pub(crate) fn line(&self, source: &str) -> usize {
        let before = source.get(..self.start).unwrap_or("");
        before.matches('\n').count() + 1
    }

    pub(crate) fn removes_lines(&self, source: &str) -> bool {
        let starts_line = source
            .get(..self.start)
            .is_none_or(|text| text.is_empty() | text.ends_with('\n'));
        let ends_line = source
            .get(..self.end)
            .is_some_and(|text| text.ends_with('\n'));
        self.replacement.is_empty() & starts_line & ends_line
    }

    pub(crate) fn rewritten_lines(&self, source: &str) -> String {
        let head = source
            .get(..self.start)
            .and_then(|text| text.rfind('\n'))
            .map_or(0, |newline| newline + 1);
        let tail = source
            .get(self.end..)
            .and_then(|text| text.find('\n'))
            .map_or(source.len(), |newline| self.end + newline);
        let before = source.get(head..self.start).unwrap_or("");
        let after = source.get(self.end..tail).unwrap_or("");
        format!("{before}{}{after}", self.replacement)
    }
}

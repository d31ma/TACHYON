#![allow(clippy::needless_bitwise_bool)]

use super::InteropRules;
use std::ops::Range;

pub(super) struct InteropFinder<'s> {
    source: &'s str,
    code: Vec<bool>,
}

impl<'s> InteropFinder<'s> {
    const QUOTES: [char; 2] = ['"', '\''];

    pub(super) fn new(extension: &str, source: &'s str) -> Self {
        let mask = crate::lexical::code_mask(extension, source);
        Self {
            source,
            code: mask.code,
        }
    }

    pub(super) fn uses(&self, rules: &InteropRules) -> Vec<Range<usize>> {
        let mut found = self.all_words(rules.words);
        found.extend(self.all_imports(rules.imports));
        found.sort_by_key(|range| range.start);
        found
    }

    pub(super) fn all_words(&self, words: &[&str]) -> Vec<Range<usize>> {
        words.iter().flat_map(|word| self.word_uses(word)).collect()
    }

    fn word_uses(&self, word: &str) -> Vec<Range<usize>> {
        self.source
            .match_indices(word)
            .map(|(start, _)| start..start + word.len())
            .filter(|range| self.is_code_word(range, word))
            .collect()
    }

    fn is_code_word(&self, range: &Range<usize>, word: &str) -> bool {
        let in_code = self.code[range.start];
        in_code & self.bounded_before(range.start) & self.bounded_after(range.end, word)
    }

    fn bounded_before(&self, start: usize) -> bool {
        let before = self.source[..start].chars().next_back();
        before.is_none_or(|character| !Self::is_name_character(character))
    }

    fn bounded_after(&self, end: usize, word: &str) -> bool {
        let after = self.source[end..].chars().next();
        let ends_itself = word.ends_with(['.', ':']);
        ends_itself | after.is_none_or(|character| !Self::is_name_character(character))
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() | (character == '_')
    }

    fn all_imports(&self, imports: &[&str]) -> Vec<Range<usize>> {
        imports
            .iter()
            .flat_map(|import| self.quoted_uses(import))
            .collect()
    }

    fn quoted_uses(&self, import: &str) -> Vec<Range<usize>> {
        Self::QUOTES
            .iter()
            .flat_map(|quote| self.import_uses(*quote, import))
            .collect()
    }

    fn import_uses(&self, quote: char, import: &str) -> Vec<Range<usize>> {
        let opened = format!("{quote}{import}");
        self.source
            .match_indices(&opened)
            .map(|(start, _)| start + 1..start + opened.len())
            .filter(|range| self.is_import(range, quote, import))
            .collect()
    }

    fn is_import(&self, range: &Range<usize>, quote: char, import: &str) -> bool {
        let opens_string = !self.code[range.start - 1] & self.code_before(range.start - 1);
        let whole = import.ends_with('/') | self.source[range.end..].starts_with(quote);
        opens_string & whole
    }

    fn code_before(&self, at: usize) -> bool {
        at.checked_sub(1).is_none_or(|before| self.code[before])
    }
}

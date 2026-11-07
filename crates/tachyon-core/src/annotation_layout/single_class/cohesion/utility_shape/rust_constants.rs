use super::Words;
use std::ops::Range;

pub(in crate::annotation_layout::single_class) struct RustConstants;

impl RustConstants {
    pub(in crate::annotation_layout::single_class) fn read(
        words: &Words<'_>,
        extension: &str,
    ) -> Vec<Range<usize>> {
        let count = Self::word_count(words, extension);
        Self::ranges(words, count)
    }

    fn word_count(words: &Words<'_>, extension: &str) -> usize {
        if extension == "rs" { words.len() } else { 0 }
    }

    fn ranges(words: &Words<'_>, count: usize) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        for at in 0..count {
            Self::add_implementation(words, at, &mut ranges);
        }
        ranges
    }

    fn add_implementation(words: &Words<'_>, at: usize, ranges: &mut Vec<Range<usize>>) {
        if words.text(at) == "impl"
            && let Some(open) = words.find_at_depth(at + 1, words.depth(at), &["{"])
        {
            ranges.extend(Self::members(words, open));
        }
    }

    fn members(words: &Words<'_>, open: usize) -> Vec<Range<usize>> {
        words
            .inside(open)
            .filter(|at| words.depth(*at) == words.depth(open) + 1)
            .filter_map(|at| Self::item(words, at))
            .collect()
    }

    fn item(words: &Words<'_>, at: usize) -> Option<Range<usize>> {
        let declaration = [
            words.text(at) == "const",
            words.is_name(at + 1),
            words.text(at + 2) == ":",
        ];
        let valid = declaration.into_iter().all(|valid| valid);
        let end = Self::item_end(words, at, valid);
        let start = Self::visibility(words, at);
        end.map(|end| start..end + 1)
    }

    fn item_end(words: &Words<'_>, at: usize, valid: bool) -> Option<usize> {
        if !valid {
            return None;
        }
        words.find_at_depth(at + 3, words.depth(at), &[";"])
    }

    pub(in crate::annotation_layout::single_class) fn method_start(
        words: &Words<'_>,
        at: usize,
    ) -> usize {
        let modifiers = Self::modifiers(words, at);
        let external = Self::extern_abi(words, modifiers);
        Self::visibility(words, external)
    }

    fn modifiers(words: &Words<'_>, at: usize) -> usize {
        let mut start = at;
        while ["const", "async", "unsafe", "extern"].contains(&words.text(start.wrapping_sub(1))) {
            start -= 1;
        }
        start
    }

    fn extern_abi(words: &Words<'_>, at: usize) -> usize {
        if words.is_literal(at.wrapping_sub(1)) && words.text(at.wrapping_sub(2)) == "extern" {
            return at - 2;
        }
        at
    }

    fn visibility(words: &Words<'_>, at: usize) -> usize {
        if words.text(at.wrapping_sub(1)) == "pub" {
            return at - 1;
        }
        (0..at)
            .rev()
            .find(|before| {
                [
                    words.text(*before) == "pub",
                    words.text(*before + 1) == "(",
                    words.after_group(*before + 1) == at,
                ]
                .into_iter()
                .all(|valid| valid)
            })
            .unwrap_or(at)
    }
}

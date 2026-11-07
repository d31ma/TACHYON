use super::super::cohesion::{RustConstants, Words};
use super::Tokens;
use std::ops::Range;

pub(super) struct RustDeclarations;

impl RustDeclarations {
    pub(super) fn read(tokens: &Tokens<'_>, text: &str, extension: &str) -> Vec<Range<usize>> {
        let words = Words::read(tokens, text);
        Self::ranges(&words, extension)
    }

    fn ranges(words: &Words<'_>, extension: &str) -> Vec<Range<usize>> {
        if extension != "rs" {
            return Vec::new();
        }
        let mut ranges: Vec<_> = RustConstants::read(words, extension)
            .into_iter()
            .filter(|range| Self::literal_value(words, range))
            .collect();
        ranges.extend(Self::scaffolding(words));
        ranges
    }

    fn literal_value(words: &Words<'_>, range: &Range<usize>) -> bool {
        let equal = words.find_at_depth(range.start, words.depth(range.start), &["="]);
        equal.is_some_and(|at| Self::single_value(words, at, range.end))
    }

    fn single_value(words: &Words<'_>, equal: usize, end: usize) -> bool {
        let at = equal + 1;
        let number = words
            .text(at)
            .starts_with(|first: char| first.is_ascii_digit());
        let literal = words.is_literal(at) | number | ["true", "false"].contains(&words.text(at));
        literal & (words.text(at + 1) == ";") & (at + 2 == end)
    }

    fn scaffolding(words: &Words<'_>) -> Vec<Range<usize>> {
        (0..words.len())
            .filter(|at| words.depth(*at) == 0)
            .flat_map(|at| Self::declaration(words, at))
            .collect()
    }

    fn declaration(words: &Words<'_>, at: usize) -> Vec<Range<usize>> {
        match words.text(at) {
            "impl" => Self::implementation(words, at),
            "struct" => Self::unit_type(words, at).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn unit_type(words: &Words<'_>, at: usize) -> Option<Range<usize>> {
        let unit = words.is_name(at + 1) & (words.text(at + 2) == ";");
        unit.then_some(RustConstants::method_start(words, at)..at + 3)
    }

    fn implementation(words: &Words<'_>, at: usize) -> Vec<Range<usize>> {
        let open = words.find_at_depth(at + 1, 0, &["{"]);
        open.map_or_else(Vec::new, |open| {
            let close = words.after_group(open).saturating_sub(1);
            vec![at..open + 1, close..close + 1]
        })
    }
}

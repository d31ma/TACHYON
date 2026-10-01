use super::cohesion::{RustConstants, Words};
use super::{Failure, Input, Tokens};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct RustUtilityExports;

impl RustUtilityExports {
    pub(super) fn source(path: &Path, source: &str) -> Result<String, Failure> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input)?;
        tokens.check_complete(&input)?;
        let words = Words::read(&tokens, source);
        let ranges = Self::ranges(&words);
        Ok(Self::rewrite(source, &ranges))
    }

    fn ranges(words: &Words<'_>) -> BTreeMap<usize, usize> {
        Self::structures(words)
            .into_iter()
            .chain(Self::members(words))
            .map(|at| Self::visibility(words, at))
            .collect()
    }

    fn structures(words: &Words<'_>) -> Vec<usize> {
        (0..words.len())
            .filter(|at| (words.depth(*at), words.text(*at)) == (0, "struct"))
            .collect()
    }

    fn members(words: &Words<'_>) -> Vec<usize> {
        (0..words.len())
            .filter(|at| (words.depth(*at), words.text(*at)) == (0, "impl"))
            .flat_map(|at| Self::implementation(words, at))
            .collect()
    }

    fn implementation(words: &Words<'_>, at: usize) -> Vec<usize> {
        let open = words.find_at_depth(at + 1, 0, &["{"]);
        let open = open.filter(|open| !words.has_word(at + 1..*open, &["for"]));
        open.map_or_else(Vec::new, |open| Self::items(words, open))
    }

    fn items(words: &Words<'_>, open: usize) -> Vec<usize> {
        words
            .inside(open)
            .filter(|at| words.depth(*at) == words.depth(open) + 1)
            .filter(|at| ["fn", "const"].contains(&words.text(*at)))
            .collect()
    }

    fn visibility(words: &Words<'_>, at: usize) -> (usize, usize) {
        let start = RustConstants::method_start(words, at);
        let end = Self::visibility_end(words, start);
        (words.start(start), words.start(end))
    }

    fn visibility_end(words: &Words<'_>, start: usize) -> usize {
        match (words.text(start), words.text(start + 1)) {
            ("pub", "(") => words.after_group(start + 1),
            ("pub", _) => start + 1,
            _ => start,
        }
    }

    fn rewrite(source: &str, ranges: &BTreeMap<usize, usize>) -> String {
        let mut output = source.to_owned();
        for (start, end) in ranges.iter().rev() {
            output.replace_range(*start..*end, "pub(super) ");
        }
        output
    }
}

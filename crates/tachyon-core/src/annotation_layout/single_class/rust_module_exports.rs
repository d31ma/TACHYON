use super::cohesion::{RustConstants, Words};
use super::{Failure, Input, Tokens};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct RustModuleExports;

impl RustModuleExports {
    const TYPE_KEYWORDS: [&'static str; 3] = ["struct", "enum", "trait"];

    pub(crate) fn source(path: &Path, source: &str) -> Result<(String, bool), Failure> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input)?;
        tokens.check_complete(&input)?;
        let words = Words::read(&tokens, source);
        let ranges = Self::ranges(&words);
        let public = Self::public_type(&words);
        Ok((Self::rewrite(source, &ranges), public))
    }

    fn ranges(words: &Words<'_>) -> BTreeMap<usize, usize> {
        Self::types(words)
            .into_iter()
            .chain(Self::fields(words))
            .chain(Self::members(words))
            .filter_map(|at| Self::visibility(words, at))
            .collect()
    }

    fn types(words: &Words<'_>) -> Vec<usize> {
        (0..words.len())
            .filter(|at| words.depth(*at) == 0)
            .filter(|at| Self::TYPE_KEYWORDS.contains(&words.text(*at)))
            .collect()
    }

    fn fields(words: &Words<'_>) -> Vec<usize> {
        Self::types(words)
            .into_iter()
            .filter(|at| words.text(*at) == "struct")
            .flat_map(|at| Self::structure_fields(words, at))
            .collect()
    }

    fn structure_fields(words: &Words<'_>, at: usize) -> Vec<usize> {
        let open = words.find_at_depth(at + 1, 0, &["{", ";", "("]);
        let open = open.filter(|open| words.text(*open) == "{");
        open.map_or_else(Vec::new, |open| Self::named_fields(words, open))
    }

    fn named_fields(words: &Words<'_>, open: usize) -> Vec<usize> {
        words
            .inside(open)
            .filter(|at| words.depth(*at) == words.depth(open) + 1)
            .filter(|at| Self::is_field_name(words, *at))
            .collect()
    }

    fn is_field_name(words: &Words<'_>, at: usize) -> bool {
        let typed = words.text(at + 1) == ":";
        let path_start = words.pair(at + 1) == "::";
        let path_end = words.text(at.wrapping_sub(1)) == ":";
        [words.is_name(at), typed, !path_start, !path_end]
            .iter()
            .all(|holds| *holds)
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

    fn public_type(words: &Words<'_>) -> bool {
        Self::types(words)
            .into_iter()
            .any(|at| words.text(at.wrapping_sub(1)) == "pub")
    }

    fn visibility(words: &Words<'_>, at: usize) -> Option<(usize, usize)> {
        let start = RustConstants::method_start(words, at);
        (words.text(start) != "pub").then_some((words.start(start), words.start(start)))
    }

    fn rewrite(source: &str, ranges: &BTreeMap<usize, usize>) -> String {
        let mut output = source.to_owned();
        for (start, end) in ranges.iter().rev() {
            output.replace_range(*start..*end, "pub(super) ");
        }
        output
    }
}

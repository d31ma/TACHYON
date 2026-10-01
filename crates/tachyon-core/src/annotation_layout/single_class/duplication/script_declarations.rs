use super::super::cohesion::Words;
use super::Tokens;
use std::ops::Range;

pub(super) struct ScriptDeclarations;

impl ScriptDeclarations {
    pub(super) fn read(tokens: &Tokens<'_>, text: &str, extension: &str) -> Vec<Range<usize>> {
        Self::ranges(&Words::read(tokens, text), text, extension)
    }

    fn ranges(words: &Words<'_>, text: &str, extension: &str) -> Vec<Range<usize>> {
        (0..Self::word_count(words, extension))
            .filter(|at| Self::is_class(words, *at))
            .filter_map(|at| words.find_at_depth(at + 1, 0, &["{"]))
            .flat_map(|open| Self::members(words, text, open))
            .collect()
    }

    fn word_count(words: &Words<'_>, extension: &str) -> usize {
        if ["js", "ts"].contains(&extension) {
            words.len()
        } else {
            0
        }
    }

    fn is_class(words: &Words<'_>, at: usize) -> bool {
        (words.depth(at) == 0) & (words.text(at) == "class")
    }

    fn members(words: &Words<'_>, text: &str, open: usize) -> Vec<Range<usize>> {
        words
            .inside(open)
            .filter(|at| Self::is_static(words, *at))
            .filter_map(|at| Self::member(words, text, at))
            .collect()
    }

    fn is_static(words: &Words<'_>, at: usize) -> bool {
        (words.depth(at) == 1) & (words.text(at) == "static")
    }

    fn member(words: &Words<'_>, text: &str, at: usize) -> Option<Range<usize>> {
        let name = Self::name_at(words, at);
        let equal = Self::equal_at(words, name)?;
        let end = words.find_at_depth(equal + 1, 1, &[";"])?;
        Self::literal(words, text, equal + 1, end).then(|| Self::start(words, at)..end + 1)
    }

    fn name_at(words: &Words<'_>, at: usize) -> usize {
        let next = at + 1 + usize::from(words.text(at + 1) == "readonly");
        next + usize::from(words.text(next) == "#")
    }

    fn equal_at(words: &Words<'_>, name: usize) -> Option<usize> {
        Self::constant_name(words.text(name))
            .then(|| Self::initializer_at(words, name))
            .flatten()
    }

    fn initializer_at(words: &Words<'_>, name: usize) -> Option<usize> {
        match words.text(name + 1) {
            "=" => Some(name + 1),
            ":" => Self::typed_equal(words, name + 2),
            _ => None,
        }
    }

    fn constant_name(name: &str) -> bool {
        name.starts_with(|first: char| first.is_ascii_uppercase())
            & name.bytes().all(Self::constant_byte)
    }

    fn constant_byte(byte: u8) -> bool {
        byte.is_ascii_uppercase() | byte.is_ascii_digit() | (byte == b'_')
    }

    fn typed_equal(words: &Words<'_>, from: usize) -> Option<usize> {
        let at = words.find_at_depth(from, 1, &["=", ";"])?;
        (words.text(at) == "=").then_some(at)
    }

    fn literal(words: &Words<'_>, text: &str, start: usize, end: usize) -> bool {
        Self::single_literal(words, start, end) | Self::number(words, text, start, end)
    }

    fn single_literal(words: &Words<'_>, start: usize, end: usize) -> bool {
        let value = words.text(start);
        let quoted = words.is_literal(start) & value.starts_with(['\'', '"']);
        let primitive = ["true", "false", "null"].contains(&value);
        (end == start + 1) & (quoted | primitive)
    }

    fn number(words: &Words<'_>, text: &str, start: usize, end: usize) -> bool {
        let value = &text[words.start(start)..words.start(end)];
        let first = value.trim_start().bytes().next().unwrap_or(b' ');
        let numeric = first.is_ascii_digit() | [b'-', b'+', b'.'].contains(&first);
        let parsed = value.trim().replace('_', "").parse::<f64>().is_ok();
        numeric & parsed
    }

    fn start(words: &Words<'_>, at: usize) -> usize {
        (0..at)
            .rev()
            .take_while(|before| {
                (words.depth(*before) == 1)
                    & ["private", "public", "protected", "readonly", "declare"]
                        .contains(&words.text(*before))
            })
            .last()
            .unwrap_or(at)
    }
}

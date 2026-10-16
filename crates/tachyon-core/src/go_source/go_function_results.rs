use std::ops::Range;

pub(crate) struct GoFunctionResults;

impl GoFunctionResults {
    pub(crate) fn read(code: &str) -> Vec<Range<usize>> {
        code.match_indices("func")
            .filter_map(|(at, _)| Self::at(code, at))
            .collect()
    }

    fn at(code: &str, at: usize) -> Option<Range<usize>> {
        (super::word_at(code, at) == "func")
            .then(|| Self::result(code, at))
            .flatten()
    }

    fn result(code: &str, at: usize) -> Option<Range<usize>> {
        let close = Self::parameters_close(code, at)?;
        let open = super::body_open(code, close + 1)?;
        Some(close + 1..open)
    }

    fn parameters_close(code: &str, at: usize) -> Option<usize> {
        let start = super::skip_spaces(code, at + "func".len());
        let name = super::word_at(code, start);
        let open = Self::parameters_open(code, start + name.len())?;
        let close = Self::parentheses_close(code, open)?;
        Self::receiver_parameters(code, close, name.is_empty())
    }

    fn parameters_open(code: &str, at: usize) -> Option<usize> {
        let open = super::skip_spaces(code, at);
        if code.as_bytes().get(open) == Some(&b'[') {
            let close = super::matching(code, open, b'[', b']')?;
            return Some(super::skip_spaces(code, close + 1));
        }
        Some(open)
    }

    fn parentheses_close(code: &str, open: usize) -> Option<usize> {
        (code.as_bytes().get(open) == Some(&b'('))
            .then(|| super::matching(code, open, b'(', b')'))
            .flatten()
    }

    fn receiver_parameters(code: &str, close: usize, unnamed: bool) -> Option<usize> {
        let next = super::skip_spaces(code, close + 1);
        let method = super::word_at(code, next);
        let parameters = super::skip_spaces(code, next + method.len());
        let receiver = [
            unnamed,
            !method.is_empty(),
            method != "func",
            code.as_bytes().get(parameters) == Some(&b'('),
        ] == [true; 4];
        receiver
            .then(|| super::matching(code, parameters, b'(', b')'))
            .unwrap_or(Some(close))
    }

    pub(super) fn continues(code: &str, at: usize, previous_word: &str) -> bool {
        let before = code[..at].trim_end();
        [
            before.ends_with(['*', '.']),
            before.ends_with("<-"),
            Self::keyword_continues(before, previous_word),
        ]
        .contains(&true)
    }

    fn keyword_continues(before: &str, previous_word: &str) -> bool {
        [
            before.ends_with(previous_word),
            ["chan", "map", "struct", "interface", "func"].contains(&previous_word),
        ] == [true; 2]
    }
}

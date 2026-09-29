use super::Input;
use super::routes::RouteMarker;
use std::ops::Range;

pub(super) struct ParameterMarkers;

impl ParameterMarkers {
    const SCRIPT_SYNTAX: [&'static str; 2] = ["js", "ts"];
    const OPEN: u8 = b'(';
    const CLOSE: u8 = b')';
    const MARK: u8 = b'@';

    pub(super) fn spans(input: &Input<'_>) -> Vec<Range<usize>> {
        if !Self::SCRIPT_SYNTAX.contains(&input.extension) {
            return Vec::new();
        }
        let mut spans = Vec::new();
        let mut depth = 0_usize;
        for (at, byte) in input.code.bytes().enumerate() {
            if byte == Self::OPEN {
                depth += 1;
            } else if byte == Self::CLOSE {
                depth = depth.saturating_sub(1);
            } else if byte == Self::MARK && depth > 0 {
                spans.extend(Self::marker_span(&input.code, at));
            }
        }
        spans
    }

    fn marker_span(code: &str, at: usize) -> Option<Range<usize>> {
        let name_start = at + 1;
        let name_end = Self::name_end(code, name_start);
        RouteMarker::source(&code[name_start..name_end])?;
        let end = Self::arguments_end(code, name_end).unwrap_or(name_end);
        Some(at..end)
    }

    fn name_end(code: &str, start: usize) -> usize {
        let length = code[start..]
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        start + length
    }

    fn arguments_end(code: &str, name_end: usize) -> Option<usize> {
        let has_arguments = code.as_bytes().get(name_end) == Some(&Self::OPEN);
        let closing = has_arguments
            .then(|| super::matching(code, name_end, Self::OPEN, Self::CLOSE))
            .flatten()?;
        Some(closing + 1)
    }
}

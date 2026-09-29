use super::Input;
use super::routes::RouteMarker;
use std::ops::Range;

pub(super) struct ParameterMarkers;

impl ParameterMarkers {
    const SCRIPT_SYNTAX: [&'static str; 2] = ["js", "ts"];
    const OPEN: u8 = b'(';
    const CLOSE: u8 = b')';
    const MARK: u8 = b'@';
    const NAME_JOIN: u8 = b'_';

    pub(super) fn spans(input: &Input<'_>) -> Vec<Range<usize>> {
        let code = Self::script_code(input);
        let positions = Self::marked_positions(code);
        positions
            .into_iter()
            .filter_map(|at| Self::marker_span(code, at))
            .collect()
    }

    fn script_code<'a>(input: &'a Input<'_>) -> &'a str {
        if Self::SCRIPT_SYNTAX.contains(&input.extension) {
            return &input.code;
        }
        ""
    }

    fn marked_positions(code: &str) -> Vec<usize> {
        let mut depth = 0_usize;
        let mut positions = Vec::new();
        for (at, byte) in code.bytes().enumerate() {
            positions.extend(Self::inside_mark(depth, byte, at));
            depth = Self::depth_after(depth, byte);
        }
        positions
    }

    fn inside_mark(depth: usize, byte: u8, at: usize) -> Option<usize> {
        (depth > 0 && byte == Self::MARK).then_some(at)
    }

    fn depth_after(depth: usize, byte: u8) -> usize {
        match byte {
            Self::OPEN => depth + 1,
            Self::CLOSE => depth.saturating_sub(1),
            _ => depth,
        }
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
            .take_while(|byte| Self::is_name_byte(*byte))
            .count();
        start + length
    }

    fn is_name_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == Self::NAME_JOIN
    }

    fn arguments_end(code: &str, name_end: usize) -> Option<usize> {
        let has_arguments = code.as_bytes().get(name_end) == Some(&Self::OPEN);
        let closing = has_arguments
            .then(|| super::matching(code, name_end, Self::OPEN, Self::CLOSE))
            .flatten()?;
        Some(closing + 1)
    }
}

pub(super) struct CssEscape;

impl CssEscape {
    const MAX_HEX_DIGITS: usize = 6;
    const WHITESPACE: &'static [u8] = b" \t\n\r\x0c";
    const REPLACEMENT: char = '\u{fffd}';

    pub(super) fn end(source: &str, start: usize) -> usize {
        let digits = Self::digits(source, start);
        let body_end = Self::body_end(source, start, digits);
        body_end + Self::trailing_whitespace(source, body_end, digits)
    }

    pub(super) fn value(source: &str, start: usize) -> char {
        let digits = Self::digits(source, start);
        Self::character(source, start, digits)
    }

    fn digits(source: &str, start: usize) -> usize {
        source.as_bytes()[start + 1..]
            .iter()
            .take(Self::MAX_HEX_DIGITS)
            .take_while(|byte| byte.is_ascii_hexdigit())
            .count()
    }

    fn body_end(source: &str, start: usize, digits: usize) -> usize {
        match digits {
            0 => start + 1 + source[start + 1..].chars().next().map_or(0, char::len_utf8),
            _ => start + 1 + digits,
        }
    }

    fn trailing_whitespace(source: &str, end: usize, digits: usize) -> usize {
        let first = source.as_bytes().get(end).copied();
        let whitespace = first.is_some_and(|byte| Self::WHITESPACE.contains(&byte));
        let crlf = (first == Some(b'\r')) & (source.as_bytes().get(end + 1) == Some(&b'\n'));
        usize::from((digits > 0) & whitespace) + usize::from((digits > 0) & crlf)
    }

    fn character(source: &str, start: usize, digits: usize) -> char {
        match digits {
            0 => source[start + 1..]
                .chars()
                .next()
                .unwrap_or(Self::REPLACEMENT),
            _ => u32::from_str_radix(&source[start + 1..start + 1 + digits], 16)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or(Self::REPLACEMENT),
        }
    }
}

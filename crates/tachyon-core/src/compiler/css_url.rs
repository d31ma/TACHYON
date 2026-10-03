use super::css_escape::CssEscape;

pub(super) struct CssUrl;

impl CssUrl {
    const NAME: &'static str = "url";
    const WHITESPACE: &'static [u8] = b" \t\n\r\x0c";
    const REPLACEMENT: char = '\u{fffd}';

    pub(super) fn token_end(
        source: &str,
        start: usize,
        name_end: usize,
    ) -> Result<usize, &'static str> {
        let name_start = Self::name_start(source, start, name_end);
        let name = Self::name(source, name_start, name_end);
        let is_url = name.eq_ignore_ascii_case(Self::NAME)
            & (source.as_bytes().get(name_end) == Some(&b'('));
        let content = Self::content_start(source, name_end);
        let url_end = Self::read_url(source, is_url, content)?;
        Ok(Self::selected_end(
            start, name_start, name_end, is_url, url_end,
        ))
    }

    fn selected_end(
        start: usize,
        name_start: usize,
        name_end: usize,
        is_url: bool,
        url_end: Option<usize>,
    ) -> usize {
        match (is_url, name_start > start, url_end) {
            (true, true, _) => name_start,
            (true, false, Some(end)) => end,
            _ => name_end,
        }
    }

    fn name_start(source: &str, start: usize, end: usize) -> usize {
        let mut name_start = start;
        let mut position = start;
        while position < end {
            (name_start, position) = Self::name_boundary(source, name_start, position);
        }
        name_start
    }

    fn name_boundary(source: &str, previous: usize, position: usize) -> (usize, usize) {
        let value = source[position..]
            .chars()
            .next()
            .unwrap_or(Self::REPLACEMENT);
        let next = position + value.len_utf8();
        match value {
            '\\' => (previous, CssEscape::end(source, position)),
            '#' | '@' => (position, next),
            value if Self::name_character(value) => (previous, next),
            _ => (next, next),
        }
    }

    fn name_character(value: char) -> bool {
        let ascii_character = Self::ascii_name_character(value);
        Self::allowed_name_character(value.is_ascii(), ascii_character)
    }

    fn ascii_name_character(value: char) -> bool {
        value.is_alphanumeric() || value == '_' || value == '-'
    }

    fn allowed_name_character(ascii: bool, ascii_character: bool) -> bool {
        !ascii || ascii_character
    }

    fn name(source: &str, start: usize, end: usize) -> String {
        let mut name = String::new();
        let mut position = start;
        while position < end {
            let (value, next) = Self::name_step(source, position);
            name.push(value);
            position = next;
        }
        name
    }

    fn name_step(source: &str, position: usize) -> (char, usize) {
        if source.as_bytes()[position] == b'\\' {
            (
                CssEscape::value(source, position),
                CssEscape::end(source, position),
            )
        } else {
            let value = source[position..]
                .chars()
                .next()
                .unwrap_or(Self::REPLACEMENT);
            (value, position + value.len_utf8())
        }
    }

    fn content_start(source: &str, name_end: usize) -> usize {
        let rest = source.get(name_end + 1..).unwrap_or_default();
        name_end
            + 1
            + rest
                .bytes()
                .take_while(|byte| Self::WHITESPACE.contains(byte))
                .count()
    }

    fn read_url(source: &str, is_url: bool, content: usize) -> Result<Option<usize>, &'static str> {
        match (is_url, source.as_bytes().get(content).copied()) {
            (true, Some(b'\'' | b'"')) => Ok(None),
            (true, _) => Self::scan(source, content).map(Some),
            _ => Ok(None),
        }
    }

    fn scan(source: &str, start: usize) -> Result<usize, &'static str> {
        let mut position = start;
        while let Some(next) = Self::step(source, position)? {
            position = next;
        }
        Ok(position + 1)
    }

    fn step(source: &str, position: usize) -> Result<Option<usize>, &'static str> {
        match (
            source.as_bytes().get(position).copied(),
            source.as_bytes().get(position + 1).copied(),
        ) {
            (None, _) => Err("Stylesheet has an unclosed URL."),
            (Some(b')'), _) => Ok(None),
            (Some(b'\\'), None) => Err("Stylesheet has an unclosed URL escape."),
            (Some(b'\\'), Some(b'\n' | b'\r' | b'\x0c')) => {
                Err("Stylesheet URL has an invalid escaped newline.")
            }
            (Some(b'\\'), Some(_)) => Ok(Some(CssEscape::end(source, position))),
            (Some(b'\'' | b'"' | b'(' | 0..=8 | 11 | 14..=31 | 127), _) => {
                Err("Stylesheet has invalid unquoted URL syntax.")
            }
            _ => Ok(Some(position + 1)),
        }
    }
}

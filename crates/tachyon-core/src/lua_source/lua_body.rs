use super::LuaLexer;
use std::ops::Range;

pub(super) struct LuaBody;

impl LuaBody {
    const WORDS: [(&'static str, &'static str); 4] = [
        ("elseif", "elsif"),
        ("repeat", "begin"),
        ("local", ""),
        ("function", "proc do"),
    ];
    const HIDDEN_IN_TEXT: &'static [u8] = b"#\\\"'";
    const REGION_KINDS: [fn(&mut [u8]); 3] = [Self::hide_comment, Self::hide_long, Self::hide_text];
    const UNTIL: &'static str = "until ";
    const COMMENT_START: char = '#';

    pub(super) fn sanitized(source: &str) -> String {
        let mask = LuaLexer::code_mask(source).code;
        let bytes = source.as_bytes();
        let mut output = bytes.to_vec();
        Self::clean_code(bytes, &mask, &mut output);
        Self::hide_runs(bytes, &Self::hidden_runs(&mask), &mut output);
        String::from_utf8(output).unwrap_or_default()
    }

    pub(super) fn translated(line: &str) -> String {
        let (code, comment) = line
            .split_once(Self::COMMENT_START)
            .map_or((line, String::new()), |(code, rest)| {
                (code, format!("#{rest}"))
            });
        let translated = format!("{}{comment}", Self::joined_parts(code));
        Self::until_line(&translated)
    }

    fn clean_code(bytes: &[u8], mask: &[bool], output: &mut [u8]) {
        for (at, visible) in mask.iter().enumerate() {
            output[at] = [output[at], Self::code_byte(bytes, at)][usize::from(*visible)];
        }
    }

    fn code_byte(bytes: &[u8], at: usize) -> u8 {
        match (bytes[at], bytes.get(at + 1)) {
            (b'~', Some(b'=')) => b'!',
            (b'#', _) => b' ',
            (b':', _) => b'.',
            (byte, _) => byte,
        }
    }

    fn hidden_runs(mask: &[bool]) -> Vec<Range<usize>> {
        let mut runs = Vec::new();
        let mut open = None;
        for (at, visible) in mask
            .iter()
            .copied()
            .chain(std::iter::once(true))
            .enumerate()
        {
            Self::track(&mut runs, &mut open, at, visible);
        }
        runs
    }

    fn track(runs: &mut Vec<Range<usize>>, open: &mut Option<usize>, at: usize, visible: bool) {
        match (visible, *open) {
            (false, None) => *open = Some(at),
            (true, Some(start)) => {
                runs.push(start..at);
                *open = None;
            }
            _ => {}
        }
    }

    fn hide_runs(bytes: &[u8], runs: &[Range<usize>], output: &mut [u8]) {
        for run in runs {
            Self::REGION_KINDS[Self::region_kind(bytes, run.start)](&mut output[run.clone()]);
        }
    }

    fn region_kind(bytes: &[u8], start: usize) -> usize {
        match bytes[start] {
            b'-' => 0,
            b'[' => 1,
            _ => 2,
        }
    }

    fn hide_comment(region: &mut [u8]) {
        Self::blank_keeping_lines(region);
        region[0] = b'#';
    }

    fn hide_long(region: &mut [u8]) {
        Self::blank_keeping_lines(region);
        Self::quote_ends(region);
    }

    fn hide_text(region: &mut [u8]) {
        for byte in region.iter_mut() {
            *byte = Self::text_byte(*byte);
        }
        Self::quote_ends(region);
    }

    fn blank_keeping_lines(region: &mut [u8]) {
        for byte in region.iter_mut() {
            *byte = b" \n"[usize::from(*byte == b'\n')];
        }
    }

    fn text_byte(byte: u8) -> u8 {
        let hidden = [Self::HIDDEN_IN_TEXT.contains(&byte), !byte.is_ascii()].contains(&true);
        [byte, b' '][usize::from(hidden)]
    }

    fn quote_ends(region: &mut [u8]) {
        let last = region.len() - 1;
        region[0] = b'"';
        region[last] = b'"';
    }

    fn joined_parts(code: &str) -> String {
        code.split('"')
            .enumerate()
            .map(|(index, part)| Self::part(index, part))
            .collect::<Vec<_>>()
            .join("\"")
    }

    fn part(index: usize, part: &str) -> String {
        [Self::code_words(part), part.to_owned()]
            .into_iter()
            .nth(index % 2)
            .unwrap_or_default()
    }

    fn code_words(code: &str) -> String {
        code.split_inclusive(Self::separates)
            .map(Self::piece)
            .collect()
    }

    fn separates(character: char) -> bool {
        ![character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn piece(piece: &str) -> String {
        let word = piece.trim_end_matches(Self::separates);
        let replaced = Self::keyword(word);
        let dropped = [replaced.is_empty(), !word.is_empty()] == [true, true];
        let rest = [&piece[word.len()..], ""][usize::from(dropped)];
        format!("{replaced}{rest}")
    }

    fn keyword(word: &str) -> &str {
        Self::WORDS
            .iter()
            .find(|(lua, _)| *lua == word)
            .map_or(word, |(_, ruby)| ruby)
    }

    fn until_line(line: &str) -> String {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        [line.to_owned(), format!("{indent}end {trimmed}")]
            .into_iter()
            .nth(usize::from(trimmed.starts_with(Self::UNTIL)))
            .unwrap_or_default()
    }
}

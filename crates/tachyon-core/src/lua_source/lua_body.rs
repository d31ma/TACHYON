use super::LuaLexer;

pub(super) struct LuaBody;

impl LuaBody {
    const WORDS: [(&'static str, &'static str); 3] =
        [("elseif", "elsif"), ("repeat", "begin"), ("local", "")];
    const HIDDEN_IN_TEXT: &'static [u8] = b"#\\\"'";
    const PROCEDURE: &'static str = "proc do";
    const FUNCTION: &'static str = "function";
    const UNTIL: &'static str = "until ";

    pub(super) fn sanitized(source: &str) -> String {
        let mask = LuaLexer::code_mask(source).code;
        let bytes = source.as_bytes();
        let mut output = bytes.to_vec();
        let mut at = 0;
        while at < bytes.len() {
            if mask[at] {
                output[at] = Self::code_byte(bytes, at);
                at += 1;
                continue;
            }
            let end = mask[at..]
                .iter()
                .position(|visible| *visible)
                .map_or(bytes.len(), |offset| at + offset);
            Self::hide(bytes, &mut output[at..end], at);
            at = end;
        }
        String::from_utf8(output).unwrap_or_default()
    }

    fn code_byte(bytes: &[u8], at: usize) -> u8 {
        match (bytes[at], bytes.get(at + 1)) {
            (b'~', Some(b'=')) => b'!',
            (b'#', _) => b' ',
            (b':', _) => b'.',
            (byte, _) => byte,
        }
    }

    fn hide(bytes: &[u8], region: &mut [u8], start: usize) {
        let first = bytes[start];
        let comment = first == b'-';
        let long = LuaLexer::long_level(bytes, start).is_some();
        for byte in region.iter_mut() {
            if *byte == b'\n' {
                continue;
            }
            if comment || Self::HIDDEN_IN_TEXT.contains(byte) || !byte.is_ascii() {
                *byte = b' ';
            }
        }
        if comment {
            region[0] = b'#';
            return;
        }
        if long {
            region.fill_with(|| b' ');
            Self::keep_newlines(bytes, region, start);
        }
        let last = region.len() - 1;
        region[0] = b'"';
        region[last] = b'"';
    }

    fn keep_newlines(bytes: &[u8], region: &mut [u8], start: usize) {
        for (offset, byte) in region.iter_mut().enumerate() {
            if bytes[start + offset] == b'\n' {
                *byte = b'\n';
            }
        }
    }

    pub(super) fn translated(line: &str) -> String {
        let code_end = line.find('#').unwrap_or(line.len());
        let (code, comment) = line.split_at(code_end);
        let mut output = String::new();
        for (index, part) in code.split('"').enumerate() {
            if index > 0 {
                output.push('"');
            }
            if index % 2 == 1 {
                output.push_str(part);
            } else {
                output.push_str(&Self::code_words(part));
            }
        }
        output.push_str(comment);
        Self::until_line(&output)
    }

    fn code_words(code: &str) -> String {
        let mut output = String::new();
        let mut rest = code;
        while let Some(start) = rest.find(|character: char| character.is_ascii_alphabetic()) {
            output.push_str(&rest[..start]);
            let word_length = rest[start..]
                .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .unwrap_or(rest.len() - start);
            let word = &rest[start..start + word_length];
            let after = &rest[start + word_length..];
            let joined = start > 0 && matches!(rest.as_bytes()[start - 1], b'_' | b'.');
            let (text, consumed) = Self::word(word, after, joined);
            output.push_str(&text);
            rest = &rest[start + word_length + consumed..];
        }
        output.push_str(rest);
        output
    }

    fn word(word: &str, after: &str, joined: bool) -> (String, usize) {
        if joined {
            return (word.to_owned(), 0);
        }
        if word == Self::FUNCTION {
            return Self::procedure(after);
        }
        let replaced = Self::WORDS
            .iter()
            .find(|(lua, _)| *lua == word)
            .map_or(word, |(_, ruby)| ruby);
        let skip = usize::from(replaced.is_empty() && after.starts_with(' '));
        (replaced.to_owned(), skip)
    }

    fn procedure(after: &str) -> (String, usize) {
        let trimmed = after.trim_start();
        let Some(close) = trimmed.find(')').filter(|_| trimmed.starts_with('(')) else {
            return (String::from(Self::PROCEDURE), 0);
        };
        let parameters = &trimmed[1..close];
        let consumed = after.len() - trimmed.len() + close + 1;
        (format!("{} |{parameters}|", Self::PROCEDURE), consumed)
    }

    fn until_line(line: &str) -> String {
        let trimmed = line.trim_start();
        if !trimmed.starts_with(Self::UNTIL) {
            return line.to_owned();
        }
        let indent = &line[..line.len() - trimmed.len()];
        format!("{indent}end {trimmed}")
    }
}

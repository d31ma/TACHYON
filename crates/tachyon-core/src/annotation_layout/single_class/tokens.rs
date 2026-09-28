use super::{Failure, Input};

pub(super) struct Token<'a> {
    pub(super) text: &'a str,
    pub(super) start: usize,
    end: usize,
    pub(super) literal: bool,
}

impl Token<'_> {
    pub(super) fn is_name(&self) -> bool {
        !self.literal
            && self
                .text
                .as_bytes()
                .first()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    }
}

pub(super) struct Tokens<'a> {
    pub(super) items: Vec<Token<'a>>,
    source: &'a str,
    extension: &'a str,
    error: Option<&'static str>,
}

impl<'a> Tokens<'a> {
    const MAX_TOKENS: usize = 262_144;

    pub(super) fn read(input: &Input<'a>) -> Result<Self, Failure> {
        let masked = input.masked_code()?;
        let mut prepared = input.source.as_bytes().to_vec();
        if matches!(input.extension, "js" | "ts") {
            super::super::mask_regex_literals(input.source, &mut prepared);
        }
        let prepared = String::from_utf8(prepared).unwrap_or_default();
        let lexical = crate::lexical::code_mask(input.extension, &prepared);
        let mut reader = TokenReader {
            source: input.source,
            extension: input.extension,
            masked: &masked,
            prepared: &prepared,
            code: &lexical.code,
            at: 0,
        };
        let mut items = Vec::new();
        while let Some(token) = reader.next() {
            if items.len() == Self::MAX_TOKENS {
                return Err(input.failure(
                    token.start,
                    "Source exceeds the bounded one class layout token limit",
                    "Split source into smaller role, model and shared modules.",
                ));
            }
            items.push(token);
        }
        Ok(Self {
            items,
            source: input.source,
            extension: input.extension,
            error: lexical.error,
        })
    }

    fn is_comment(extension: &str, text: &str) -> bool {
        if extension == "rb" {
            return text.starts_with('#') || text.starts_with("=begin");
        }
        text.starts_with("//")
            || text.starts_with("/*")
            || (matches!(extension, "py" | "php") && text.starts_with('#'))
    }

    fn comment_end(extension: &str, source: &str, start: usize) -> usize {
        if extension == "rb" && source[start..].starts_with("=begin") {
            let close = source[start..]
                .find("\n=end")
                .map_or(source.len(), |length| start + length + 1);
            return source[close..]
                .find('\n')
                .map_or(source.len(), |length| close + length);
        }
        if !source[start..].starts_with("/*") {
            return source[start..]
                .find('\n')
                .map_or(source.len(), |length| start + length);
        }
        let nested = matches!(extension, "rs" | "dart");
        let mut depth = 1_usize;
        let mut at = start + 2;
        while at + 1 < source.len() {
            let pair = &source.as_bytes()[at..at + 2];
            if pair == b"*/" {
                depth -= 1;
                if depth == 0 {
                    return at + 2;
                }
                at += 2;
            } else if nested && pair == b"/*" {
                depth += 1;
                at += 2;
            } else {
                at += 1;
            }
        }
        source.len()
    }

    fn literal_end(extension: &str, source: &str, start: usize, code: &[bool]) -> usize {
        if matches!(extension, "rb" | "groovy") {
            return Self::hidden_end(source, start, code);
        }
        let tail = &source[start..];
        if extension == "rs" && tail.starts_with('r') {
            let hashes = tail[1..].bytes().take_while(|byte| *byte == b'#').count();
            let close = format!("\"{}", "#".repeat(hashes));
            let body = start + hashes + 2;
            return source
                .get(body..)
                .and_then(|rest| rest.find(&close))
                .map_or(source.len(), |length| body + length + close.len());
        }
        let prefix = usize::from(
            (extension == "cs" && tail.starts_with("@\""))
                || (extension == "dart" && tail.starts_with('r')),
        );
        let quote = source.as_bytes()[start + prefix];
        if matches!(quote, b'\'' | b'"') {
            return Self::quoted_end(extension, source, start + prefix, quote);
        }
        Self::hidden_end(source, start, code)
    }

    fn hidden_end(source: &str, start: usize, code: &[bool]) -> usize {
        let mut at = start;
        while at < source.len() && !code[at] {
            at += 1;
        }
        at
    }

    fn quoted_end(extension: &str, source: &str, start: usize, quote: u8) -> usize {
        let bytes = source.as_bytes();
        let triple = matches!(extension, "py" | "kt" | "java" | "cs" | "dart")
            && bytes.get(start..start + 3) == Some(&[quote, quote, quote]);
        let delimiter = if triple { 3 } else { 1 };
        let verbatim = extension == "cs" && start > 0 && bytes[start - 1] == b'@';
        let raw = extension == "dart" && start > 0 && bytes[start - 1] == b'r';
        let mut at = start + delimiter;
        while at < bytes.len() {
            if !verbatim && !raw && bytes[at] == b'\\' {
                at = (at + 2).min(bytes.len());
                continue;
            }
            if verbatim && bytes.get(at..at + 2) == Some(&[quote, quote]) {
                at += 2;
                continue;
            }
            if bytes[at] == quote
                && (delimiter == 1 || bytes.get(at..at + 3) == Some(&[quote, quote, quote]))
            {
                return at + delimiter;
            }
            at += 1;
        }
        bytes.len()
    }

    pub(super) fn check_complete(&self, input: &Input<'_>) -> Result<(), Failure> {
        if let Some(error) = self.error {
            return Err(input.failure(0, format!("The one class layout cannot be checked: {error}"), "Close the source literal or comment so the compiler can validate every top-level declaration."));
        }
        Ok(())
    }

    pub(super) fn marker_start(&self, at: usize) -> bool {
        match self.extension {
            "rs" | "php" | "rb" => self.word(at) == "#" && self.word(at + 1) == "[",
            "cs" => self.word(at) == "[" && self.can_start_attribute(at),
            "py" => self.word(at) == "@" && self.new_line_before(at),
            "kt" => self.word(at) == "@" && !self.follows_label_keyword(at),
            _ => self.word(at) == "@",
        }
    }

    pub(super) fn marker_end(&self, at: usize) -> Option<usize> {
        self.marker_start(at).then(|| self.marker_span_end(at))
    }

    fn marker_span_end(&self, at: usize) -> usize {
        let end = match self.word(at) {
            "#" => self.after_group(at + 1),
            "[" => self.after_group(at),
            _ => Some(self.after_marker(at)),
        };
        end.unwrap_or(at + 1)
    }

    fn can_start_attribute(&self, at: usize) -> bool {
        if self.new_line_before(at) {
            return true;
        }
        matches!(self.word(at - 1), "]" | ";" | "{" | "}")
    }

    fn follows_label_keyword(&self, at: usize) -> bool {
        let Some(previous) = at.checked_sub(1).and_then(|index| self.items.get(index)) else {
            return false;
        };
        let touches = previous.end == self.items[at].start;
        touches
            && matches!(
                previous.text,
                "this" | "super" | "return" | "break" | "continue"
            )
    }

    fn word(&self, at: usize) -> &str {
        self.items.get(at).map_or("", |token| token.text)
    }

    pub(super) fn has_framework_marker(&self) -> bool {
        let marked = self.has_marker(|name| {
            crate::behavior::is_framework_marker(name) || super::Role::named(name).is_some()
        });
        marked || self.has_parameter_marker()
    }

    fn has_parameter_marker(&self) -> bool {
        for at in 1..self.items.len() {
            if !matches!(self.word(at - 1), "(" | ",") {
                continue;
            }
            let name_at = match self.word(at) {
                "@" | "[" => at + 1,
                "#" if self.word(at + 1) == "[" => at + 2,
                _ => continue,
            };
            let (name, _) = self.marker_path(name_at);
            if crate::behavior::is_parameter_marker(&name) {
                return true;
            }
        }
        false
    }

    pub(super) fn has_role_marker(&self, marker: &str) -> bool {
        self.has_marker(|name| super::Role::named(name).is_some_and(|role| role.marker == marker))
    }

    pub(super) fn has_test_marker(&self) -> bool {
        self.has_marker(|name| {
            let name = name
                .strip_prefix("yon::")
                .or_else(|| name.strip_prefix("tac::"))
                .unwrap_or(name);
            matches!(
                name.replace('_', "").to_ascii_lowercase().as_str(),
                "test" | "testsetup"
            )
        })
    }

    fn has_marker(&self, matches: impl Fn(&str) -> bool) -> bool {
        for at in 0..self.items.len() {
            if !self.marker_start(at) {
                continue;
            }
            let mut next = at + if self.word(at) == "#" { 2 } else { 1 };
            loop {
                let (name, end) = self.marker_path(next);
                let rust_test_attribute = self.extension == "rs" && name == "test";
                if !rust_test_attribute && matches(&name) {
                    return true;
                }
                next = end;
                if self.word(next) == "(" {
                    next = self.after_group(next).unwrap_or(self.items.len());
                }
                if self.word(at) == "@" || self.word(next) != "," {
                    break;
                }
                next += 1;
            }
        }
        false
    }

    pub(super) fn after_marker(&self, at: usize) -> usize {
        let (_, mut next) = self.marker_path(at + 1);
        if self.word(next) == "(" {
            next = self.after_group(next).unwrap_or(self.items.len());
        }
        next
    }

    pub(super) fn marker_arguments(&self, at: usize) -> Option<usize> {
        if !self.marker_start(at) || self.word(at) != "@" {
            return None;
        }
        let (_, next) = self.marker_path(at + 1);
        if self.word(next) == "(" {
            return Some(next);
        }
        None
    }

    fn marker_path(&self, at: usize) -> (String, usize) {
        if self.items.get(at).is_none_or(|token| !token.is_name()) {
            return (String::new(), at);
        }
        let mut next = at + 1;
        while matches!(self.word(next), ":" | "." | "\\") {
            while matches!(self.word(next), ":" | "." | "\\") {
                next += 1;
            }
            if self.items.get(next).is_none_or(|token| !token.is_name()) {
                break;
            }
            next += 1;
        }
        (
            self.items[at..next]
                .iter()
                .map(|token| token.text)
                .collect(),
            next,
        )
    }

    pub(super) fn new_line_before(&self, at: usize) -> bool {
        let Some(previous) = at.checked_sub(1).and_then(|index| self.items.get(index)) else {
            return true;
        };
        let Some(next) = self.items.get(at) else {
            return true;
        };
        self.source[previous.end..next.start].contains('\n')
    }

    pub(super) fn column(&self, at: usize) -> usize {
        let start = self.items[at].start;
        let line = self.source[..start]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        start - line
    }

    pub(super) fn after_group(&self, at: usize) -> Option<usize> {
        let close = Self::closing(self.word(at))?;
        let angles = self.word(at) == "<";
        let mut expected = vec![close];
        let mut next = at + 1;
        while next < self.items.len() {
            let word = self.word(next);
            if angles && word == ">" && matches!(self.word(next - 1), "-" | "=") {
                next += 1;
                continue;
            }
            if word == *expected.last()? {
                expected.pop();
                if expected.is_empty() {
                    return Some(next + 1);
                }
            } else if let Some(close) = Self::closing(word) {
                if word != "<" || (angles && !expected.contains(&"}")) {
                    expected.push(close);
                }
            } else if matches!(word, ")" | "]" | "}") {
                return None;
            }
            next += 1;
        }
        None
    }

    fn closing(word: &str) -> Option<&'static str> {
        match word {
            "(" => Some(")"),
            "[" => Some("]"),
            "{" => Some("}"),
            "<" => Some(">"),
            _ => None,
        }
    }
}

struct TokenReader<'a, 'b> {
    source: &'a str,
    extension: &'a str,
    masked: &'b str,
    prepared: &'b str,
    code: &'b [bool],
    at: usize,
}

impl<'a> TokenReader<'a, '_> {
    fn next(&mut self) -> Option<Token<'a>> {
        while self.at < self.source.len() {
            let start = self.at;
            if !self.code[start] {
                if self.extension == "go"
                    && crate::lexical::go_annotation_comment(self.source, start)
                    && self.code.get(start + 3) == Some(&true)
                {
                    self.at += 3;
                    continue;
                }
                if Tokens::is_comment(self.extension, &self.source[start..]) {
                    self.at = Tokens::comment_end(self.extension, self.source, start);
                    continue;
                }
                self.at = Tokens::literal_end(self.extension, self.source, start, self.code);
                return Some(self.token(start, true));
            }
            if self.is_regex(start) {
                while self.at < self.source.len() && self.prepared.as_bytes()[self.at] == b' ' {
                    self.at += 1;
                }
                return Some(self.token(start, true));
            }
            if self.masked.as_bytes()[start].is_ascii_whitespace() {
                self.at += 1;
                continue;
            }
            self.read_code();
            return Some(self.token(start, false));
        }
        None
    }

    fn is_regex(&self, at: usize) -> bool {
        self.prepared.as_bytes()[at] == b' ' && !self.source.as_bytes()[at].is_ascii_whitespace()
    }

    fn read_code(&mut self) {
        let byte = self.masked.as_bytes()[self.at];
        self.at += self.source[self.at..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);
        if !byte.is_ascii_alphanumeric() && byte != b'_' {
            return;
        }
        while self.at < self.source.len() {
            let next = self.masked.as_bytes()[self.at];
            if next.is_ascii_alphanumeric() || matches!(next, b'_' | b'$') || !next.is_ascii() {
                self.at += 1;
            } else {
                return;
            }
        }
    }

    fn token(&self, start: usize, literal: bool) -> Token<'a> {
        Token {
            text: &self.source[start..self.at],
            start,
            end: self.at,
            literal,
        }
    }
}

use crate::lexical::LexicalMask;
use std::ops::Range;

pub(crate) struct LuaLexer<'a> {
    bytes: &'a [u8],
    code: Vec<bool>,
}

type LexicalError = (usize, &'static str);

impl<'a> LuaLexer<'a> {
    const UNTERMINATED_STRING: &'static str = "Unterminated Lua string";
    const UNTERMINATED_LONG: &'static str = "Unterminated Lua long string or long comment";

    pub(crate) fn code_mask(source: &'a str) -> LexicalMask {
        let mut lexer = Self {
            bytes: source.as_bytes(),
            code: vec![true; source.len()],
        };
        let error = lexer.scan().err();
        lexer.finished(error)
    }

    pub(crate) fn long_level(bytes: &[u8], at: usize) -> Option<usize> {
        (bytes.get(at) == Some(&b'['))
            .then(|| Self::level_after(bytes, at))
            .flatten()
    }

    fn finished(self, error: Option<LexicalError>) -> LexicalMask {
        let mut code = self.code;
        let error = error.map(|(start, message)| {
            code[start..].fill(false);
            message
        });
        LexicalMask {
            code,
            error,
            marker_problem: None,
        }
    }

    fn scan(&mut self) -> Result<(), LexicalError> {
        let mut at = 0;
        while at < self.bytes.len() {
            let hidden = self.hidden_range(at)?;
            self.code[hidden.clone()].fill(false);
            at = hidden.end.max(at + 1);
        }
        Ok(())
    }

    fn hidden_range(&self, at: usize) -> Result<Range<usize>, LexicalError> {
        Ok(at..self.region_end(at)?.unwrap_or(at))
    }

    fn region_end(&self, at: usize) -> Result<Option<usize>, LexicalError> {
        match self.bytes[at] {
            b'-' => self.comment_at(at),
            b'"' | b'\'' => self.quoted(at),
            b'[' => self.long_string_at(at),
            _ => Ok(None),
        }
    }

    fn comment_at(&self, start: usize) -> Result<Option<usize>, LexicalError> {
        (self.bytes.get(start + 1) == Some(&b'-'))
            .then(|| self.comment(start))
            .transpose()
    }

    fn comment(&self, start: usize) -> Result<usize, LexicalError> {
        let body = start + 2;
        Self::long_level(self.bytes, body)
            .map_or_else(|| Ok(self.line_end(body)), |_| self.long(start, body))
    }

    fn long_string_at(&self, start: usize) -> Result<Option<usize>, LexicalError> {
        Self::long_level(self.bytes, start)
            .map(|_| self.long(start, start))
            .transpose()
    }

    fn line_end(&self, body: usize) -> usize {
        self.bytes[body..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(self.bytes.len(), |offset| body + offset)
    }

    fn quoted(&self, start: usize) -> Result<Option<usize>, LexicalError> {
        self.closing_quote(start)
            .map_or(Err((start, Self::UNTERMINATED_STRING)), |end| Ok(Some(end)))
    }

    fn closing_quote(&self, start: usize) -> Option<usize> {
        let quote = self.bytes[start];
        let mut escaped = false;
        let offset = self.bytes[start + 1..]
            .iter()
            .position(|byte| Self::quote_stop(*byte, quote, &mut escaped))?;
        let end = start + 1 + offset;
        (self.bytes[end] == quote).then_some(end + 1)
    }

    fn quote_stop(byte: u8, quote: u8, escaped: &mut bool) -> bool {
        let closes = [!*escaped, byte == quote] == [true, true];
        *escaped = [!*escaped, byte == b'\\'] == [true, true];
        [closes, byte == b'\n'].contains(&true)
    }

    fn long(&self, start: usize, open: usize) -> Result<usize, LexicalError> {
        let level = Self::long_level(self.bytes, open).unwrap_or_default();
        let closing = format!("]{}]", "=".repeat(level));
        let body = (open + level + 2).min(self.bytes.len());
        self.bytes[body..]
            .windows(closing.len())
            .position(|window| window == closing.as_bytes())
            .map_or(Err((start, Self::UNTERMINATED_LONG)), |offset| {
                Ok(body + offset + closing.len())
            })
    }

    fn level_after(bytes: &[u8], at: usize) -> Option<usize> {
        let level = bytes[at + 1..]
            .iter()
            .take_while(|byte| **byte == b'=')
            .count();
        (bytes.get(at + 1 + level) == Some(&b'[')).then_some(level)
    }
}

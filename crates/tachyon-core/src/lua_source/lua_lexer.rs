use crate::lexical::LexicalMask;

pub(crate) struct LuaLexer<'a> {
    bytes: &'a [u8],
    code: Vec<bool>,
}

impl<'a> LuaLexer<'a> {
    const UNTERMINATED_STRING: &'static str = "Unterminated Lua string";
    const UNTERMINATED_LONG: &'static str = "Unterminated Lua long string or long comment";

    pub(crate) fn code_mask(source: &'a str) -> LexicalMask {
        let mut lexer = Self {
            bytes: source.as_bytes(),
            code: vec![true; source.len()],
        };
        let error = lexer.scan().err();
        let mut code = lexer.code;
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

    fn scan(&mut self) -> Result<(), (usize, &'static str)> {
        let mut at = 0;
        while at < self.bytes.len() {
            let end = match self.bytes[at] {
                b'-' if self.bytes.get(at + 1) == Some(&b'-') => self.comment(at)?,
                b'"' | b'\'' => self.quoted(at)?,
                b'[' if Self::long_level(self.bytes, at).is_some() => self.long(at, at)?,
                _ => {
                    at += 1;
                    continue;
                }
            };
            self.code[at..end].fill(false);
            at = end;
        }
        Ok(())
    }

    fn comment(&self, start: usize) -> Result<usize, (usize, &'static str)> {
        let body = start + 2;
        if Self::long_level(self.bytes, body).is_some() {
            return self.long(start, body);
        }
        Ok(self.bytes[body..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(self.bytes.len(), |offset| body + offset))
    }

    fn quoted(&self, start: usize) -> Result<usize, (usize, &'static str)> {
        let quote = self.bytes[start];
        let mut at = start + 1;
        while let Some(byte) = self.bytes.get(at) {
            match *byte {
                b'\\' => at += 2,
                b'\n' => break,
                byte if byte == quote => return Ok(at + 1),
                _ => at += 1,
            }
        }
        Err((start, Self::UNTERMINATED_STRING))
    }

    fn long(&self, start: usize, open: usize) -> Result<usize, (usize, &'static str)> {
        let level = Self::long_level(self.bytes, open).unwrap_or_default();
        let closing = format!("]{}]", "=".repeat(level));
        let body = open + level + 2;
        let text = &self.bytes[body.min(self.bytes.len())..];
        text.windows(closing.len())
            .position(|window| window == closing.as_bytes())
            .map(|offset| body + offset + closing.len())
            .ok_or((start, Self::UNTERMINATED_LONG))
    }

    pub(crate) fn long_level(bytes: &[u8], at: usize) -> Option<usize> {
        if bytes.get(at) != Some(&b'[') {
            return None;
        }
        let level = bytes[at + 1..].iter().take_while(|byte| **byte == b'=').count();
        (bytes.get(at + 1 + level) == Some(&b'[')).then_some(level)
    }
}

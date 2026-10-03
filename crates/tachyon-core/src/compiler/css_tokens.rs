use super::css_escape::CssEscape;
use super::css_token::CssToken;
use super::css_url::CssUrl;

pub(super) struct CssTokens<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> CssTokens<'a> {
    const PUNCTUATION: &'static [u8] = b"{};:(),[]!";
    const MAX_TOKENS: usize = 262_144;

    pub(super) fn read(source: &'a str) -> Result<Vec<CssToken<'a>>, &'static str> {
        let mut reader = Self {
            source,
            position: 0,
        };
        let mut tokens = Vec::new();
        reader.read_all(&mut tokens)?;
        Ok(tokens)
    }

    fn read_all(&mut self, tokens: &mut Vec<CssToken<'a>>) -> Result<(), &'static str> {
        while self.position < self.source.len() {
            self.add_next(tokens)?;
        }
        Ok(())
    }

    fn add_next(&mut self, tokens: &mut Vec<CssToken<'a>>) -> Result<(), &'static str> {
        self.check_count(tokens.len())?;
        tokens.extend(self.next()?);
        Ok(())
    }

    fn check_count(&self, count: usize) -> Result<(), &'static str> {
        if count >= Self::MAX_TOKENS {
            return Err("Stylesheet exceeds the bounded token limit.");
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Option<CssToken<'a>>, &'static str> {
        let bytes = self.source.as_bytes();
        let start = self.position;
        match (bytes[start], bytes.get(start + 1).copied()) {
            (b'/', Some(b'*')) => {
                self.skip_comment()?;
                Ok(None)
            }
            (b'\\', Some(b'\n' | b'\r' | b'\x0c')) => {
                self.position += 1;
                Ok(Some(self.token(start)))
            }
            (byte, _) if byte.is_ascii_whitespace() => {
                self.position += 1;
                Ok(None)
            }
            (b'\'' | b'"', _) => {
                self.position = self.quoted_end(start)?;
                Ok(Some(self.token(start)))
            }
            (byte, _) if Self::PUNCTUATION.contains(&byte) => {
                self.position += 1;
                Ok(Some(self.token(start)))
            }
            _ => {
                let name_end = self.word_end(start);
                self.position = CssUrl::token_end(self.source, start, name_end)?;
                Ok(Some(self.token(start)))
            }
        }
    }

    fn token(&self, start: usize) -> CssToken<'a> {
        CssToken {
            text: &self.source[start..self.position],
            start,
            end: self.position,
        }
    }

    fn skip_comment(&mut self) -> Result<(), &'static str> {
        let length = self.source[self.position + 2..]
            .find("*/")
            .ok_or("Stylesheet has an unclosed comment.")?;
        self.position += length + 4;
        Ok(())
    }

    fn quoted_end(&self, start: usize) -> Result<usize, &'static str> {
        let mut position = start + 1;
        while let Some(next) = self.quoted_step(position, self.source.as_bytes()[start])? {
            position = next;
        }
        Ok(position + 1)
    }

    fn quoted_step(&self, position: usize, quote: u8) -> Result<Option<usize>, &'static str> {
        match self.source.as_bytes().get(position).copied() {
            None => Err("Stylesheet has an unclosed string."),
            Some(byte) if byte == quote => Ok(None),
            Some(b'\\') => Ok(Some(self.quoted_escape_end(position))),
            Some(b'\n' | b'\r' | b'\x0c') => {
                Err("Stylesheet has an unescaped newline in a string.")
            }
            Some(_) => Ok(Some(position + 1)),
        }
    }

    fn quoted_escape_end(&self, position: usize) -> usize {
        let crlf = (self.source.as_bytes().get(position + 1) == Some(&b'\r'))
            & (self.source.as_bytes().get(position + 2) == Some(&b'\n'));
        (position + 2 + usize::from(crlf)).min(self.source.len())
    }

    fn word_end(&self, start: usize) -> usize {
        let mut position = start;
        while let Some(next) = self.word_step(position) {
            position = next;
        }
        position
    }

    fn word_step(&self, position: usize) -> Option<usize> {
        match (
            self.source.as_bytes().get(position).copied(),
            self.source.as_bytes().get(position + 1).copied(),
        ) {
            (None, _) => None,
            (Some(b'\\'), Some(b'\n' | b'\r' | b'\x0c')) => None,
            (Some(b'\\'), Some(_)) => Some(CssEscape::end(self.source, position)),
            (Some(b'/'), Some(b'*')) => None,
            (Some(byte), _) if byte.is_ascii_whitespace() || Self::PUNCTUATION.contains(&byte) => {
                None
            }
            (Some(_), _) => Some(position + 1),
        }
    }
}

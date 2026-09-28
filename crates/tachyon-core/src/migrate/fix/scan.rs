pub(super) struct SourceText<'a> {
    pub(super) source: &'a str,
    pub(super) bytes: &'a [u8],
    code: Vec<bool>,
}

impl<'a> SourceText<'a> {
    pub(super) fn read(extension: &str, source: &'a str) -> Option<Self> {
        let mask = crate::lexical::code_mask(extension, source);
        if mask.error.is_some() {
            return None;
        }
        Some(Self {
            source,
            bytes: source.as_bytes(),
            code: mask.code,
        })
    }

    pub(super) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(super) fn is_code(&self, at: usize) -> bool {
        self.code.get(at).copied().unwrap_or(false)
    }

    pub(super) fn is_word_byte(&self, at: usize) -> bool {
        if !self.is_code(at) {
            return false;
        }
        let byte = self.bytes[at];
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
    }

    pub(super) fn starts_word(&self, at: usize) -> bool {
        self.is_word_byte(at) && (at == 0 || !self.is_word_byte(at - 1))
    }

    pub(super) fn word_end(&self, start: usize) -> usize {
        let mut end = start;
        while self.is_word_byte(end) {
            end += 1;
        }
        end
    }

    pub(super) fn word_start(&self, end: usize) -> usize {
        let mut start = end;
        while start > 0 && self.is_word_byte(start - 1) {
            start -= 1;
        }
        start
    }

    pub(super) fn code_words(&self, word: &str) -> Vec<usize> {
        let mut positions = Vec::new();
        for (at, _) in self.source.match_indices(word) {
            if self.starts_word(at) && self.word_end(at) == at + word.len() {
                positions.push(at);
            }
        }
        positions
    }

    pub(super) fn next_code(&self, from: usize) -> Option<usize> {
        let mut at = from;
        while at < self.len() {
            if self.is_code(at) && !self.bytes[at].is_ascii_whitespace() {
                return Some(at);
            }
            at += 1;
        }
        None
    }

    pub(super) fn previous_code(&self, before: usize) -> Option<usize> {
        let mut at = before;
        while at > 0 {
            at -= 1;
            if self.is_code(at) && !self.bytes[at].is_ascii_whitespace() {
                return Some(at);
            }
        }
        None
    }

    pub(super) fn previous_word(&self, before: usize) -> &'a str {
        let Some(last) = self.previous_code(before) else {
            return "";
        };
        if !self.is_word_byte(last) {
            return "";
        }
        &self.source[self.word_start(last + 1)..=last]
    }

    pub(super) fn matching(&self, open: usize) -> Option<usize> {
        let open_byte = self.bytes[open];
        let close_byte = match open_byte {
            b'(' => b')',
            b'[' => b']',
            _ => b'}',
        };
        let mut depth = 0_usize;
        for at in open..self.len() {
            if !self.is_code(at) {
                continue;
            }
            if self.bytes[at] == open_byte {
                depth += 1;
            } else if self.bytes[at] == close_byte {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
        }
        None
    }

    pub(super) fn is_balanced(&self, start: usize, end: usize) -> bool {
        let mut depth = 0_i64;
        for at in start..end {
            if !self.is_code(at) {
                continue;
            }
            match self.bytes[at] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
        depth == 0
    }

    pub(super) fn line_start(&self, at: usize) -> usize {
        self.source[..at]
            .rfind('\n')
            .map_or(0, |newline| newline + 1)
    }

    pub(super) fn line_end(&self, at: usize) -> usize {
        self.source[at..]
            .find('\n')
            .map_or(self.len(), |newline| at + newline + 1)
    }

    pub(super) fn line_text(&self, line_start: usize) -> &'a str {
        let end = self.line_end(line_start);
        self.source[line_start..end].trim_end_matches(['\n', '\r'])
    }

    pub(super) fn is_blank_line(&self, line_start: usize) -> bool {
        line_start < self.len() && self.line_text(line_start).trim().is_empty()
    }

    pub(super) fn previous_line_start(&self, line_start: usize) -> Option<usize> {
        if line_start == 0 {
            return None;
        }
        Some(self.line_start(line_start - 1))
    }

    pub(super) fn indentation(&self, line_start: usize) -> usize {
        let text = self.line_text(line_start);
        text.len() - text.trim_start().len()
    }

    pub(super) fn line_number(&self, at: usize) -> usize {
        self.source[..at].matches('\n').count() + 1
    }
}

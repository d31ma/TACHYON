pub(super) struct SourceText<'a> {
    pub(super) source: &'a str,
    pub(super) bytes: &'a [u8],
    code: Vec<bool>,
}

impl<'a> SourceText<'a> {
    pub(super) fn read(extension: &str, source: &'a str) -> Option<Self> {
        let mask = crate::lexical::code_mask(extension, source);
        let readable = mask.error.is_none();
        readable.then_some(Self {
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

    fn is_word_character(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'$'].contains(&true)
    }

    pub(super) fn is_word_byte(&self, at: usize) -> bool {
        let next = self.bytes.get(at).copied();
        let word = next.is_some_and(Self::is_word_character);
        [self.is_code(at), word] == [true, true]
    }

    pub(super) fn starts_word(&self, at: usize) -> bool {
        let before = at.checked_sub(1);
        let follows_word = before.is_some_and(|before| self.is_word_byte(before));
        [self.is_word_byte(at), follows_word] == [true, false]
    }

    pub(super) fn word_end(&self, start: usize) -> usize {
        let end = (start..self.len()).find(|at| !self.is_word_byte(*at));
        end.unwrap_or(self.len().max(start))
    }

    pub(super) fn word_start(&self, end: usize) -> usize {
        let before_word = (0..end).rev().find(|at| !self.is_word_byte(*at));
        before_word.map_or(0, |at| at + 1)
    }

    fn is_whole_word(&self, at: usize, length: usize) -> bool {
        [self.starts_word(at), self.word_end(at) == at + length] == [true, true]
    }

    pub(super) fn code_words(&self, word: &str) -> Vec<usize> {
        self.source
            .match_indices(word)
            .map(|(at, _)| at)
            .filter(|at| self.is_whole_word(*at, word.len()))
            .collect()
    }

    fn is_visible_code(&self, at: usize) -> bool {
        let next = self.bytes.get(at);
        let visible = next.is_some_and(|byte| !byte.is_ascii_whitespace());
        [self.is_code(at), visible] == [true, true]
    }

    pub(super) fn next_code(&self, from: usize) -> Option<usize> {
        (from..self.len()).find(|at| self.is_visible_code(*at))
    }

    pub(super) fn previous_code(&self, before: usize) -> Option<usize> {
        (0..before).rev().find(|at| self.is_visible_code(*at))
    }

    pub(super) fn previous_word(&self, before: usize) -> &'a str {
        let last = self.previous_code(before);
        let last_word_byte = last.filter(|last| self.is_word_byte(*last));
        last_word_byte.map_or("", |last| &self.source[self.word_start(last + 1)..=last])
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
        let inside = line_start < self.len();
        let text = inside.then(|| self.line_text(line_start));
        text.is_some_and(|text| text.trim().is_empty())
    }

    pub(super) fn previous_line_start(&self, line_start: usize) -> Option<usize> {
        let before = line_start.checked_sub(1);
        before.map(|before| self.line_start(before))
    }

    pub(super) fn indentation(&self, line_start: usize) -> usize {
        let text = self.line_text(line_start);
        text.len() - text.trim_start().len()
    }

    pub(super) fn line_number(&self, at: usize) -> usize {
        self.source[..at].matches('\n').count() + 1
    }
}

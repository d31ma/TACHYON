use crate::ascii_name::AsciiName;

#[derive(Clone, Copy, Debug)]
pub(super) struct Piece<'a> {
    pub(super) at: usize,
    pub(super) text: &'a str,
}

pub(super) struct Body<'a> {
    pub(super) head: Piece<'a>,
    pub(super) inner: Piece<'a>,
    pub(super) rest: Piece<'a>,
}

impl<'a> Piece<'a> {
    const CONTINUED_ENDINGS: [&'static str; 7] = ["=", ",", ":", "->", "&", "|", "<"];
    const CONTINUED_STARTS: [&'static str; 8] = [":", ".", "{", "=", "->", "&", "|", ">"];

    pub(super) fn new(at: usize, text: &'a str) -> Self {
        Self { at, text }
    }

    pub(super) fn slice(self, start: usize, end: usize) -> Self {
        Self::new(self.at + start, &self.text[start..end])
    }

    pub(super) fn from(self, start: usize) -> Self {
        self.slice(start, self.text.len())
    }

    pub(super) fn trim(self) -> Self {
        let trimmed = self.text.trim_start();
        let start = self.text.len() - trimmed.len();
        Self::new(self.at + start, trimmed.trim_end())
    }

    pub(super) fn starts_with(self, prefix: &str) -> bool {
        self.text.starts_with(prefix)
    }

    pub(super) fn ends_with(self, suffix: &str) -> bool {
        self.text.ends_with(suffix)
    }

    pub(super) fn after(self, prefix: &str) -> Option<Self> {
        if !self.starts_with(prefix) {
            return None;
        }
        Some(self.from(prefix.len()).trim())
    }

    pub(super) fn after_word(self, word: &str) -> Option<Self> {
        if self.first_word() != word {
            return None;
        }
        Some(self.from(word.len()).trim())
    }

    pub(super) fn after_words(self, words: &[&str]) -> Self {
        let mut piece = self;
        while words.contains(&piece.first_word()) {
            piece = piece.from(piece.first_word().len()).trim();
        }
        piece
    }

    pub(super) fn first_word(self) -> &'a str {
        let length = self
            .text
            .bytes()
            .take_while(|byte| AsciiName::is_name_byte(*byte))
            .count();
        &self.text[..length]
    }

    pub(super) fn path(self) -> &'a str {
        let text = self.text.trim_start();
        let length = text
            .bytes()
            .take_while(|byte| AsciiName::is_name_byte(*byte) || matches!(byte, b'.' | b':'))
            .count();
        &text[..length]
    }

    pub(super) fn last_segment(path: &str) -> &str {
        path.rsplit(['.', ':']).next().unwrap_or(path)
    }

    pub(super) fn depth_after(byte: u8, depth: usize) -> usize {
        match byte {
            b'(' | b'[' | b'{' => depth + 1,
            b')' | b']' | b'}' => depth.saturating_sub(1),
            _ => depth,
        }
    }

    pub(super) fn closing(self, open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for (index, byte) in self.text.bytes().enumerate().skip(open) {
            depth = Self::depth_after(byte, depth);
            if depth == 0 {
                return Some(index);
            }
        }
        None
    }

    pub(super) fn after_group(self) -> Option<Self> {
        let close = self.closing(0)?;
        Some(self.from(close + 1).trim())
    }

    pub(super) fn is_empty_or_group(self) -> bool {
        if self.text.is_empty() {
            return true;
        }
        if !self.starts_with("(") {
            return false;
        }
        self.after_group().is_some_and(|rest| rest.text.is_empty())
    }

    pub(super) fn after_generics(self) -> Option<Self> {
        if !self.starts_with("<") {
            return Some(self);
        }
        let mut depth = 0_usize;
        for (index, byte) in self.text.bytes().enumerate() {
            if byte == b'<' {
                depth += 1;
            }
            if byte == b'>' {
                depth -= 1;
            }
            if depth == 0 {
                return Some(self.from(index + 1).trim());
            }
        }
        None
    }

    pub(super) fn find_top(self, targets: &[u8]) -> Option<usize> {
        let mut depth = 0_usize;
        for (index, byte) in self.text.bytes().enumerate() {
            if depth == 0 && targets.contains(&byte) {
                return Some(index);
            }
            depth = Self::depth_after(byte, depth);
        }
        None
    }

    pub(super) fn find_top_word(self, word: &str) -> Option<usize> {
        let mut depth = 0_usize;
        for (index, byte) in self.text.bytes().enumerate() {
            if depth == 0 && self.word_starts_at(index, word) {
                return Some(index);
            }
            depth = Self::depth_after(byte, depth);
        }
        None
    }

    pub(super) fn has_top_word(self, word: &str) -> bool {
        self.find_top_word(word).is_some()
    }

    fn word_starts_at(self, index: usize, word: &str) -> bool {
        let bytes = self.text.as_bytes();
        if !bytes[index..].starts_with(word.as_bytes()) {
            return false;
        }
        let before = index.checked_sub(1).map(|previous| bytes[previous]);
        let after = bytes.get(index + word.len()).copied();
        !before.is_some_and(AsciiName::is_name_byte) && !after.is_some_and(AsciiName::is_name_byte)
    }

    pub(super) fn split_top(self, separator: u8) -> Vec<Self> {
        let mut parts = Vec::new();
        let mut depth = 0_usize;
        let mut start = 0;
        for (index, byte) in self.text.bytes().enumerate() {
            if depth == 0 && byte == separator {
                parts.push(self.slice(start, index).trim());
                start = index + 1;
            }
            depth = Self::depth_after(byte, depth);
        }
        parts.push(self.from(start).trim());
        parts
    }

    pub(super) fn split_first_top(self, separator: u8) -> (Self, Self) {
        let Some(index) = self.find_top(&[separator]) else {
            return (self, self.from(self.text.len()));
        };
        (self.slice(0, index).trim(), self.from(index + 1).trim())
    }

    pub(super) fn body(self) -> Option<Body<'a>> {
        let open = self.find_top(b"{")?;
        let close = self.closing(open)?;
        Some(Body {
            head: self.slice(0, open).trim(),
            inner: self.slice(open + 1, close).trim(),
            rest: self.from(close + 1).trim(),
        })
    }

    pub(super) fn items(self) -> Vec<Self> {
        let mut items = Vec::new();
        let mut depth = 0_usize;
        let mut start = 0;
        for (index, byte) in self.text.bytes().enumerate() {
            depth = Self::depth_after(byte, depth);
            let closes_block = byte == b'}' && !self.block_continues(index);
            if depth == 0 && (byte == b';' || closes_block) {
                self.push_part(&mut items, start, index + 1);
                start = index + 1;
            }
        }
        self.push_part(&mut items, start, self.text.len());
        items
    }

    fn block_continues(self, index: usize) -> bool {
        let rest = self.text[index + 1..].trim_start();
        rest.starts_with([';', '='])
    }

    pub(super) fn statements(self) -> Vec<Self> {
        let mut statements = Vec::new();
        let mut depth = 0_usize;
        let mut start = 0;
        for (index, byte) in self.text.bytes().enumerate() {
            depth = Self::depth_after(byte, depth);
            if depth > 0 {
                continue;
            }
            let line_ends = byte == b'\n' && !self.statement_continues(start, index);
            if byte == b';' || line_ends {
                self.push_part(&mut statements, start, index);
                start = index + 1;
            }
        }
        self.push_part(&mut statements, start, self.text.len());
        statements
    }

    fn statement_continues(self, start: usize, index: usize) -> bool {
        let before = self.text[start..index].trim_end();
        if before.is_empty() {
            return false;
        }
        let after = Self::new(0, self.text[index + 1..].trim_start());
        for ending in Self::CONTINUED_ENDINGS {
            if before.ends_with(ending) {
                return true;
            }
        }
        for beginning in Self::CONTINUED_STARTS {
            if after.starts_with(beginning) {
                return true;
            }
        }
        after.first_word() == "where"
    }

    fn push_part(self, parts: &mut Vec<Self>, start: usize, end: usize) {
        let part = self.slice(start, end).trim();
        if !part.text.is_empty() {
            parts.push(part);
        }
    }

    pub(super) fn after_annotations(self) -> Self {
        let mut piece = self;
        while let Some(annotation) = piece.after("@") {
            let name = annotation.path();
            piece = annotation.from(name.len()).trim().after_arguments();
        }
        piece
    }

    pub(super) fn after_arguments(self) -> Self {
        if !self.starts_with("(") {
            return self;
        }
        self.after_group().unwrap_or(self)
    }

    pub(super) fn first_framework_annotation(self) -> Option<usize> {
        for (index, _) in self.text.match_indices('@') {
            let name = self.from(index + 1).path();
            if crate::behavior::is_framework_marker(Self::last_segment(name)) {
                return Some(self.at + index);
            }
        }
        None
    }
}

impl Body<'_> {
    pub(super) fn has_plain_end(&self) -> bool {
        self.rest.text.is_empty() || self.rest.text == ";"
    }
}

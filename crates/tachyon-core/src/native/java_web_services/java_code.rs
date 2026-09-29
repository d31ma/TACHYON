use super::java_type_declaration::JavaTypeDeclaration;
use std::ops::Range;

pub(super) struct JavaCode {
    visible: Vec<u8>,
    braces: Vec<i32>,
    parens: Vec<i32>,
}

impl JavaCode {
    const LANGUAGE: &'static str = "java";
    const OPEN: u8 = b'{';
    const CLOSE: u8 = b'}';
    const STATEMENT_END: u8 = b';';
    const OPEN_ARGUMENTS: u8 = b'(';
    const CLOSE_ARGUMENTS: u8 = b')';
    const BLANK: u8 = b' ';
    const TYPE_WORDS: [&'static str; 4] = ["class", "interface", "enum", "record"];
    const NOT_BEFORE_TYPE_WORD: [u8; 2] = *b"@.";

    pub(super) fn new(source: &str) -> Self {
        let visible = Self::visible(source);
        let braces = Self::depths(&visible, Self::OPEN, Self::CLOSE);
        let parens = Self::depths(&visible, Self::OPEN_ARGUMENTS, Self::CLOSE_ARGUMENTS);
        Self {
            visible,
            braces,
            parens,
        }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        &self.visible
    }

    pub(super) fn text(&self, range: &Range<usize>) -> String {
        String::from_utf8_lossy(&self.visible[range.clone()]).into_owned()
    }

    pub(super) fn is_open_arguments(&self, at: usize) -> bool {
        self.visible.get(at) == Some(&Self::OPEN_ARGUMENTS)
    }

    pub(super) fn closing_parenthesis(&self, open: usize) -> Option<usize> {
        Self::closing(&self.parens, open)
    }

    pub(super) fn ends_word(&self, end: usize) -> bool {
        let next = self.visible.get(end).copied().unwrap_or(Self::BLANK);
        !Self::is_name_byte(next)
    }

    pub(super) fn declaration(&self, keyword: &str, name: &str) -> Option<usize> {
        let wanted = format!("{keyword} {name}");
        let text = String::from_utf8_lossy(&self.visible);
        text.match_indices(&wanted)
            .map(|(at, _)| at + wanted.len())
            .find(|end| self.ends_word(*end))
    }

    pub(super) fn body_after(&self, declared: usize) -> Option<Range<usize>> {
        let open = self.next(declared, Self::OPEN)?;
        let close = Self::closing(&self.braces, open)?;
        Some(open + 1..close)
    }

    pub(super) fn next(&self, from: usize, wanted: u8) -> Option<usize> {
        (from..self.visible.len()).find(|at| self.visible[*at] == wanted)
    }

    pub(super) fn member_headers(&self, body: &Range<usize>) -> Vec<Range<usize>> {
        self.headers_at(body, self.braces[body.start])
    }

    pub(super) fn top_level_types(&self) -> Vec<JavaTypeDeclaration> {
        let mut found: Vec<JavaTypeDeclaration> = Self::TYPE_WORDS
            .iter()
            .flat_map(|word| self.declarations_of(word))
            .collect();
        found.sort_by_key(|declaration| declaration.keyword_at);
        found
    }

    pub(super) fn statement_end_before(&self, at: usize) -> usize {
        (0..at)
            .rev()
            .find(|index| self.ends_top_statement(*index))
            .map_or(0, |index| index + 1)
    }

    fn ends_top_statement(&self, at: usize) -> bool {
        (self.braces[at], self.visible[at]) == (0, Self::STATEMENT_END)
    }

    fn declarations_of(&self, word: &'static str) -> Vec<JavaTypeDeclaration> {
        let text = String::from_utf8_lossy(&self.visible);
        text.match_indices(word)
            .filter_map(|(at, _)| self.type_word(at, word))
            .collect()
    }

    fn headers_at(&self, body: &Range<usize>, level: i32) -> Vec<Range<usize>> {
        let mut headers = Vec::new();
        let mut start = body.start;
        for end in self.statement_ends(body, level) {
            headers.push(start..end);
            start = self.member_end(end) + 1;
        }
        headers
    }

    fn type_word(&self, at: usize, keyword: &'static str) -> Option<JavaTypeDeclaration> {
        let before = at.checked_sub(1).map(|index| self.visible[index]);
        let starts_word = before.is_none_or(|byte| {
            [
                Self::is_name_byte(byte),
                Self::NOT_BEFORE_TYPE_WORD.contains(&byte),
            ] == [false, false]
        });
        let end = at + keyword.len();
        let (name, name_end) = self.name_after(end);
        let holds = [
            self.braces[at] == 0,
            starts_word,
            self.ends_word(end),
            !name.is_empty(),
        ];
        (!holds.contains(&false)).then_some(JavaTypeDeclaration {
            keyword_at: at,
            keyword,
            name,
            name_end,
        })
    }

    fn name_after(&self, end: usize) -> (String, usize) {
        let rest = &self.visible[end.min(self.visible.len())..];
        let skipped = rest
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        let name: Vec<u8> = rest[skipped..]
            .iter()
            .copied()
            .take_while(|byte| Self::is_name_byte(*byte))
            .collect();
        let name_end = end + skipped + name.len();
        (String::from_utf8_lossy(&name).into_owned(), name_end)
    }

    fn is_name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'$'].contains(&true)
    }

    fn statement_ends(&self, body: &Range<usize>, level: i32) -> Vec<usize> {
        body.clone()
            .filter(|at| self.ends_statement(*at, level))
            .collect()
    }

    fn ends_statement(&self, at: usize, level: i32) -> bool {
        let ends = matches!(self.visible[at], Self::STATEMENT_END | Self::OPEN);
        (self.braces[at], self.parens[at], ends) == (level, 0, true)
    }

    fn member_end(&self, end: usize) -> usize {
        if self.visible[end] == Self::OPEN {
            return Self::closing(&self.braces, end).unwrap_or(end);
        }
        end
    }

    fn visible(source: &str) -> Vec<u8> {
        let mask = crate::lexical::code_mask(Self::LANGUAGE, source);
        source
            .bytes()
            .zip(mask.code)
            .map(|(byte, visible)| [Self::BLANK, byte][usize::from(visible)])
            .collect()
    }

    fn depths(code: &[u8], open: u8, close: u8) -> Vec<i32> {
        let mut depth = 0;
        let mut depths = Vec::with_capacity(code.len() + 1);
        for byte in code {
            depths.push(depth);
            depth += i32::from(*byte == open) - i32::from(*byte == close);
        }
        depths.push(depth);
        depths
    }

    fn closing(depths: &[i32], open: usize) -> Option<usize> {
        let outside = depths[open];
        let after = (open + 1..depths.len()).find(|at| depths[*at] == outside)?;
        Some(after - 1)
    }
}

use super::super::Tokens;
use super::word::Word;
use std::ops::Range;

pub(in super::super) struct Words<'a> {
    items: Vec<Word<'a>>,
}

impl<'a> Words<'a> {
    const OPENERS: &'static [&'static str] = &["(", "[", "{"];
    const CLOSERS: &'static [&'static str] = &[")", "]", "}"];
    const ACCESS: &'static [&'static str] = &[".", "$", ">", ":"];
    const SELF_NAMES: &'static [&'static str] = &["this", "self", "Self", "static"];
    const VALUE_ENDS: &'static [&'static str] = &[")", "]", "}", "?", "!"];
    const NOT_VALUES: &'static [&'static str] = &[
        "return", "move", "in", "yield", "await", "else", "case", "throw", "not", "and", "or",
    ];

    pub(in super::super) fn read(tokens: &Tokens<'a>, source: &str) -> Self {
        let mut words = Self {
            items: Self::plain_words(tokens, source),
        };
        words.match_brackets();
        words
    }

    pub(in super::super) fn len(&self) -> usize {
        self.items.len()
    }

    pub(in super::super) fn text(&self, at: usize) -> &'a str {
        self.items.get(at).map_or("", |word| word.text)
    }

    pub(in super::super) fn pair(&self, at: usize) -> &str {
        self.items.get(at).map_or("", |word| word.pair.as_str())
    }

    pub(in super::super) fn start(&self, at: usize) -> usize {
        self.items.get(at).map_or(0, |word| word.start)
    }

    pub(in super::super) fn depth(&self, at: usize) -> usize {
        self.items.get(at).map_or(0, |word| word.depth)
    }

    pub(in super::super) fn column(&self, at: usize) -> usize {
        self.items.get(at).map_or(0, |word| word.column)
    }

    pub(in super::super) fn line_start(&self, at: usize) -> bool {
        self.items.get(at).is_none_or(|word| word.line_start)
    }

    pub(in super::super) fn spaced(&self, at: usize) -> bool {
        self.items.get(at).is_some_and(|word| word.spaced)
    }

    pub(in super::super) fn is_literal(&self, at: usize) -> bool {
        self.items.get(at).is_some_and(|word| word.literal)
    }

    pub(in super::super) fn touches_previous(&self, at: usize) -> bool {
        self.items.get(at).is_some_and(|word| word.touches_previous)
    }

    pub(in super::super) fn is_name(&self, at: usize) -> bool {
        let first = self.text(at).bytes().next().unwrap_or(b' ');
        let literal = self.items.get(at).is_none_or(|word| word.literal);
        !literal & (first.is_ascii_alphabetic() | (first == b'_'))
    }

    pub(in super::super) fn after_member_access(&self, at: usize) -> bool {
        let touches = self.items.get(at).is_some_and(|word| word.touches_previous);
        touches & Self::ACCESS.contains(&self.text(at.wrapping_sub(1)))
    }

    pub(in super::super) fn value_before(&self, at: usize) -> bool {
        let before = at.wrapping_sub(1);
        let literal = self.items.get(before).is_some_and(|word| word.literal);
        let name = self.is_name(before) & !Self::NOT_VALUES.contains(&self.text(before));
        let number = self
            .text(before)
            .starts_with(|first: char| first.is_ascii_digit());
        literal | name | number | Self::VALUE_ENDS.contains(&self.text(before))
    }

    pub(in super::super) fn inside(&self, open: usize) -> Range<usize> {
        open + 1
            ..self
                .items
                .get(open)
                .and_then(|word| word.partner)
                .unwrap_or(open)
    }

    pub(in super::super) fn inside_of(&self, open: Option<usize>) -> Range<usize> {
        open.map_or(0..0, |open| self.inside(open))
    }

    pub(in super::super) fn after_group(&self, open: usize) -> usize {
        self.items
            .get(open)
            .and_then(|word| word.partner)
            .map_or(open, |close| close + 1)
    }

    pub(in super::super) fn find_at_depth(
        &self,
        from: usize,
        depth: usize,
        wanted: &[&str],
    ) -> Option<usize> {
        let stop = self.stop_index(from, depth, wanted);
        self.is_wanted(stop, depth, wanted).then_some(stop)
    }

    pub(in super::super) fn expression_end(
        &self,
        from: usize,
        depth: usize,
        column: usize,
    ) -> usize {
        (from..self.len())
            .find(|&at| self.ends_expression(at, depth, column))
            .unwrap_or(self.len())
    }

    pub(in super::super) fn block_end(&self, from: usize, column: usize) -> usize {
        (from..self.len())
            .find(|&at| self.line_start(at) & (self.column(at) <= column))
            .unwrap_or(self.len())
    }

    pub(in super::super) fn line_begin(&self, at: usize) -> usize {
        (0..=at)
            .rev()
            .find(|&index| self.line_start(index))
            .unwrap_or(0)
    }

    pub(in super::super) fn line_end(&self, at: usize) -> usize {
        (at + 1..self.len())
            .find(|&index| self.line_start(index))
            .unwrap_or(self.len())
    }

    pub(in super::super) fn has_word(&self, range: Range<usize>, wanted: &[&str]) -> bool {
        range.clone().any(|at| wanted.contains(&self.text(at)))
    }

    pub(in super::super) fn is_self_call(
        &self,
        at: usize,
        receiver: &str,
        parentheses: bool,
    ) -> bool {
        let start = at - usize::from(self.text(at.wrapping_sub(1)) == "#");
        let width = self.access_width(start);
        let owner = self.text(start.wrapping_sub(width + 1));
        let through_self = (width > 0) & (Self::SELF_NAMES.contains(&owner) | (owner == receiver));
        let called = (self.text(at + 1) == "(") | !parentheses;
        ((width == 0) | through_self) & called
    }

    fn access_width(&self, at: usize) -> usize {
        match (self.text(at.wrapping_sub(2)), self.text(at.wrapping_sub(1))) {
            ("-", ">") | (":", ":") => 2,
            (_, ".") => 1,
            _ => 0,
        }
    }

    fn stop_index(&self, from: usize, depth: usize, wanted: &[&str]) -> usize {
        (from..self.len())
            .find(|&at| (self.depth(at) < depth) | self.is_wanted(at, depth, wanted))
            .unwrap_or(self.len())
    }

    fn is_wanted(&self, at: usize, depth: usize, wanted: &[&str]) -> bool {
        let named = wanted.contains(&self.text(at)) | wanted.contains(&self.pair(at));
        (self.depth(at) == depth) & named
    }

    fn ends_expression(&self, at: usize, depth: usize, column: usize) -> bool {
        let closes = self.depth(at) < depth;
        let semicolon = (self.depth(at) == depth) & (self.text(at) == ";");
        let next_member = self.line_start(at) & (self.column(at) <= column);
        closes | semicolon | next_member
    }

    fn plain_words(tokens: &Tokens<'a>, source: &str) -> Vec<Word<'a>> {
        (0..tokens.items.len())
            .map(|at| Self::plain_word(tokens, source, at))
            .collect()
    }

    fn plain_word(tokens: &Tokens<'a>, source: &str, at: usize) -> Word<'a> {
        let token = &tokens.items[at];
        let end = token.start + token.text.len();
        let bytes = source.as_bytes();
        let space_before = bytes
            .get(token.start.wrapping_sub(1))
            .is_some_and(u8::is_ascii_whitespace);
        let space_after = bytes.get(end).is_some_and(u8::is_ascii_whitespace);
        Word {
            text: token.text,
            start: token.start,
            literal: token.literal,
            depth: 0,
            partner: None,
            pair: Self::pair_text(tokens, at),
            touches_previous: Self::touching(tokens, at.wrapping_sub(1)),
            spaced: space_before & space_after,
            line_start: tokens.new_line_before(at),
            column: tokens.column(at),
        }
    }

    fn pair_text(tokens: &Tokens<'a>, at: usize) -> String {
        let second = tokens.items.get(at + 1).map_or("", |token| token.text);
        if Self::touching(tokens, at) {
            return format!("{}{second}", tokens.items[at].text);
        }
        String::new()
    }

    fn touching(tokens: &Tokens<'a>, at: usize) -> bool {
        let first_end = tokens
            .items
            .get(at)
            .map_or(usize::MAX, |token| token.start + token.text.len());
        let second_start = tokens
            .items
            .get(at.wrapping_add(1))
            .map_or(0, |token| token.start);
        first_end == second_start
    }

    fn match_brackets(&mut self) {
        let mut open = Vec::new();
        for at in 0..self.items.len() {
            self.place(at, &mut open);
        }
    }

    fn place(&mut self, at: usize, open: &mut Vec<usize>) {
        self.close(at, open);
        self.items[at].depth = open.len();
        self.open(at, open);
    }

    fn close(&mut self, at: usize, open: &mut Vec<usize>) {
        let closes = Self::CLOSERS.contains(&self.items[at].text);
        let Some(opener) = open.pop_if(|_| closes) else {
            return;
        };
        self.items[opener].partner = Some(at);
        self.items[at].partner = Some(opener);
    }

    fn open(&mut self, at: usize, open: &mut Vec<usize>) {
        if Self::OPENERS.contains(&self.items[at].text) {
            open.push(at);
        }
    }
}

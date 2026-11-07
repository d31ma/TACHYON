#![allow(clippy::needless_bitwise_bool)]

use super::super::cohesion::Words;
use super::code_language::CodeLanguage;
use super::piece::Piece;

pub(super) struct Pieces<'a> {
    items: Vec<Piece<'a>>,
    piece_by_word: Vec<usize>,
    line_firsts: Vec<usize>,
    line_nexts: Vec<usize>,
    source: &'a str,
    language: CodeLanguage,
}

impl<'a> Pieces<'a> {
    const OPERATORS: [&'static str; 40] = [
        "===", "..<", "!==", "<=>", "**=", "<<=", ">>=", "??=", "&&=", "||=", "...", "==", "!=",
        "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "%=", ".=", "->", "=>", "::", "??", "?.",
        "?:", "**", "++", "--", "..", ":=", "<<", ">>", "|=", "&=", "^=", "<>",
    ];
    const LONGEST_OPERATOR: usize = 3;
    const NAME_SUFFIXES: [&'static str; 2] = ["?", "!"];
    const OPENERS: [&'static str; 3] = ["(", "[", "{"];
    const CLOSERS: [&'static str; 3] = [")", "]", "}"];

    pub(super) fn read(words: &Words<'a>, source: &'a str, language: CodeLanguage) -> Self {
        let mut pieces = Self {
            items: Vec::new(),
            piece_by_word: Vec::new(),
            line_firsts: Vec::new(),
            line_nexts: Vec::new(),
            source,
            language,
        };
        pieces.collect(words);
        pieces.link_all(words);
        pieces.measure_firsts();
        pieces.measure_nexts();
        pieces
    }

    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    pub(super) fn language(&self) -> CodeLanguage {
        self.language
    }

    pub(super) fn piece_of_word(&self, word: usize) -> usize {
        self.piece_by_word
            .get(word)
            .copied()
            .unwrap_or(self.items.len())
    }

    pub(super) fn text(&self, at: usize) -> &'a str {
        self.items.get(at).map_or("", |piece| piece.text)
    }

    pub(super) fn start(&self, at: usize) -> usize {
        self.items
            .get(at)
            .map_or(self.source.len(), |piece| piece.start)
    }

    pub(super) fn end(&self, at: usize) -> usize {
        self.items
            .get(at)
            .map_or(self.source.len(), |piece| piece.end)
    }

    pub(super) fn depth(&self, at: usize) -> usize {
        self.items.get(at).map_or(0, |piece| piece.depth)
    }

    pub(super) fn partner(&self, at: usize) -> Option<usize> {
        self.items.get(at).and_then(|piece| piece.partner)
    }

    pub(super) fn after(&self, open: usize) -> usize {
        self.partner(open).map_or(open, |close| close + 1)
    }

    pub(super) fn line_start(&self, at: usize) -> bool {
        self.items.get(at).is_none_or(|piece| piece.line_start)
    }

    pub(super) fn column(&self, at: usize) -> usize {
        self.items.get(at).map_or(0, |piece| piece.column)
    }

    pub(super) fn is_literal(&self, at: usize) -> bool {
        self.items.get(at).is_some_and(|piece| piece.literal)
    }

    pub(super) fn is(&self, at: usize, text: &str) -> bool {
        self.text(at) == text
    }

    pub(super) fn is_any(&self, at: usize, texts: &[&str]) -> bool {
        texts.contains(&self.text(at))
    }

    pub(super) fn matches(&self, at: usize, texts: &[&str]) -> bool {
        texts
            .iter()
            .enumerate()
            .all(|(offset, text)| self.text(at + offset) == *text)
    }

    pub(super) fn is_name(&self, at: usize) -> bool {
        let first = self.text(at).bytes().next().unwrap_or(b' ');
        !self.is_literal(at) & (first.is_ascii_alphabetic() | (first == b'_'))
    }

    pub(super) fn is_number(&self, at: usize) -> bool {
        let first = self.text(at).bytes().next().unwrap_or(b' ');
        !self.is_literal(at) & first.is_ascii_digit()
    }

    pub(super) fn is_opener(&self, at: usize) -> bool {
        self.is_any(at, &Self::OPENERS)
    }

    pub(super) fn is_closer(&self, at: usize) -> bool {
        self.is_any(at, &Self::CLOSERS)
    }

    pub(super) fn is_value_end(&self, at: usize) -> bool {
        self.is_name(at) | self.is_literal(at) | self.is_number(at) | self.is_closer(at)
    }

    pub(super) fn touches(&self, at: usize) -> bool {
        (at > 0) & (self.start(at) == self.end(at.wrapping_sub(1)))
    }

    pub(super) fn slice(&self, from: usize, to: usize) -> &'a str {
        let end = self.end(to.wrapping_sub(1)).max(self.start(from));
        self.source.get(self.start(from)..end).unwrap_or("")
    }

    pub(super) fn bytes(&self, start: usize, end: usize) -> &'a str {
        self.source.get(start..end.max(start)).unwrap_or("")
    }

    pub(super) fn is_plain(&self, from: usize, to: usize) -> bool {
        (from..to.saturating_sub(1)).all(|at| self.gap_is_blank(at))
    }

    pub(super) fn is_whole_lines(&self, from: usize, to: usize) -> bool {
        let before = self.bytes(self.line_head(from), self.start(from));
        let last = to.saturating_sub(1);
        let after = self.bytes(self.end(last), self.line_tail(last));
        before.trim().is_empty() & after.trim().is_empty() & self.is_plain(from, to)
    }

    pub(super) fn line_after(&self, at: usize) -> usize {
        self.line_nexts.get(at).copied().unwrap_or(self.len())
    }

    pub(super) fn line_first(&self, at: usize) -> usize {
        let last = self.len().saturating_sub(1);
        self.line_firsts.get(at.min(last)).copied().unwrap_or(0)
    }

    pub(super) fn line_head(&self, at: usize) -> usize {
        let start = self.start(at);
        self.bytes(0, start)
            .rfind('\n')
            .map_or(0, |newline| newline + 1)
    }

    pub(super) fn line_tail(&self, at: usize) -> usize {
        let end = self.end(at);
        self.bytes(end, self.source.len())
            .find('\n')
            .map_or(self.source.len(), |newline| end + newline + 1)
    }

    pub(super) fn indent(&self, at: usize) -> &'a str {
        let first = self.line_first(at);
        self.bytes(self.line_head(first), self.start(first))
    }

    pub(super) fn top(&self, from: usize, to: usize) -> Vec<usize> {
        let depth = self.depth(from);
        (from..to).filter(|&at| self.depth(at) == depth).collect()
    }

    pub(super) fn has_top(&self, from: usize, to: usize, texts: &[&str]) -> bool {
        self.find_top(from, to, texts).is_some()
    }

    pub(super) fn find_top(&self, from: usize, to: usize, texts: &[&str]) -> Option<usize> {
        self.top(from, to)
            .into_iter()
            .find(|&at| self.is_any(at, texts))
    }

    pub(super) fn count_top(&self, from: usize, to: usize, texts: &[&str]) -> usize {
        self.top(from, to)
            .into_iter()
            .filter(|&at| self.is_any(at, texts))
            .count()
    }

    fn gap_is_blank(&self, at: usize) -> bool {
        self.bytes(self.end(at), self.start(at + 1))
            .trim()
            .is_empty()
    }

    fn collect(&mut self, words: &Words<'a>) {
        let mut at = 0;
        while at < words.len() {
            let width = self.width(words, at);
            let piece = self.piece(words, at, width);
            let index = self.items.len();
            self.items.push(piece);
            self.piece_by_word.extend(std::iter::repeat_n(index, width));
            at += width;
        }
    }

    fn width(&self, words: &Words<'_>, at: usize) -> usize {
        let operator = (2..=Self::LONGEST_OPERATOR)
            .rev()
            .find(|&count| Self::is_operator(words, at, count));
        operator.unwrap_or_else(|| 1 + usize::from(self.has_name_suffix(words, at)))
    }

    fn is_operator(words: &Words<'_>, at: usize, count: usize) -> bool {
        let touching = (at + 1..at + count).all(|next| words.touches_previous(next));
        let literal = (at..at + count).any(|index| words.is_literal(index));
        let text: String = (at..at + count).map(|index| words.text(index)).collect();
        touching & !literal & Self::OPERATORS.contains(&text.as_str())
    }

    fn has_name_suffix(&self, words: &Words<'_>, at: usize) -> bool {
        let suffix =
            Self::NAME_SUFFIXES.contains(&words.text(at + 1)) & words.touches_previous(at + 1);
        let operator = (words.text(at + 2) == "=") & words.touches_previous(at + 2);
        self.language.is_ruby() & words.is_name(at) & suffix & !operator
    }

    fn piece(&self, words: &Words<'a>, at: usize, width: usize) -> Piece<'a> {
        let last = at + width - 1;
        let start = words.start(at);
        let end = words.start(last) + words.text(last).len();
        Piece {
            text: self.source.get(start..end).unwrap_or(""),
            start,
            end,
            depth: words.depth(at),
            partner: None,
            line_start: words.line_start(at),
            column: words.column(at),
            literal: words.is_literal(at),
        }
    }

    fn measure_firsts(&mut self) {
        let mut first = 0;
        for at in 0..self.len() {
            first = [first, at][usize::from(self.line_start(at))];
            self.line_firsts.push(first);
        }
    }

    fn measure_nexts(&mut self) {
        let mut next = self.len();
        self.line_nexts = vec![next; self.len()];
        for at in (0..self.len()).rev() {
            self.line_nexts[at] = next;
            next = [next, at][usize::from(self.line_start(at))];
        }
    }

    fn link_all(&mut self, words: &Words<'_>) {
        for word in 0..words.len() {
            self.link(words, word);
        }
    }

    fn link(&mut self, words: &Words<'_>, word: usize) {
        let after = words.after_group(word);
        let Some(close) = (after > word + 1).then(|| after - 1) else {
            return;
        };
        let open = self.piece_of_word(word);
        let close = self.piece_of_word(close);
        self.items[open].partner = Some(close);
        self.items[close].partner = Some(open);
    }
}

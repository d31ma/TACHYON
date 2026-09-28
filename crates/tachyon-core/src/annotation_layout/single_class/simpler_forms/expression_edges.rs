#![allow(clippy::needless_bitwise_bool)]

use super::pieces::Pieces;

pub(super) struct ExpressionEdges;

impl ExpressionEdges {
    const ACCESS: [&'static str; 4] = [".", "?.", "::", "->"];
    const SIGILS: [&'static str; 3] = ["$", "@", "@@"];
    const LEFT: [&'static str; 38] = [
        "(", "[", "{", ",", ";", "=", "+=", "-=", "*=", "/=", "%=", ".=", "??=", "||=", "&&=",
        ":=", "return", "&&", "||", "and", "or", "not", "if", "elif", "elsif", "while", "unless",
        "until", "?", ":", "=>", "->", "yield", "throw", "then", "do", "else", "case",
    ];
    const RIGHT: [&'static str; 16] = [
        ")", "]", "}", ",", ";", "&&", "||", "and", "or", "?", ":", "{", "then", "do", "else", "if",
    ];
    const CONTINUATIONS: [&'static str; 26] = [
        "+",
        "-",
        "*",
        "/",
        "%",
        ".",
        "?.",
        "<",
        ">",
        "==",
        "!=",
        "===",
        "!==",
        "<=",
        ">=",
        "(",
        "[",
        "**",
        "|",
        "&",
        "^",
        "??",
        "in",
        "instanceof",
        "is",
        "as",
    ];

    pub(super) fn chain_start(pieces: &Pieces<'_>, last: usize) -> usize {
        Self::walk_back(pieces, last)
    }

    pub(super) fn is_open_left(pieces: &Pieces<'_>, start: usize) -> bool {
        let before = start.wrapping_sub(1);
        let listed = pieces.is_any(before, &Self::LEFT);
        let statement = pieces.line_start(start) & pieces.is_value_end(before);
        listed | (start == 0) | statement
    }

    pub(super) fn is_closed_right(pieces: &Pieces<'_>, after: usize) -> bool {
        let listed = pieces.is_any(after, &Self::RIGHT);
        let new_line = pieces.line_start(after) & !pieces.is_any(after, &Self::CONTINUATIONS);
        listed | new_line
    }

    pub(super) fn is_access(pieces: &Pieces<'_>, at: usize) -> bool {
        pieces.is_any(at, &Self::ACCESS)
    }

    fn walk_back(pieces: &Pieces<'_>, last: usize) -> usize {
        let mut at = last;
        while let Some(next) = Self::step(pieces, at) {
            at = next;
        }
        at
    }

    fn step(pieces: &Pieces<'_>, at: usize) -> Option<usize> {
        let before = at.wrapping_sub(1);
        let word = pieces.is_name(at) | pieces.is_literal(at) | pieces.is_number(at);
        let closer = pieces.partner(at).filter(|&open| open < at);
        let joined = pieces.touches(at) | pieces.is(at, "{");
        let called = pieces.is_opener(at) & joined & pieces.is_value_end(before);
        let accessed = word & pieces.touches(at) & Self::is_access(pieces, before);
        let member =
            accessed & pieces.touches(before) & pieces.is_value_end(before.wrapping_sub(1));
        let marked = word & pieces.touches(at) & pieces.is_any(before, &Self::SIGILS);
        closer
            .or_else(|| called.then_some(before))
            .or_else(|| member.then_some(before.wrapping_sub(1)))
            .or_else(|| marked.then_some(before))
    }
}

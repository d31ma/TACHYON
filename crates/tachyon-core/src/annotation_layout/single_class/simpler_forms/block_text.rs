#![allow(clippy::needless_bitwise_bool)]

use super::pieces::Pieces;

pub(super) struct BlockText;

impl BlockText {
    const DECLARATIONS: [&'static str; 8] = [
        "let", "const", "var", "val", "function", "class", "struct", "fun",
    ];
    const NOT_TYPES: [&'static str; 16] = [
        "return", "throw", "new", "await", "yield", "delete", "typeof", "void", "echo", "print",
        "else", "case", "go", "defer", "if", "raise",
    ];

    pub(super) fn dedent(text: &str, unit: &str) -> Option<String> {
        text.split_inclusive('\n')
            .map(|line| Self::dedent_line(line, unit))
            .collect()
    }

    pub(super) fn unit<'t>(outer: &str, inner: &'t str) -> Option<&'t str> {
        inner.strip_prefix(outer).filter(|unit| !unit.is_empty())
    }

    pub(super) fn is_movable(pieces: &Pieces<'_>, from: usize, to: usize) -> bool {
        let scoped = pieces.language().has_block_scope() & Self::declares(pieces, from, to);
        !scoped & !Self::has_long_literal(pieces, from, to) & pieces.is_plain(from, to)
    }

    pub(super) fn last_statement(pieces: &Pieces<'_>, from: usize, to: usize) -> usize {
        (from..to)
            .rev()
            .find(|&at| Self::starts_statement(pieces, from, at))
            .unwrap_or(from)
    }

    pub(super) fn statement_count(pieces: &Pieces<'_>, from: usize, to: usize) -> usize {
        (from..to)
            .filter(|&at| Self::starts_statement(pieces, from, at))
            .count()
    }

    fn starts_statement(pieces: &Pieces<'_>, first: usize, at: usize) -> bool {
        let aligned =
            (pieces.depth(at) == pieces.depth(first)) & (pieces.column(at) == pieces.column(first));
        pieces.line_start(at) & aligned & !pieces.is_closer(at)
    }

    fn dedent_line(line: &str, unit: &str) -> Option<String> {
        if line.trim().is_empty() {
            return Some(line.trim_start_matches([' ', '\t']).to_owned());
        }
        line.strip_prefix(unit).map(str::to_owned)
    }

    fn has_long_literal(pieces: &Pieces<'_>, from: usize, to: usize) -> bool {
        (from..to).any(|at| pieces.is_literal(at) & pieces.text(at).contains('\n'))
    }

    fn declares(pieces: &Pieces<'_>, from: usize, to: usize) -> bool {
        pieces
            .top(from, to)
            .into_iter()
            .any(|at| Self::is_declaration(pieces, at))
    }

    fn is_declaration(pieces: &Pieces<'_>, at: usize) -> bool {
        let keyword = pieces.is_any(at, &Self::DECLARATIONS) | pieces.is_any(at + 1, &[":="]);
        let typed = pieces.is_name(at)
            & !pieces.is_any(at, &Self::NOT_TYPES)
            & (pieces.is_name(at + 1)
                | pieces.is(at + 1, "<")
                | pieces.matches(at + 1, &["[", "]"]));
        pieces.line_start(at) & (keyword | typed)
    }
}

#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::expression_edges::ExpressionEdges;
use super::pieces::Pieces;

pub(super) struct Condition<'p, 'a> {
    pieces: &'p Pieces<'a>,
    from: usize,
    to: usize,
}

impl<'p, 'a> Condition<'p, 'a> {
    const JOINERS: [&'static str; 4] = ["&&", "||", "and", "or"];
    const UNSAFE: [&'static str; 9] = ["?", "??", "=", ",", "if", "?:", "lambda", ":=", "unless"];
    const COMPARISONS: [&'static str; 12] = [
        "==",
        "===",
        "!=",
        "!==",
        "<",
        ">",
        "<=",
        ">=",
        "instanceof",
        "in",
        "is",
        "<>",
    ];
    const NEGATIONS: [&'static str; 2] = ["!", "not"];
    const EQUALITIES: [(&'static str, &'static str); 4] =
        [("==", "!="), ("!=", "=="), ("===", "!=="), ("!==", "===")];
    const LOOSE_THAN_EQUALITY: [&'static str; 26] = [
        "&&",
        "||",
        "??",
        "?",
        "=",
        ",",
        "?:",
        "&",
        "|",
        "^",
        "and",
        "or",
        "not",
        "in",
        "is",
        "if",
        "<",
        ">",
        "<=",
        ">=",
        "instanceof",
        "=~",
        "!~",
        "<=>",
        "..",
        "...",
    ];
    const KEYWORDS: [&'static str; 18] = [
        "and",
        "or",
        "not",
        "in",
        "is",
        "if",
        "else",
        "instanceof",
        "typeof",
        "await",
        "new",
        "as",
        "lambda",
        "yield",
        "then",
        "do",
        "unless",
        "defined?",
    ];
    const BOOLEAN_CALLS: [&'static str; 25] = [
        "includes",
        "some",
        "every",
        "startsWith",
        "endsWith",
        "has",
        "isArray",
        "isNaN",
        "isInteger",
        "isFinite",
        "test",
        "isinstance",
        "issubclass",
        "hasattr",
        "callable",
        "startswith",
        "endswith",
        "isdigit",
        "isalpha",
        "isalnum",
        "isspace",
        "isupper",
        "islower",
        "any",
        "all",
    ];
    const SIGILS: [&'static str; 3] = ["$", "@", "@@"];

    pub(super) fn new(pieces: &'p Pieces<'a>, from: usize, to: usize) -> Self {
        Self { pieces, from, to }
    }

    pub(super) fn text(&self) -> &'a str {
        self.pieces.slice(self.from, self.to)
    }

    pub(super) fn is_filled(&self) -> bool {
        self.to > self.from
    }

    pub(super) fn is_boolean(&self) -> bool {
        let typed = self.pieces.language().has_typed_conditions();
        self.is_filled() & (typed | self.has_boolean_operands())
    }

    pub(super) fn is_simple(&self) -> bool {
        self.is_filled() & (self.from..self.to).all(|at| self.is_simple_piece(at))
    }

    pub(super) fn negated(&self) -> Option<String> {
        Some(self).filter(|condition| condition.is_filled())?;
        let flipped = self.flipped_equality();
        flipped.or_else(|| self.plain_negation())
    }

    pub(super) fn for_and(&self) -> String {
        let loose = self.pieces.language().looser_than_and();
        let wrapped = self.pieces.has_top(self.from, self.to, loose);
        Self::wrap(self.text(), wrapped)
    }

    pub(super) fn for_operand(&self) -> String {
        Self::wrap(self.text(), !self.is_simple())
    }

    pub(super) fn flipped_equality(&self) -> Option<String> {
        let at = self
            .pieces
            .find_top(self.from, self.to, &["==", "!=", "===", "!=="])?;
        let opposite = Self::opposite(self.pieces.text(at))?;
        let before = self
            .pieces
            .bytes(self.pieces.start(self.from), self.pieces.start(at));
        let after = self
            .pieces
            .bytes(self.pieces.end(at), self.pieces.end(self.to - 1));
        let equalities = self
            .pieces
            .count_top(self.from, self.to, &["==", "!=", "===", "!=="]);
        let loose = self
            .pieces
            .has_top(self.from, self.to, &Self::LOOSE_THAN_EQUALITY);
        ((equalities == 1) & !loose).then(|| format!("{before}{opposite}{after}"))
    }

    fn wrap(text: &str, wrapped: bool) -> String {
        if wrapped {
            return format!("({text})");
        }
        text.to_owned()
    }

    fn opposite(operator: &str) -> Option<&'static str> {
        Self::EQUALITIES
            .iter()
            .find(|(one, _)| *one == operator)
            .map(|(_, other)| *other)
    }

    fn plain_negation(&self) -> Option<String> {
        let negated_already = self.pieces.is_any(self.from, &Self::NEGATIONS);
        let not = self.pieces.language().not_word();
        let operand = self.for_operand();
        (!negated_already).then(|| format!("{not}{operand}"))
    }

    fn has_boolean_operands(&self) -> bool {
        let safe = !self.pieces.has_top(self.from, self.to, &Self::UNSAFE);
        safe & self
            .operands()
            .iter()
            .all(|&(from, to)| self.is_boolean_operand(from, to))
    }

    fn operands(&self) -> Vec<(usize, usize)> {
        let joins: Vec<usize> = self
            .pieces
            .top(self.from, self.to)
            .into_iter()
            .filter(|&at| self.pieces.is_any(at, &Self::JOINERS))
            .collect();
        let starts = std::iter::once(self.from).chain(joins.iter().map(|at| at + 1));
        let ends = joins.iter().copied().chain(std::iter::once(self.to));
        starts.zip(ends).collect()
    }

    fn is_boolean_operand(&self, from: usize, to: usize) -> bool {
        let negated = self.pieces.is_any(from, &Self::NEGATIONS);
        let compared = self.pieces.has_top(from, to, &Self::COMPARISONS);
        let language = self.pieces.language();
        let words = [language.true_word(), language.false_word()];
        let literal = (to == from + 1) & self.pieces.is_any(from, &words);
        (to > from) & (negated | compared | literal | self.is_boolean_call(from, to))
    }

    fn is_boolean_call(&self, from: usize, to: usize) -> bool {
        let last = to.saturating_sub(1);
        let name_at = self
            .pieces
            .partner(last)
            .filter(|&open| open < last)
            .map_or(last, |open| open.wrapping_sub(1));
        let name = self.pieces.text(name_at);
        let ruby = self.pieces.language() == CodeLanguage::Ruby;
        let predicate = ruby & name.ends_with('?') & (name != "defined?");
        let called = Self::BOOLEAN_CALLS.contains(&name) & self.pieces.is(last, ")");
        let top = self.pieces.depth(name_at) == self.pieces.depth(from);
        let whole = ExpressionEdges::chain_start(self.pieces, last) == from;
        (predicate | called) & top & whole
    }

    fn is_simple_piece(&self, at: usize) -> bool {
        let nested = self.pieces.depth(at) > self.pieces.depth(self.from);
        let joined = (at == self.from) | self.pieces.touches(at) | self.pieces.is_closer(at);
        nested | (self.is_simple_kind(at) & joined)
    }

    fn is_simple_kind(&self, at: usize) -> bool {
        let name = self.pieces.is_name(at) & !self.pieces.is_any(at, &Self::KEYWORDS);
        let value = self.pieces.is_literal(at) | self.pieces.is_number(at);
        let group = self.pieces.is_opener(at) | self.pieces.is_closer(at);
        let access =
            ExpressionEdges::is_access(self.pieces, at) | self.pieces.is_any(at, &Self::SIGILS);
        name | value | group | access
    }
}

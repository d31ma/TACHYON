#![allow(clippy::needless_bitwise_bool)]

use super::condition::Condition;
use super::if_statement::IfStatement;
use super::pieces::Pieces;

pub(super) struct BooleanBranch {
    pub(super) value: bool,
    pub(super) target: String,
    pub(super) last: usize,
}

impl BooleanBranch {
    pub(super) fn returned(pieces: &Pieces<'_>, from: usize, to: usize) -> Option<Self> {
        let starts = Some(from).filter(|&at| pieces.is(at, "return"))?;
        Self::valued(pieces, starts, starts + 1, to)
    }

    pub(super) fn assigned(pieces: &Pieces<'_>, from: usize, to: usize) -> Option<Self> {
        let equals = pieces.find_top(from, to, &["="])?;
        let target = Condition::new(pieces, from, equals);
        let simple = Some(target).filter(Condition::is_simple)?;
        let branch = Self::valued(pieces, from, equals + 1, to)?;
        Some(Self {
            target: simple.text().to_owned(),
            ..branch
        })
    }

    pub(super) fn following_return(pieces: &Pieces<'_>, statement: &IfStatement) -> Option<Self> {
        let at = statement.end;
        let aligned = pieces.line_start(at)
            & (pieces.depth(at) == pieces.depth(statement.keyword))
            & (pieces.column(at) == pieces.column(statement.keyword));
        let returned = Self::returned(pieces, at, pieces.line_after(at))?;
        aligned.then_some(returned)
    }

    pub(super) fn condition_for(
        pieces: &Pieces<'_>,
        statement: &IfStatement,
        value: bool,
    ) -> Option<String> {
        let condition = statement.condition(pieces);
        let kept = (value & condition.is_boolean()).then(|| condition.text().to_owned());
        kept.or_else(|| (!value).then_some(condition)?.negated())
    }

    pub(super) fn ending(pieces: &Pieces<'_>, last: usize) -> &'static str {
        ["", ";"][usize::from(pieces.is(last, ";"))]
    }

    fn valued(pieces: &Pieces<'_>, head: usize, value_at: usize, to: usize) -> Option<Self> {
        let language = pieces.language();
        let words = [language.false_word(), language.true_word()];
        let value = words.iter().position(|word| pieces.is(value_at, word))?;
        let semicolon = pieces.is(value_at + 1, ";");
        let fits = to == value_at + 1 + usize::from(semicolon);
        fits.then_some(Self {
            value: value == 1,
            target: pieces.slice(head, head + 1).to_owned(),
            last: to - 1,
        })
    }
}

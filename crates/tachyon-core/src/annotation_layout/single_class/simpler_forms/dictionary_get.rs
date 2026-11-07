#![allow(clippy::needless_bitwise_bool)]

use super::condition::Condition;
use super::expression_edges::ExpressionEdges;
use super::form_pattern::FormPattern;
use super::if_statement::IfStatement;
use super::lookup_rule::LookupRule;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct DictionaryGet;

impl DictionaryGet {
    pub(super) fn find(pieces: &Pieces<'_>, statements: &[IfStatement]) -> Vec<SimplerForm> {
        LookupRule::of(pieces.language())
            .map(|rule| Self::forms(pieces, rule, statements))
            .unwrap_or_default()
    }

    fn forms(
        pieces: &Pieces<'_>,
        rule: &LookupRule,
        statements: &[IfStatement],
    ) -> Vec<SimplerForm> {
        let mut forms: Vec<SimplerForm> = (0..pieces.len())
            .filter_map(|at| Self::choice(pieces, rule, at))
            .collect();
        forms.extend(
            statements
                .iter()
                .filter_map(|statement| Self::statement(pieces, rule, statement)),
        );
        forms
    }

    fn choice(pieces: &Pieces<'_>, rule: &LookupRule, at: usize) -> Option<SimplerForm> {
        let python = pieces.language().is_python();
        let keyword =
            Some(at).filter(|&index| pieces.is(index, ["?", "if"][usize::from(python)]))?;
        let line_end = pieces.line_after(keyword);
        let divider =
            pieces.find_top(keyword + 1, line_end, &[[":", "else"][usize::from(python)]])?;
        let before = ExpressionEdges::chain_start(pieces, keyword.wrapping_sub(1));
        let (condition, value) =
            Self::choice_parts(python, (before, keyword), (keyword + 1, divider));
        let default_end = Self::default_end(pieces, divider + 1)?;
        let lookup = Self::lookup(pieces, rule, condition, value)?;
        let text = lookup.replace("{default}", pieces.slice(divider + 1, default_end));
        let placed = !pieces.line_start(keyword)
            & ExpressionEdges::is_open_left(pieces, before)
            & ExpressionEdges::is_closed_right(pieces, default_end)
            & pieces.is_plain(before, default_end);
        placed.then(|| {
            SimplerForm::new(
                FormPattern::DictionaryGet,
                pieces.start(before),
                pieces.end(default_end - 1),
                text,
            )
        })
    }

    fn choice_parts(
        python: bool,
        first: (usize, usize),
        second: (usize, usize),
    ) -> ((usize, usize), (usize, usize)) {
        [(first, second), (second, first)][usize::from(python)]
    }

    fn statement(
        pieces: &Pieces<'_>,
        rule: &LookupRule,
        statement: &IfStatement,
    ) -> Option<SimplerForm> {
        let other = statement.else_part?;
        let then_part = statement.then_part;
        let found = pieces.find_top(then_part.from, then_part.to, &["="])?;
        let missing = pieces.find_top(other.from, other.to, &["="])?;
        let target = pieces.slice(then_part.from, found);
        let same_target = pieces.slice(other.from, missing) == target;
        let default_end = Self::default_end(pieces, missing + 1)?;
        let condition = (statement.condition_from, statement.condition_to);
        let lookup = Self::lookup(pieces, rule, condition, (found + 1, then_part.to))?;
        let text = lookup.replace("{default}", pieces.slice(missing + 1, default_end));
        let last = statement.end - 1;
        let fits = statement.is_chain_head(pieces)
            & !pieces.language().is_braced()
            & same_target
            & (default_end == other.to)
            & Condition::new(pieces, then_part.from, found).is_simple()
            & pieces.is_plain(statement.keyword, last + 1);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::DictionaryGet,
                pieces.start(statement.keyword),
                pieces.end(last),
                format!("{target} = {text}"),
            )
        })
    }

    fn lookup(
        pieces: &Pieces<'_>,
        rule: &LookupRule,
        condition: (usize, usize),
        value: (usize, usize),
    ) -> Option<String> {
        let (from, to) = value;
        let last = Some(to.saturating_sub(1)).filter(|&at| pieces.is_any(at, &["]", ")"]))?;
        let open = pieces.partner(last)?;
        let map_end = open.saturating_sub(2 * usize::from(rule.read_by_call));
        let called = pieces.is(open.wrapping_sub(1), "get") & pieces.is(open.wrapping_sub(2), ".");
        let shaped =
            [pieces.is(last, "]"), pieces.is(last, ")") & called][usize::from(rule.read_by_call)];
        let map = Condition::new(pieces, from, map_end);
        let key = Condition::new(pieces, open + 1, last);
        let checked = rule.checks_key(
            pieces.slice(condition.0, condition.1),
            map.text(),
            key.text(),
        );
        let whole = ExpressionEdges::chain_start(pieces, last) == from;
        let fits = shaped & whole & checked & map.is_simple() & key.is_simple();
        fits.then(|| rule.lookup(map.text(), key.text(), "{default}"))
    }

    fn default_end(pieces: &Pieces<'_>, at: usize) -> Option<usize> {
        let value = pieces.is_name(at) | pieces.is_literal(at) | pieces.is_number(at);
        let single = value & !pieces.is_opener(at + 1);
        let empty = pieces.is_opener(at) & (pieces.partner(at) == Some(at + 1));
        [single.then_some(at + 1), empty.then_some(at + 2)]
            .into_iter()
            .flatten()
            .next()
    }
}

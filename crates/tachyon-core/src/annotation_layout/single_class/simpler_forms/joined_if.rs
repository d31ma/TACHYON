#![allow(clippy::needless_bitwise_bool)]

use super::block_text::BlockText;
use super::code_language::CodeLanguage;
use super::form_pattern::FormPattern;
use super::if_statement::IfStatement;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct JoinedIf;

impl JoinedIf {
    const BOUND_CONDITIONS: [&'static str; 7] = [";", ":=", "let", "var", "case", ",", "then"];

    pub(super) fn find(pieces: &Pieces<'_>, statements: &[IfStatement]) -> Vec<SimplerForm> {
        statements
            .iter()
            .filter_map(|outer| Self::form(pieces, statements, outer))
            .collect()
    }

    fn form(
        pieces: &Pieces<'_>,
        statements: &[IfStatement],
        outer: &IfStatement,
    ) -> Option<SimplerForm> {
        let outer = Some(outer).filter(|found| found.is_chain_head(pieces))?;
        let chain = Self::nested(pieces, statements, outer);
        let inner = chain.last().filter(|_| chain.len() > 1)?;
        let joined: Vec<String> = chain
            .iter()
            .map(|statement| statement.condition(pieces).for_and())
            .collect();
        let and = format!(" {} ", pieces.language().and_word());
        let head = format!(
            "{}{}{}",
            pieces.bytes(
                pieces.start(outer.keyword),
                pieces.start(outer.condition_from)
            ),
            joined.join(&and),
            Self::head_end(pieces, outer),
        );
        let unit = BlockText::unit(pieces.indent(outer.keyword), pieces.indent(inner.keyword))?;
        let lines = pieces.bytes(
            pieces.line_head(inner.then_part.from),
            Self::body_end(pieces, inner),
        );
        let body = BlockText::dedent(lines, unit)?;
        let movable = BlockText::is_movable(pieces, inner.then_part.from, inner.then_part.to);
        let closing = Self::closing(pieces, outer);
        movable.then(|| {
            SimplerForm::new(
                FormPattern::JoinedIf,
                pieces.start(outer.keyword),
                closing.1,
                format!("{head}\n{body}{}", closing.0),
            )
        })
    }

    fn nested(
        pieces: &Pieces<'_>,
        statements: &[IfStatement],
        outer: &IfStatement,
    ) -> Vec<IfStatement> {
        let mut chain = vec![*outer];
        while let Some(inner) = chain
            .last()
            .and_then(|last| Self::only_inner(pieces, statements, last))
        {
            chain.push(inner);
        }
        chain
    }

    fn only_inner(
        pieces: &Pieces<'_>,
        statements: &[IfStatement],
        outer: &IfStatement,
    ) -> Option<IfStatement> {
        let index = statements
            .binary_search_by_key(&outer.then_part.from, |statement| statement.keyword)
            .ok()?;
        let inner = statements.get(index).copied()?;
        let fits = Self::is_plain_block(pieces, outer)
            & Self::is_plain_block(pieces, &inner)
            & (inner.end == outer.then_part.to)
            & pieces.line_start(inner.keyword)
            & pieces.is(inner.keyword, "if");
        fits.then_some(inner)
    }

    fn is_plain_block(pieces: &Pieces<'_>, statement: &IfStatement) -> bool {
        let part = &statement.then_part;
        let braced = part.is_braced() | !pieces.language().is_braced();
        let bound = pieces.has_top(
            statement.condition_from,
            statement.condition_to,
            &Self::BOUND_CONDITIONS,
        );
        let head_line =
            pieces.line_start(part.from) & pieces.is_plain(statement.keyword, part.from + 1);
        let closed = part
            .close
            .is_none_or(|close| pieces.line_start(close) & pieces.line_start(close + 1));
        braced & !bound & !statement.has_else() & head_line & closed
    }

    fn head_end<'a>(pieces: &Pieces<'a>, outer: &IfStatement) -> &'a str {
        let last = outer.then_part.open.unwrap_or(outer.condition_to - 1);
        pieces.bytes(pieces.end(outer.condition_to - 1), pieces.end(last))
    }

    fn body_end(pieces: &Pieces<'_>, inner: &IfStatement) -> usize {
        match pieces.language() {
            CodeLanguage::Python => pieces.line_tail(inner.then_part.to - 1),
            _ => pieces.line_head(inner.then_part.to),
        }
    }

    fn closing(pieces: &Pieces<'_>, outer: &IfStatement) -> (String, usize) {
        let indent = pieces.indent(outer.keyword);
        match pieces.language() {
            CodeLanguage::Python => (String::new(), pieces.line_tail(outer.then_part.to - 1)),
            CodeLanguage::Ruby => (format!("{indent}end"), pieces.end(outer.end - 1)),
            _ => (format!("{indent}}}"), pieces.end(outer.end - 1)),
        }
    }
}

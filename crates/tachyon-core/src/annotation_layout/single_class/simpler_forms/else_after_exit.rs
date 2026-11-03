#![allow(clippy::needless_bitwise_bool)]

use super::block_text::BlockText;
use super::code_language::CodeLanguage;
use super::form_pattern::FormPattern;
use super::if_part::IfPart;
use super::if_statement::IfStatement;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct ElseAfterExit;

impl ElseAfterExit {
    const SCOPED_CONDITIONS: [&'static str; 2] = [";", ":="];

    pub(super) fn find(pieces: &Pieces<'_>, statements: &[IfStatement]) -> Vec<SimplerForm> {
        statements
            .iter()
            .filter(|statement| statement.is_chain_head(pieces))
            .flat_map(|head| Self::chain(pieces, statements, head))
            .collect()
    }

    fn chain(
        pieces: &Pieces<'_>,
        statements: &[IfStatement],
        head: &IfStatement,
    ) -> Vec<SimplerForm> {
        let mut forms = Vec::new();
        let mut current = Some(*head);
        while let Some(statement) = current.filter(|found| Self::exits(pieces, &found.then_part)) {
            forms.extend(Self::else_form(pieces, &statement));
            current = statement
                .else_if
                .and_then(|at| Self::statement_at(statements, at));
        }
        forms
    }

    fn statement_at(statements: &[IfStatement], keyword: usize) -> Option<IfStatement> {
        let index = statements
            .binary_search_by_key(&keyword, |statement| statement.keyword)
            .ok()?;
        statements.get(index).copied()
    }

    fn exits(pieces: &Pieces<'_>, part: &IfPart) -> bool {
        let last = BlockText::last_statement(pieces, part.from, part.to);
        pieces.is_any(last, pieces.language().exit_words())
    }

    fn else_form(pieces: &Pieces<'_>, statement: &IfStatement) -> Option<SimplerForm> {
        let unscoped = !pieces.has_top(
            statement.condition_from,
            statement.condition_to,
            &Self::SCOPED_CONDITIONS,
        );
        let chained = statement
            .else_if
            .and_then(|next| Self::else_if_form(pieces, statement, next));
        let block = statement
            .else_part
            .and_then(|part| Self::else_block_form(pieces, statement, &part));
        chained.or(block).filter(|_| unscoped)
    }

    fn else_if_form(
        pieces: &Pieces<'_>,
        statement: &IfStatement,
        next: usize,
    ) -> Option<SimplerForm> {
        let keyword = statement.else_keyword?;
        let indent = pieces.indent(keyword);
        let own_line = usize::from(pieces.line_start(keyword));
        let braced_start = [pieces.end(keyword.wrapping_sub(1)), pieces.start(keyword)][own_line];
        let braced_text = format!("{}{}", ["\n", ""][own_line], [indent, ""][own_line]);
        let (start, end, text) = match pieces.language() {
            CodeLanguage::Python => (pieces.start(keyword), pieces.end(keyword), "if".to_owned()),
            CodeLanguage::Ruby => (
                pieces.start(keyword),
                pieces.end(keyword),
                format!("end\n{indent}if"),
            ),
            _ => (braced_start, pieces.start(next), braced_text),
        };
        Some(SimplerForm::new(
            FormPattern::ElseAfterExit,
            start,
            end,
            text,
        ))
    }

    fn else_block_form(
        pieces: &Pieces<'_>,
        statement: &IfStatement,
        part: &IfPart,
    ) -> Option<SimplerForm> {
        let movable = BlockText::is_movable(pieces, part.from, part.to);
        let form = match pieces.language() {
            CodeLanguage::Python => Self::indented_block(pieces, statement, part),
            CodeLanguage::Ruby => Self::ended_block(pieces, statement, part),
            _ => Self::braced_block(pieces, statement, part),
        };
        form.filter(|_| movable)
    }

    fn braced_block(
        pieces: &Pieces<'_>,
        statement: &IfStatement,
        part: &IfPart,
    ) -> Option<SimplerForm> {
        let keyword = statement.else_keyword?;
        let then_close = statement.then_part.close?;
        let open = part.open?;
        let close = part.close?;
        let shaped = pieces.line_start(open + 1)
            & pieces.line_start(close)
            & pieces.line_start(close + 1)
            & !pieces.line_start(keyword)
            & pieces.is_plain(then_close, open + 1);
        let lines = pieces.bytes(pieces.line_tail(open), pieces.line_head(close));
        let body = Self::moved_body(pieces, keyword, part, lines)?;
        let kept = body.trim_end_matches(['\n', '\r']);
        let text = Self::on_new_line(kept);
        shaped.then(|| {
            SimplerForm::new(
                FormPattern::ElseAfterExit,
                pieces.end(then_close),
                pieces.end(close),
                text,
            )
        })
    }

    fn indented_block(
        pieces: &Pieces<'_>,
        statement: &IfStatement,
        part: &IfPart,
    ) -> Option<SimplerForm> {
        let keyword = statement.else_keyword?;
        let last = part.to - 1;
        let shaped = pieces.is_whole_lines(keyword, keyword + 2);
        let lines = pieces.bytes(pieces.line_tail(keyword + 1), pieces.line_tail(last));
        let body = Self::moved_body(pieces, keyword, part, lines)?;
        let start = pieces.line_head(keyword);
        shaped.then(|| {
            SimplerForm::new(
                FormPattern::ElseAfterExit,
                start,
                pieces.line_tail(last),
                body,
            )
        })
    }

    fn ended_block(
        pieces: &Pieces<'_>,
        statement: &IfStatement,
        part: &IfPart,
    ) -> Option<SimplerForm> {
        let keyword = statement.else_keyword?;
        let closing = part.close?;
        let shaped = pieces.is_whole_lines(keyword, keyword + 1)
            & pieces.is_whole_lines(closing, closing + 1);
        let lines = pieces.bytes(pieces.line_tail(keyword), pieces.line_head(closing));
        let body = Self::moved_body(pieces, keyword, part, lines)?;
        shaped.then(|| {
            SimplerForm::new(
                FormPattern::ElseAfterExit,
                pieces.start(keyword),
                pieces.line_tail(closing),
                format!("end\n{body}"),
            )
        })
    }

    fn on_new_line(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        format!("\n{text}")
    }

    fn moved_body(
        pieces: &Pieces<'_>,
        anchor: usize,
        part: &IfPart,
        lines: &str,
    ) -> Option<String> {
        let unit = BlockText::unit(pieces.indent(anchor), pieces.indent(part.from));
        let empty = part.from >= part.to;
        let dedented = unit.and_then(|unit| BlockText::dedent(lines, unit));
        dedented.or_else(|| empty.then(String::new))
    }
}

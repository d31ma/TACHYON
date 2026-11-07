#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::condition::Condition;
use super::if_part::IfPart;
use super::pieces::Pieces;

#[derive(Clone, Copy, Debug)]
pub(super) struct IfStatement {
    pub(super) keyword: usize,
    pub(super) condition_from: usize,
    pub(super) condition_to: usize,
    pub(super) then_part: IfPart,
    pub(super) else_keyword: Option<usize>,
    pub(super) else_part: Option<IfPart>,
    pub(super) else_if: Option<usize>,
    pub(super) end: usize,
}

impl IfStatement {
    const KEYWORDS: [&'static str; 3] = ["if", "elif", "elsif"];
    const INDENTED_ELSES: [&'static str; 2] = ["else", "elif"];
    const ENDED_ELSES: [&'static str; 2] = ["else", "elsif"];

    pub(super) fn read(pieces: &Pieces<'_>, at: usize) -> Option<Self> {
        let keyword = Some(at).filter(|&index| pieces.is_any(index, &Self::KEYWORDS))?;
        Self::read_for(pieces, keyword)
    }

    pub(super) fn condition<'p, 'a>(&self, pieces: &'p Pieces<'a>) -> Condition<'p, 'a> {
        Condition::new(pieces, self.condition_from, self.condition_to)
    }

    pub(super) fn is_chain_head(&self, pieces: &Pieces<'_>) -> bool {
        let first = pieces.is(self.keyword, "if");
        let after_else = pieces.is(self.keyword.wrapping_sub(1), "else");
        first & !after_else & pieces.line_start(self.keyword)
    }

    pub(super) fn has_else(&self) -> bool {
        self.else_keyword.is_some()
    }

    fn read_for(pieces: &Pieces<'_>, keyword: usize) -> Option<Self> {
        match pieces.language() {
            CodeLanguage::Python => Self::indented(pieces, keyword),
            CodeLanguage::Ruby => Self::ended(pieces, keyword),
            _ => Self::braced(pieces, keyword),
        }
    }

    fn braced(pieces: &Pieces<'_>, keyword: usize) -> Option<Self> {
        let (condition_from, condition_to, head_end) = Self::braced_head(pieces, keyword)?;
        let then_part = IfPart::after_head(pieces, head_end)?;
        let else_keyword = Some(then_part.end).filter(|&at| pieces.is(at, "else"));
        let else_if = else_keyword
            .map(|at| at + 1)
            .filter(|&at| pieces.is(at, "if"));
        let else_part = else_keyword
            .filter(|_| else_if.is_none())
            .and_then(|at| IfPart::after_head(pieces, at + 1));
        let end = else_part.map_or(then_part.end, |part| part.end);
        Some(Self {
            keyword,
            condition_from,
            condition_to,
            then_part,
            else_keyword,
            else_part,
            else_if,
            end,
        })
    }

    fn braced_head(pieces: &Pieces<'_>, keyword: usize) -> Option<(usize, usize, usize)> {
        if pieces.language().wraps_conditions() {
            return Self::wrapped_head(pieces, keyword);
        }
        Self::bare_head(pieces, keyword)
    }

    fn wrapped_head(pieces: &Pieces<'_>, keyword: usize) -> Option<(usize, usize, usize)> {
        let open = Some(keyword + 1).filter(|&at| pieces.is(at, "("))?;
        let close = pieces.partner(open)?;
        Some((open + 1, close, close + 1))
    }

    fn bare_head(pieces: &Pieces<'_>, keyword: usize) -> Option<(usize, usize, usize)> {
        let line_end = pieces.line_after(keyword);
        let brace = pieces.find_top(keyword + 1, line_end, &["{"])?;
        Some((keyword + 1, brace, brace))
    }

    fn indented(pieces: &Pieces<'_>, keyword: usize) -> Option<Self> {
        let colon = pieces.line_after(keyword) - 1;
        let column = pieces.column(keyword);
        let heads_line = pieces.is(colon, ":") & pieces.line_start(keyword);
        let then_part = heads_line
            .then_some(colon)
            .and_then(|at| IfPart::indented(pieces, at, column))?;
        let else_keyword = Some(then_part.end)
            .filter(|&at| pieces.is_any(at, &Self::INDENTED_ELSES) & (pieces.column(at) == column));
        let else_if = else_keyword.filter(|&at| pieces.is(at, "elif"));
        let else_part = else_keyword
            .filter(|&at| pieces.is(at, "else") & pieces.is(at + 1, ":"))
            .and_then(|at| IfPart::indented(pieces, at + 1, column));
        let end = else_part.map_or(then_part.end, |part| part.end);
        Some(Self {
            keyword,
            condition_from: keyword + 1,
            condition_to: colon,
            then_part,
            else_keyword,
            else_part,
            else_if,
            end,
        })
    }

    fn ended(pieces: &Pieces<'_>, keyword: usize) -> Option<Self> {
        let line_end = pieces.line_after(keyword);
        let condition_to = line_end - usize::from(pieces.is(line_end - 1, "then"));
        let column = pieces.column(keyword);
        let then_part = pieces
            .line_start(keyword)
            .then_some(line_end)
            .and_then(|at| IfPart::ended(pieces, at, column))?;
        let else_keyword = Some(then_part.end).filter(|&at| pieces.is_any(at, &Self::ENDED_ELSES));
        let else_if = else_keyword.filter(|&at| pieces.is(at, "elsif"));
        let else_part = else_keyword
            .filter(|&at| pieces.is(at, "else"))
            .and_then(|at| IfPart::ended(pieces, pieces.line_after(at), column))
            .filter(|part| pieces.is(part.end, "end"));
        let closing = else_part.map_or(then_part.end, |part| part.end);
        let finished = pieces.is(closing, "end") | else_if.is_some();
        finished.then_some(Self {
            keyword,
            condition_from: keyword + 1,
            condition_to,
            then_part,
            else_keyword,
            else_part,
            else_if,
            end: closing + 1,
        })
    }
}

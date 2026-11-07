#![allow(clippy::needless_bitwise_bool)]

use super::boolean_branch::BooleanBranch;
use super::form_pattern::FormPattern;
use super::if_statement::IfStatement;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct BooleanReturn;

impl BooleanReturn {
    pub(super) fn form(pieces: &Pieces<'_>, statement: &IfStatement) -> Option<SimplerForm> {
        let head = Some(statement).filter(|found| found.is_chain_head(pieces))?;
        let first = BooleanBranch::returned(pieces, head.then_part.from, head.then_part.to)?;
        let second = Self::second(pieces, head)?;
        let condition = BooleanBranch::condition_for(pieces, head, first.value)?;
        let last = head.else_part.map_or(second.last, |_| head.end - 1);
        let ending = BooleanBranch::ending(pieces, second.last);
        let fits = (first.value != second.value) & pieces.is_plain(head.keyword, last + 1);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::BooleanReturn,
                pieces.start(head.keyword),
                pieces.end(last),
                format!("return {condition}{ending}"),
            )
        })
    }

    fn second(pieces: &Pieces<'_>, statement: &IfStatement) -> Option<BooleanBranch> {
        let written = statement
            .else_part
            .and_then(|part| BooleanBranch::returned(pieces, part.from, part.to));
        let bare = Some(statement).filter(|found| !found.has_else());
        written.or_else(|| BooleanBranch::following_return(pieces, bare?))
    }
}

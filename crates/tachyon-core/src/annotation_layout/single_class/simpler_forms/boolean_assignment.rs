#![allow(clippy::needless_bitwise_bool)]

use super::boolean_branch::BooleanBranch;
use super::form_pattern::FormPattern;
use super::if_statement::IfStatement;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct BooleanAssignment;

impl BooleanAssignment {
    pub(super) fn form(pieces: &Pieces<'_>, statement: &IfStatement) -> Option<SimplerForm> {
        let head = Some(statement).filter(|found| found.is_chain_head(pieces))?;
        let other = head.else_part?;
        let first = BooleanBranch::assigned(pieces, head.then_part.from, head.then_part.to)?;
        let second = BooleanBranch::assigned(pieces, other.from, other.to)?;
        let condition = BooleanBranch::condition_for(pieces, head, first.value)?;
        let last = head.end - 1;
        let ending = BooleanBranch::ending(pieces, second.last);
        let same = first.target == second.target;
        let fits = same & (first.value != second.value) & pieces.is_plain(head.keyword, last + 1);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::BooleanAssignment,
                pieces.start(head.keyword),
                pieces.end(last),
                format!("{} = {condition}{ending}", first.target),
            )
        })
    }
}

#![allow(clippy::needless_bitwise_bool)]

use super::condition::Condition;
use super::condition_context::ConditionContext;
use super::expression_edges::ExpressionEdges;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct NegatedEquality;

impl NegatedEquality {
    pub(super) fn find(pieces: &Pieces<'_>, context: &ConditionContext) -> Vec<SimplerForm> {
        let mut forms: Vec<SimplerForm> = (0..pieces.len())
            .filter_map(|at| Self::grouped(pieces, at))
            .collect();
        forms.extend((0..pieces.len()).filter_map(|at| Self::python(pieces, context, at)));
        forms
    }

    fn grouped(pieces: &Pieces<'_>, at: usize) -> Option<SimplerForm> {
        let wanted = !pieces.language().is_python() & pieces.is(at, "!") & pieces.is(at + 1, "(");
        let close = pieces.partner(at + 1).filter(|_| wanted)?;
        let flipped = Condition::new(pieces, at + 2, close).flipped_equality()?;
        let edges = ExpressionEdges::is_open_left(pieces, at)
            & ExpressionEdges::is_closed_right(pieces, close + 1);
        let fits = edges & pieces.is_plain(at, close + 1);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::NegatedEquality,
                pieces.start(at),
                pieces.end(close),
                flipped,
            )
        })
    }

    fn python(pieces: &Pieces<'_>, context: &ConditionContext, at: usize) -> Option<SimplerForm> {
        let keyword =
            Some(at).filter(|&index| pieces.language().is_python() & pieces.is(index, "not"))?;
        let placed = context.starts_operand(pieces, keyword);
        let end = placed.then(|| ConditionContext::operand_end(pieces, keyword + 1))?;
        let flipped = Condition::new(pieces, at + 1, end).flipped_equality()?;
        let fits = pieces.is_plain(at, end);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::NegatedEquality,
                pieces.start(at),
                pieces.end(end - 1),
                flipped,
            )
        })
    }
}

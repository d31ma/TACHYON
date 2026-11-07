#![allow(clippy::needless_bitwise_bool)]

use super::condition_context::ConditionContext;
use super::expression_edges::ExpressionEdges;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct DoubleNegation;

impl DoubleNegation {
    pub(super) fn find(pieces: &Pieces<'_>, context: &ConditionContext) -> Vec<SimplerForm> {
        (0..pieces.len())
            .filter_map(|at| Self::form(pieces, context, at))
            .collect()
    }

    fn form(pieces: &Pieces<'_>, context: &ConditionContext, at: usize) -> Option<SimplerForm> {
        let language = pieces.language();
        let not = language.not_word().trim();
        let first = Some(at).filter(|&index| pieces.is(index, not) & pieces.is(index + 1, not))?;
        let prefix = ExpressionEdges::is_open_left(pieces, first);
        let operand = pieces.is_value_end(at + 2) | pieces.is_opener(at + 2);
        let placed = language.has_typed_conditions() | context.starts_operand(pieces, at);
        let fits = prefix & operand & placed & pieces.is_plain(at, at + 3);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::DoubleNegation,
                pieces.start(at),
                pieces.start(at + 2),
                String::new(),
            )
        })
    }
}

#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::condition::Condition;
use super::expression_edges::ExpressionEdges;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct BooleanCompare;

impl BooleanCompare {
    const LANGUAGES: [CodeLanguage; 2] = [CodeLanguage::Java, CodeLanguage::Rust];
    const COMPARISONS: [&'static str; 2] = ["==", "!="];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        let wanted = pieces.language().is_one_of(&Self::LANGUAGES);
        (0..pieces.len())
            .filter(|_| wanted)
            .filter_map(|at| Self::form(pieces, at))
            .collect()
    }

    fn form(pieces: &Pieces<'_>, at: usize) -> Option<SimplerForm> {
        let operator = Some(at).filter(|&index| pieces.is_any(index, &Self::COMPARISONS))?;
        let value_at = operator + 1;
        let language = pieces.language();
        let value = [language.false_word(), language.true_word()]
            .iter()
            .position(|word| pieces.is(value_at, word))?;
        let last = operator.wrapping_sub(1);
        let start = ExpressionEdges::chain_start(pieces, last);
        let operand = Condition::new(pieces, start, operator);
        let kept = (value == 1) == pieces.is(operator, "==");
        let text = Self::text(&operand, kept)?;
        let fits = pieces.is_value_end(last)
            & operand.is_simple()
            & ExpressionEdges::is_open_left(pieces, start)
            & ExpressionEdges::is_closed_right(pieces, value_at + 1)
            & pieces.is_plain(start, value_at + 1);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::BooleanCompare,
                pieces.start(start),
                pieces.end(value_at),
                text,
            )
        })
    }

    fn text(operand: &Condition<'_, '_>, kept: bool) -> Option<String> {
        if kept {
            return Some(operand.text().to_owned());
        }
        operand.negated()
    }
}

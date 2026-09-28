#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::condition::Condition;
use super::expression_edges::ExpressionEdges;
use super::form_pattern::FormPattern;
use super::if_statement::IfStatement;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct BooleanChoice;

impl BooleanChoice {
    const TERNARY_LANGUAGES: [CodeLanguage; 8] = [
        CodeLanguage::JavaScript,
        CodeLanguage::TypeScript,
        CodeLanguage::Java,
        CodeLanguage::CSharp,
        CodeLanguage::Php,
        CodeLanguage::Dart,
        CodeLanguage::Swift,
        CodeLanguage::Ruby,
    ];
    const IF_EXPRESSION_LANGUAGES: [CodeLanguage; 2] = [CodeLanguage::Kotlin, CodeLanguage::Rust];
    const STOPS: [&'static str; 16] = [
        "(", "[", "{", ",", ";", "=", "+=", "-=", ":=", "return", "?", ":", "=>", "yield", "throw",
        "case",
    ];

    pub(super) fn find(pieces: &Pieces<'_>, statements: &[IfStatement]) -> Vec<SimplerForm> {
        let mut forms: Vec<SimplerForm> = (0..pieces.len())
            .filter_map(|at| Self::ternary(pieces, at))
            .collect();
        forms.extend((0..pieces.len()).filter_map(|at| Self::python_choice(pieces, at)));
        forms.extend(
            statements
                .iter()
                .filter_map(|statement| Self::if_expression(pieces, statement)),
        );
        forms
    }

    fn ternary(pieces: &Pieces<'_>, question: usize) -> Option<SimplerForm> {
        let language = pieces.language();
        let wanted = language.is_one_of(&Self::TERNARY_LANGUAGES) & pieces.is(question, "?");
        let shaped =
            pieces.is(question + 2, ":") & ExpressionEdges::is_closed_right(pieces, question + 4);
        let first = Self::value(pieces, question + 1)?;
        let second = Self::value(pieces, question + 3)?;
        let start = Self::condition_start(pieces, question);
        let condition = Condition::new(pieces, start, question);
        let filled = start < question;
        let fits =
            wanted & shaped & filled & (first != second) & pieces.is_plain(start, question + 4);
        fits.then_some(condition)
            .and_then(|condition| Self::replacement(&condition, first))
            .map(|text| {
                SimplerForm::new(
                    FormPattern::BooleanChoice,
                    pieces.start(start),
                    pieces.end(question + 3),
                    text,
                )
            })
    }

    fn python_choice(pieces: &Pieces<'_>, keyword: usize) -> Option<SimplerForm> {
        let wanted =
            pieces.language().is_python() & pieces.is(keyword, "if") & !pieces.line_start(keyword);
        let first = Self::value(pieces, keyword.wrapping_sub(1))?;
        let otherwise = pieces.find_top(keyword + 1, pieces.line_after(keyword), &["else"])?;
        let second = Self::value(pieces, otherwise + 1)?;
        let open = ExpressionEdges::is_open_left(pieces, keyword - 1);
        let closed = ExpressionEdges::is_closed_right(pieces, otherwise + 2);
        let condition = Condition::new(pieces, keyword + 1, otherwise);
        let fits = wanted
            & open
            & closed
            & (first != second)
            & pieces.is_plain(keyword - 1, otherwise + 2);
        fits.then_some(condition)
            .and_then(|condition| Self::replacement(&condition, first))
            .map(|text| {
                SimplerForm::new(
                    FormPattern::BooleanChoice,
                    pieces.start(keyword - 1),
                    pieces.end(otherwise + 1),
                    text,
                )
            })
    }

    fn if_expression(pieces: &Pieces<'_>, statement: &IfStatement) -> Option<SimplerForm> {
        let language = pieces.language();
        let other = statement.else_part?;
        let wanted = language.is_one_of(&Self::IF_EXPRESSION_LANGUAGES)
            & ExpressionEdges::is_open_left(pieces, statement.keyword)
            & (statement.then_part.to == statement.then_part.from + 1)
            & (other.to == other.from + 1);
        let first = Self::value(pieces, statement.then_part.from)?;
        let second = Self::value(pieces, other.from)?;
        let last = statement.end - 1;
        let fits = wanted & (first != second) & pieces.is_plain(statement.keyword, last + 1);
        fits.then(|| statement.condition(pieces))
            .and_then(|condition| Self::replacement(&condition, first))
            .map(|text| {
                SimplerForm::new(
                    FormPattern::BooleanChoice,
                    pieces.start(statement.keyword),
                    pieces.end(last),
                    text,
                )
            })
    }

    fn value(pieces: &Pieces<'_>, at: usize) -> Option<bool> {
        let language = pieces.language();
        [language.false_word(), language.true_word()]
            .iter()
            .position(|word| pieces.is(at, word))
            .map(|index| index == 1)
    }

    fn replacement(condition: &Condition<'_, '_>, first: bool) -> Option<String> {
        if first {
            return condition.is_boolean().then(|| condition.text().to_owned());
        }
        condition.negated()
    }

    fn condition_start(pieces: &Pieces<'_>, question: usize) -> usize {
        let depth = pieces.depth(question);
        (0..question)
            .rev()
            .find(|&at| Self::stops_condition(pieces, at, depth))
            .map_or(0, |at| at + 1)
    }

    fn stops_condition(pieces: &Pieces<'_>, at: usize, depth: usize) -> bool {
        let here = pieces.depth(at) == depth;
        let listed = here & pieces.is_any(at, &Self::STOPS);
        let statement = here & pieces.is_value_end(at) & pieces.line_start(at + 1);
        (pieces.depth(at) < depth) | listed | statement
    }
}

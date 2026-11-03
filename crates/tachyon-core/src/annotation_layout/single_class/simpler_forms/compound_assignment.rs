#![allow(clippy::needless_bitwise_bool)]

use super::code_language::CodeLanguage;
use super::condition::Condition;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct CompoundAssignment;

impl CompoundAssignment {
    const OPERATORS: [&'static str; 5] = ["+", "-", "*", "/", "%"];
    const PHP_OPERATORS: [&'static str; 6] = ["+", "-", "*", "/", "%", "."];
    const LITERAL_ONLY: [CodeLanguage; 3] = [
        CodeLanguage::Python,
        CodeLanguage::Kotlin,
        CodeLanguage::Rust,
    ];
    const KNOWN_START: [CodeLanguage; 2] = [CodeLanguage::Python, CodeLanguage::Kotlin];
    const GROUPS: [&'static str; 2] = ["(", "["];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        (0..pieces.len())
            .filter_map(|at| Self::form(pieces, at))
            .collect()
    }

    fn form(pieces: &Pieces<'_>, at: usize) -> Option<SimplerForm> {
        let equals = Some(at).filter(|&index| pieces.is(index, "="))?;
        let from = pieces.line_first(equals);
        let target = Condition::new(pieces, from, equals);
        let width = equals - from;
        let operator = equals + 1 + width;
        let line_end = pieces.line_after(operator);
        let term_to = line_end - usize::from(pieces.is(line_end - 1, ";"));
        let term = Condition::new(pieces, operator + 1, term_to);
        let target_text = target.text();
        let repeated = pieces.slice(equals + 1, operator) == target_text;
        let shaped = (pieces.depth(equals) == pieces.depth(from))
            & target.is_simple()
            & !pieces.has_top(from, equals, &Self::GROUPS)
            & repeated
            & pieces.is_any(operator, Self::operators(pieces.language()))
            & term.is_simple()
            & pieces.is_plain(from, term_to);
        shaped.then_some(())?;
        let safe = Self::is_safe_term(pieces, (operator + 1, term_to), (from, equals));
        safe.then(|| {
            SimplerForm::new(
                FormPattern::CompoundAssignment,
                pieces.start(from),
                pieces.end(term_to - 1),
                format!("{target_text} {}= {}", pieces.text(operator), term.text()),
            )
        })
    }

    fn operators(language: CodeLanguage) -> &'static [&'static str] {
        [&Self::OPERATORS[..], &Self::PHP_OPERATORS][usize::from(language == CodeLanguage::Php)]
    }

    fn is_safe_term(pieces: &Pieces<'_>, term: (usize, usize), target: (usize, usize)) -> bool {
        let language = pieces.language();
        let (first, end) = term;
        let single = end == first + 1;
        let literal = single & (pieces.is_number(first) | pieces.is_literal(first));
        let number = single & pieces.is_number(first);
        let limited = language.is_one_of(&Self::LITERAL_ONLY);
        let known =
            !language.is_one_of(&Self::KNOWN_START) | Self::starts_with_literal(pieces, target);
        let rust_number = (language != CodeLanguage::Rust) | number;
        (!limited | literal) & known & rust_number
    }

    fn starts_with_literal(pieces: &Pieces<'_>, target: (usize, usize)) -> bool {
        let (from, equals) = target;
        (0..from).any(|at| Self::sets_literal(pieces, at, target)) & (equals > from)
    }

    fn sets_literal(pieces: &Pieces<'_>, at: usize, target: (usize, usize)) -> bool {
        let (from, equals) = target;
        let width = equals - from;
        let value = at + width + 1;
        let alone = pieces.line_after(value) == value + 1;
        let literal = pieces.is_number(value) | pieces.is_literal(value);
        let same = pieces.slice(at, at + width) == pieces.slice(from, equals);
        same & pieces.is(at + width, "=") & literal & alone
    }
}

use super::call_match::CallMatch;
use super::call_rewrite::CallRewrite;
use super::call_rule::CallRule;
use super::code_language::CodeLanguage;
use super::condition::Condition;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct ContainsInsteadOfIndex;

impl ContainsInsteadOfIndex {
    const TYPED: &'static [CodeLanguage] =
        &[CodeLanguage::Java, CodeLanguage::Kotlin, CodeLanguage::Dart];
    const SCRIPT_FOUND: &'static [&'static [&'static str]] = &[
        &["!==", "-", "1"],
        &["!=", "-", "1"],
        &[">", "-", "1"],
        &[">=", "0"],
    ];
    const SCRIPT_MISSING: &'static [&'static [&'static str]] =
        &[&["===", "-", "1"], &["==", "-", "1"], &["<", "0"]];
    const FOUND: &'static [&'static [&'static str]] =
        &[&["!=", "-", "1"], &[">", "-", "1"], &[">=", "0"]];
    const MISSING: &'static [&'static [&'static str]] = &[&["==", "-", "1"], &["<", "0"]];
    const NOT_FALSE: &'static [&'static [&'static str]] = &[&["!==", "false"]];
    const FALSE: &'static [&'static [&'static str]] = &[&["===", "false"]];
    const NOT_NIL: &'static [&'static [&'static str]] = &[&["!=", "nil"]];
    const NIL: &'static [&'static [&'static str]] = &[&["==", "nil"]];
    const PACKAGES: &'static [&'static str] = &["strings", "bytes"];
    const RULES: [CallRule; 16] = [
        CallRule::method(
            CallRule::SCRIPT,
            "indexOf",
            CallRule::PARENTHESES,
            Self::SCRIPT_FOUND,
            "includes",
        ),
        CallRule::method(
            CallRule::SCRIPT,
            "indexOf",
            CallRule::PARENTHESES,
            Self::SCRIPT_MISSING,
            "includes",
        )
        .negated(),
        CallRule::method(
            Self::TYPED,
            "indexOf",
            CallRule::PARENTHESES,
            Self::FOUND,
            "contains",
        )
        .arguments(1)
        .forbidden(&[CallRule::CHARACTER]),
        CallRule::method(
            Self::TYPED,
            "indexOf",
            CallRule::PARENTHESES,
            Self::MISSING,
            "contains",
        )
        .arguments(1)
        .forbidden(&[CallRule::CHARACTER])
        .negated(),
        CallRule::method(
            CallRule::GO,
            "Index",
            CallRule::PARENTHESES,
            Self::FOUND,
            "Contains",
        )
        .receivers(Self::PACKAGES),
        CallRule::method(
            CallRule::GO,
            "Index",
            CallRule::PARENTHESES,
            Self::MISSING,
            "Contains",
        )
        .receivers(Self::PACKAGES)
        .negated(),
        CallRule::method(
            CallRule::GO,
            "IndexRune",
            CallRule::PARENTHESES,
            Self::FOUND,
            "ContainsRune",
        )
        .receivers(Self::PACKAGES),
        CallRule::method(
            CallRule::GO,
            "IndexAny",
            CallRule::PARENTHESES,
            Self::FOUND,
            "ContainsAny",
        )
        .receivers(Self::PACKAGES),
        CallRule::method(
            CallRule::PHP,
            "strpos",
            CallRule::PARENTHESES,
            Self::NOT_FALSE,
            "str_contains",
        )
        .function()
        .arguments(2),
        CallRule::method(
            CallRule::PHP,
            "strpos",
            CallRule::PARENTHESES,
            Self::FALSE,
            "str_contains",
        )
        .function()
        .arguments(2)
        .negated(),
        CallRule::method(
            CallRule::PHP,
            "array_search",
            CallRule::PARENTHESES,
            Self::NOT_FALSE,
            "in_array",
        )
        .function(),
        CallRule::method(
            CallRule::PHP,
            "array_search",
            CallRule::PARENTHESES,
            Self::FALSE,
            "in_array",
        )
        .function()
        .negated(),
        CallRule::method(
            CallRule::RUBY,
            "index",
            CallRule::PARENTHESES,
            Self::NOT_NIL,
            "include?",
        )
        .arguments(1),
        CallRule::method(
            CallRule::RUBY,
            "index",
            CallRule::PARENTHESES,
            Self::NIL,
            "include?",
        )
        .arguments(1)
        .negated(),
        CallRule::method(
            CallRule::SWIFT,
            "firstIndex",
            CallRule::PARENTHESES,
            Self::NOT_NIL,
            "contains",
        )
        .dropped("of: "),
        CallRule::method(
            CallRule::SWIFT,
            "firstIndex",
            CallRule::PARENTHESES,
            Self::NIL,
            "contains",
        )
        .dropped("of: ")
        .negated(),
    ];
    const PYTHON_RULES: [CallRule; 2] = [
        CallRule::method(
            CallRule::PYTHON,
            "find",
            CallRule::PARENTHESES,
            Self::FOUND,
            "in",
        )
        .arguments(1),
        CallRule::method(
            CallRule::PYTHON,
            "find",
            CallRule::PARENTHESES,
            Self::MISSING,
            "not in",
        )
        .arguments(1),
    ];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        let mut forms =
            CallRewrite::forms(pieces, &Self::RULES, FormPattern::ContainsInsteadOfIndex);
        forms.extend(
            CallRewrite::matches(pieces, &Self::PYTHON_RULES)
                .iter()
                .map(|found| Self::membership(pieces, found)),
        );
        forms
    }

    fn membership(pieces: &Pieces<'_>, found: &CallMatch<'_>) -> SimplerForm {
        let argument = Condition::new(pieces, found.name + 2, found.group_end - 1).for_operand();
        let receiver = CallRewrite::receiver_text(pieces, found);
        SimplerForm::new(
            FormPattern::ContainsInsteadOfIndex,
            pieces.start(found.start),
            pieces.end(found.tail_end - 1),
            format!("{argument} {} {receiver}", found.rule.replacement),
        )
    }
}

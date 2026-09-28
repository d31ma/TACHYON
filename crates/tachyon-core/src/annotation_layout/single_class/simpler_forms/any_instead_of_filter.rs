use super::call_rewrite::CallRewrite;
use super::call_rule::CallRule;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct AnyInsteadOfFilter;

impl AnyInsteadOfFilter {
    const SOME: &'static [&'static str] = &[">", "0"];
    const NOT_NONE: &'static [&'static str] = &["!=", "0"];
    const AT_LEAST_ONE: &'static [&'static str] = &[">=", "1"];
    const NONE: &'static [&'static str] = &["==", "0"];
    const COUNTED: &'static [&'static [&'static str]] =
        &[Self::SOME, Self::NOT_NONE, Self::AT_LEAST_ONE];
    const RULES: [CallRule; 16] = [
        CallRule::method(
            CallRule::SCRIPT,
            "filter",
            CallRule::PARENTHESES,
            &[
                &[".", "length", ">", "0"],
                &[".", "length", "!==", "0"],
                &[".", "length", "!=", "0"],
                &[".", "length", ">=", "1"],
            ],
            "some",
        ),
        CallRule::method(
            CallRule::JAVA,
            "filter",
            CallRule::PARENTHESES,
            &[
                &[".", "count", "(", ")", ">", "0"],
                &[".", "count", "(", ")", "!=", "0"],
                &[".", "count", "(", ")", ">=", "1"],
            ],
            "anyMatch",
        ),
        CallRule::method(
            CallRule::KOTLIN,
            "filter",
            CallRule::EITHER,
            &[
                &[".", "isNotEmpty", "(", ")"],
                &[".", "size", ">", "0"],
                &[".", "size", "!=", "0"],
                &[".", "size", ">=", "1"],
                &[".", "count", "(", ")", ">", "0"],
                &[".", "count", "(", ")", "!=", "0"],
            ],
            "any",
        ),
        CallRule::method(
            CallRule::KOTLIN,
            "filter",
            CallRule::EITHER,
            &[
                &[".", "isEmpty", "(", ")"],
                &[".", "size", "==", "0"],
                &[".", "count", "(", ")", "==", "0"],
            ],
            "none",
        ),
        CallRule::method(
            CallRule::KOTLIN,
            "count",
            CallRule::EITHER,
            Self::COUNTED,
            "any",
        ),
        CallRule::method(
            CallRule::KOTLIN,
            "count",
            CallRule::EITHER,
            &[Self::NONE],
            "none",
        ),
        CallRule::method(
            CallRule::CSHARP,
            "Where",
            CallRule::PARENTHESES,
            &[
                &[".", "Any", "(", ")"],
                &[".", "Count", "(", ")", ">", "0"],
                &[".", "Count", "(", ")", "!=", "0"],
                &[".", "Count", "(", ")", ">=", "1"],
            ],
            "Any",
        ),
        CallRule::method(
            CallRule::CSHARP,
            "Count",
            CallRule::PARENTHESES,
            Self::COUNTED,
            "Any",
        ),
        CallRule::method(
            CallRule::RUBY,
            "select",
            CallRule::BRACES,
            Self::RUBY_SOME,
            "any?",
        ),
        CallRule::method(
            CallRule::RUBY,
            "filter",
            CallRule::BRACES,
            Self::RUBY_SOME,
            "any?",
        ),
        CallRule::method(
            CallRule::RUBY,
            "select",
            CallRule::BRACES,
            Self::RUBY_NONE,
            "none?",
        ),
        CallRule::method(
            CallRule::RUBY,
            "filter",
            CallRule::BRACES,
            Self::RUBY_NONE,
            "none?",
        ),
        CallRule::method(
            CallRule::RUBY,
            "count",
            CallRule::BRACES,
            Self::COUNTED,
            "any?",
        ),
        CallRule::method(
            CallRule::RUBY,
            "count",
            CallRule::BRACES,
            &[Self::NONE],
            "none?",
        ),
        CallRule::method(
            CallRule::DART,
            "where",
            CallRule::PARENTHESES,
            &[
                &[".", "isNotEmpty"],
                &[".", "length", ">", "0"],
                &[".", "length", "!=", "0"],
                &[".", "length", ">=", "1"],
            ],
            "any",
        ),
        CallRule::method(
            CallRule::SWIFT,
            "filter",
            CallRule::BRACES,
            &[
                &[".", "count", ">", "0"],
                &[".", "count", "!=", "0"],
                &[".", "count", ">=", "1"],
                &[".", "isEmpty", "==", "false"],
            ],
            "contains",
        ),
    ];
    const RUBY_SOME: &'static [&'static [&'static str]] = &[
        &[".", "length", ">", "0"],
        &[".", "size", ">", "0"],
        &[".", "count", ">", "0"],
        &[".", "length", "!=", "0"],
        &[".", "size", "!=", "0"],
        &[".", "count", "!=", "0"],
    ];
    const RUBY_NONE: &'static [&'static [&'static str]] = &[
        &[".", "empty?"],
        &[".", "length", "==", "0"],
        &[".", "size", "==", "0"],
        &[".", "count", "==", "0"],
    ];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        CallRewrite::forms(pieces, &Self::RULES, FormPattern::AnyInsteadOfFilter)
    }
}

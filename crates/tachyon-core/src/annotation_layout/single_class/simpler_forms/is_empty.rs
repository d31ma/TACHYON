use super::call_rewrite::CallRewrite;
use super::call_rule::CallRule;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;
use super::size_rule::SizeRule;

pub(super) struct IsEmpty;

impl IsEmpty {
    const SIZES: [SizeRule; 6] = [
        SizeRule {
            languages: CallRule::RUBY,
            names: &["length", "size", "count"],
            group: CallRule::NO_GROUP,
            empty: "empty?",
            filled: "",
        },
        SizeRule {
            languages: CallRule::KOTLIN,
            names: &["size", "length"],
            group: CallRule::NO_GROUP,
            empty: "isEmpty()",
            filled: "isNotEmpty()",
        },
        SizeRule {
            languages: CallRule::JAVA,
            names: &["size", "length"],
            group: CallRule::EMPTY_GROUP,
            empty: "isEmpty",
            filled: "",
        },
        SizeRule {
            languages: CallRule::DART,
            names: &["length"],
            group: CallRule::NO_GROUP,
            empty: "isEmpty",
            filled: "isNotEmpty",
        },
        SizeRule {
            languages: CallRule::SWIFT,
            names: &["count"],
            group: CallRule::NO_GROUP,
            empty: "isEmpty",
            filled: "",
        },
        SizeRule {
            languages: CallRule::RUST,
            names: &["len"],
            group: CallRule::EMPTY_GROUP,
            empty: "is_empty",
            filled: "",
        },
    ];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        let rules: Vec<CallRule> = Self::SIZES.iter().flat_map(SizeRule::call_rules).collect();
        CallRewrite::forms(pieces, &rules, FormPattern::IsEmpty)
    }
}

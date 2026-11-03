use super::call_rule::CallRule;
use super::code_language::CodeLanguage;

pub(super) struct SizeRule {
    pub(super) languages: &'static [CodeLanguage],
    pub(super) names: &'static [&'static str],
    pub(super) group: &'static [&'static str],
    pub(super) empty: &'static str,
    pub(super) filled: &'static str,
}

impl SizeRule {
    const ZERO: &'static [&'static [&'static str]] = &[&["==", "0"]];
    const MORE: &'static [&'static [&'static str]] = &[&[">", "0"], &["!=", "0"], &[">=", "1"]];

    pub(super) fn call_rules(&self) -> Vec<CallRule> {
        self.names
            .iter()
            .flat_map(|name| {
                [
                    CallRule::method(self.languages, name, self.group, Self::ZERO, self.empty),
                    CallRule::method(self.languages, name, self.group, Self::MORE, self.filled),
                ]
            })
            .filter(|rule| !rule.replacement.is_empty())
            .collect()
    }
}

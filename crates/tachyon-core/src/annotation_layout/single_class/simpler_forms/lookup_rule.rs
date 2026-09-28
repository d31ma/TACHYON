use super::code_language::CodeLanguage;

#[derive(Clone, Copy, Debug)]
pub(super) struct LookupRule {
    pub(super) language: CodeLanguage,
    pub(super) read_by_call: bool,
    pub(super) checks: &'static [&'static str],
    pub(super) result: &'static str,
}

impl LookupRule {
    pub(super) const RULES: [Self; 5] = [
        Self {
            language: CodeLanguage::Python,
            read_by_call: false,
            checks: &["{key}in{map}"],
            result: "{map}.get({key}, {default})",
        },
        Self {
            language: CodeLanguage::Ruby,
            read_by_call: false,
            checks: &["{map}.key?({key})", "{map}.has_key?({key})"],
            result: "{map}.fetch({key}, {default})",
        },
        Self {
            language: CodeLanguage::Java,
            read_by_call: true,
            checks: &["{map}.containsKey({key})"],
            result: "{map}.getOrDefault({key}, {default})",
        },
        Self {
            language: CodeLanguage::CSharp,
            read_by_call: false,
            checks: &["{map}.ContainsKey({key})"],
            result: "{map}.GetValueOrDefault({key}, {default})",
        },
        Self {
            language: CodeLanguage::Php,
            read_by_call: false,
            checks: &["isset({map}[{key}])"],
            result: "{map}[{key}] ?? {default}",
        },
    ];

    pub(super) fn of(language: CodeLanguage) -> Option<&'static Self> {
        Self::RULES.iter().find(|rule| rule.language == language)
    }

    pub(super) fn checks_key(&self, condition: &str, map: &str, key: &str) -> bool {
        let written: String = condition.split_whitespace().collect();
        self.checks.iter().any(|check| {
            Self::filled(check, map, key)
                .split_whitespace()
                .collect::<String>()
                == written
        })
    }

    pub(super) fn lookup(&self, map: &str, key: &str, default: &str) -> String {
        Self::filled(self.result, map, key).replace("{default}", default)
    }

    fn filled(template: &str, map: &str, key: &str) -> String {
        template.replace("{map}", map).replace("{key}", key)
    }
}

use super::code_language::CodeLanguage;

#[derive(Clone, Copy, Debug)]
pub(super) struct CallRule {
    pub(super) languages: &'static [CodeLanguage],
    pub(super) name: &'static str,
    pub(super) groups: &'static [&'static str],
    pub(super) tails: &'static [&'static [&'static str]],
    pub(super) replacement: &'static str,
    pub(super) negated: bool,
    pub(super) method: bool,
    pub(super) receivers: &'static [&'static str],
    pub(super) arguments: usize,
    pub(super) forbidden: &'static [&'static str],
    pub(super) dropped: &'static str,
}

impl CallRule {
    pub(super) const BARE: &'static str = "";
    pub(super) const EMPTY_PARENTHESES: &'static str = "()";
    pub(super) const CHARACTER: &'static str = "'";
    pub(super) const PARENTHESES: &'static [&'static str] = &["("];
    pub(super) const BRACES: &'static [&'static str] = &["{"];
    pub(super) const EITHER: &'static [&'static str] = &["(", "{"];
    pub(super) const NO_GROUP: &'static [&'static str] = &[Self::BARE];
    pub(super) const EMPTY_GROUP: &'static [&'static str] = &[Self::EMPTY_PARENTHESES];
    pub(super) const SCRIPT: &'static [CodeLanguage] =
        &[CodeLanguage::JavaScript, CodeLanguage::TypeScript];
    pub(super) const JAVA: &'static [CodeLanguage] = &[CodeLanguage::Java];
    pub(super) const KOTLIN: &'static [CodeLanguage] = &[CodeLanguage::Kotlin];
    pub(super) const CSHARP: &'static [CodeLanguage] = &[CodeLanguage::CSharp];
    pub(super) const RUBY: &'static [CodeLanguage] = &[CodeLanguage::Ruby];
    pub(super) const DART: &'static [CodeLanguage] = &[CodeLanguage::Dart];
    pub(super) const SWIFT: &'static [CodeLanguage] = &[CodeLanguage::Swift];
    pub(super) const RUST: &'static [CodeLanguage] = &[CodeLanguage::Rust];
    pub(super) const GO: &'static [CodeLanguage] = &[CodeLanguage::Go];
    pub(super) const PHP: &'static [CodeLanguage] = &[CodeLanguage::Php];
    pub(super) const PYTHON: &'static [CodeLanguage] = &[CodeLanguage::Python];

    pub(super) const fn method(
        languages: &'static [CodeLanguage],
        name: &'static str,
        groups: &'static [&'static str],
        tails: &'static [&'static [&'static str]],
        replacement: &'static str,
    ) -> Self {
        Self {
            languages,
            name,
            groups,
            tails,
            replacement,
            negated: false,
            method: true,
            receivers: &[],
            arguments: 0,
            forbidden: &[],
            dropped: "",
        }
    }

    pub(super) const fn negated(mut self) -> Self {
        self.negated = true;
        self
    }

    pub(super) const fn function(mut self) -> Self {
        self.method = false;
        self
    }

    pub(super) const fn receivers(mut self, receivers: &'static [&'static str]) -> Self {
        self.receivers = receivers;
        self
    }

    pub(super) const fn arguments(mut self, arguments: usize) -> Self {
        self.arguments = arguments;
        self
    }

    pub(super) const fn forbidden(mut self, forbidden: &'static [&'static str]) -> Self {
        self.forbidden = forbidden;
        self
    }

    pub(super) const fn dropped(mut self, dropped: &'static str) -> Self {
        self.dropped = dropped;
        self
    }
}

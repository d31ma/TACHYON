#[derive(Clone, Copy, Debug)]
pub(super) struct TestLanguage {
    pub(super) logic_words: &'static [&'static str],
    pub(super) statement_asserts: &'static [&'static str],
    pub(super) macro_asserts: &'static [&'static str],
    pub(super) assert_help: &'static str,
}

impl TestLanguage {
    const JAVASCRIPT: Self = Self {
        logic_words: &["if", "else", "for", "while", "do", "switch", "case"],
        statement_asserts: &[],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.isTrue(condition)` or `Assert.throws(action)`.",
    };
    const PYTHON: Self = Self {
        logic_words: &["if", "elif", "else", "for", "while"],
        statement_asserts: &["assert"],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.is_true(condition)`, `Assert.throws(action)` or an `assert` statement.",
    };
    const RUST: Self = Self {
        logic_words: &["if", "else", "match", "for", "while", "loop"],
        statement_asserts: &[],
        macro_asserts: &["assert", "assert_eq", "assert_ne"],
        assert_help: "Check the result with `Assert::equals(expected, actual)`, `Assert::is_true(condition)`, `Assert::throws(action)`, `assert!`, `assert_eq!` or `assert_ne!`.",
    };
    const LANGUAGES: [(&'static str, Self); 4] = [
        ("js", Self::JAVASCRIPT),
        ("ts", Self::JAVASCRIPT),
        ("py", Self::PYTHON),
        ("rs", Self::RUST),
    ];

    pub(super) fn of(extension: &str) -> Option<Self> {
        Self::LANGUAGES
            .iter()
            .find(|(name, _)| *name == extension)
            .map(|(_, language)| *language)
    }
}

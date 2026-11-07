#[derive(Clone, Copy, Debug)]
pub(super) struct TestLanguage {
    pub(super) case_prefix: &'static str,
    pub(super) logic_words: &'static [&'static str],
    pub(super) statement_asserts: &'static [&'static str],
    pub(super) macro_asserts: &'static [&'static str],
    pub(super) assert_help: &'static str,
}

impl TestLanguage {
    const JAVASCRIPT: Self = Self {
        case_prefix: "test",
        logic_words: &["if", "else", "for", "while", "do", "switch", "case"],
        statement_asserts: &[],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.isTrue(condition)` or `Assert.throws(action)`.",
    };
    const PYTHON: Self = Self {
        case_prefix: "test",
        logic_words: &["if", "elif", "else", "for", "while"],
        statement_asserts: &["assert"],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.is_true(condition)`, `Assert.throws(action)` or an `assert` statement.",
    };
    const RUST: Self = Self {
        case_prefix: "test",
        logic_words: &["if", "else", "match", "for", "while", "loop"],
        statement_asserts: &[],
        macro_asserts: &["assert", "assert_eq", "assert_ne"],
        assert_help: "Check the result with `Assert::equals(expected, actual)`, `Assert::is_true(condition)`, `Assert::throws(action)`, `assert!`, `assert_eq!` or `assert_ne!`.",
    };
    const JAVA: Self = Self {
        case_prefix: "test",
        logic_words: &["if", "else", "for", "while", "do", "switch", "case"],
        statement_asserts: &["assert"],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.isTrue(condition)`, `Assert.throwsError(action)` or an `assert` statement.",
    };
    const KOTLIN: Self = Self {
        case_prefix: "test",
        logic_words: &["if", "else", "for", "while", "do", "when"],
        statement_asserts: &["check", "require"],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.isTrue(condition)`, `Assert.throws { action }`, `check(condition)` or `require(condition)`.",
    };
    const CSHARP: Self = Self {
        case_prefix: "test",
        logic_words: &[
            "if", "else", "for", "foreach", "while", "do", "switch", "case", "when",
        ],
        statement_asserts: &[],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.isTrue(condition)` or `Assert.throws(action)`.",
    };
    const GO: Self = Self {
        case_prefix: "Test",
        logic_words: &["if", "else", "for", "switch", "select", "case", "goto"],
        statement_asserts: &[],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.Equals(expected, actual)`, `Assert.IsTrue(condition)` or `Assert.Throws(action)`.",
    };
    const RUBY: Self = Self {
        case_prefix: "test",
        logic_words: &[
            "if", "elsif", "else", "unless", "until", "while", "for", "case", "when", "loop",
        ],
        statement_asserts: &[],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.is_true(condition)` or `Assert.throws { action }`.",
    };
    const PHP: Self = Self {
        case_prefix: "test",
        logic_words: &[
            "if", "elseif", "else", "for", "foreach", "while", "do", "switch", "case", "match",
        ],
        statement_asserts: &["assert"],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert::equals($expected, $actual)`, `Assert::isTrue($condition)`, `Assert::throws($action)` or `assert($condition)`.",
    };
    const DART: Self = Self {
        case_prefix: "test",
        logic_words: &["if", "else", "for", "while", "do", "switch", "case"],
        statement_asserts: &["assert"],
        macro_asserts: &[],
        assert_help: "Check the result with `Assert.equals(expected, actual)`, `Assert.isTrue(condition)`, `await Assert.throws(action)` or an `assert` statement.",
    };
    const SWIFT: Self = Self {
        case_prefix: "test",
        logic_words: &[
            "if", "guard", "else", "for", "while", "repeat", "switch", "case",
        ],
        statement_asserts: &["precondition"],
        macro_asserts: &[],
        assert_help: "Check the result with `try Assert.equals(expected, actual)`, `try Assert.isTrue(condition)`, `try await Assert.throws { action }` or `precondition(condition)`.",
    };
    const LANGUAGES: [(&'static [&'static str], Self); 11] = [
        (&["js", "ts"], Self::JAVASCRIPT),
        (&["py"], Self::PYTHON),
        (&["rs"], Self::RUST),
        (&["java", "groovy"], Self::JAVA),
        (&["kt"], Self::KOTLIN),
        (&["cs"], Self::CSHARP),
        (&["go"], Self::GO),
        (&["rb"], Self::RUBY),
        (&["php"], Self::PHP),
        (&["dart"], Self::DART),
        (&["swift"], Self::SWIFT),
    ];

    pub(super) fn of(extension: &str) -> Option<Self> {
        Self::LANGUAGES
            .iter()
            .find(|(extensions, _)| extensions.contains(&extension))
            .map(|(_, language)| *language)
    }
}

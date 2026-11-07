#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CodeLanguage {
    JavaScript,
    TypeScript,
    Python,
    Ruby,
    Java,
    Kotlin,
    CSharp,
    Php,
    Go,
    Rust,
    Swift,
    Dart,
}

impl CodeLanguage {
    const EXTENSIONS: [&'static str; 12] = [
        "js", "ts", "py", "rb", "java", "kt", "cs", "php", "go", "rs", "swift", "dart",
    ];
    const ALL: [Self; 12] = [
        Self::JavaScript,
        Self::TypeScript,
        Self::Python,
        Self::Ruby,
        Self::Java,
        Self::Kotlin,
        Self::CSharp,
        Self::Php,
        Self::Go,
        Self::Rust,
        Self::Swift,
        Self::Dart,
    ];
    const TYPED_CONDITIONS: [Self; 7] = [
        Self::Java,
        Self::Kotlin,
        Self::CSharp,
        Self::Go,
        Self::Rust,
        Self::Swift,
        Self::Dart,
    ];
    const BARE_CONDITIONS: [Self; 5] =
        [Self::Go, Self::Rust, Self::Swift, Self::Python, Self::Ruby];
    const BLOCK_SCOPES: [Self; 9] = [
        Self::JavaScript,
        Self::TypeScript,
        Self::Java,
        Self::Kotlin,
        Self::CSharp,
        Self::Go,
        Self::Rust,
        Self::Swift,
        Self::Dart,
    ];
    const RUBY_LOOSE: [&'static str; 7] = ["||", "or", "and", "not", "?", "=", "unless"];
    const PYTHON_LOOSE: [&'static str; 5] = ["or", "if", "lambda", ":=", "else"];
    const BRACED_LOOSE: [&'static str; 6] = ["||", "??", "?", "=", ",", "?:"];
    const RUBY_EXITS: [&'static str; 4] = ["return", "raise", "break", "next"];
    const PYTHON_EXITS: [&'static str; 4] = ["return", "raise", "break", "continue"];
    const BRACED_EXITS: [&'static str; 4] = ["return", "throw", "break", "continue"];

    pub(super) fn of(extension: &str) -> Option<Self> {
        Self::from_extension(extension)
    }

    fn from_extension(extension: &str) -> Option<Self> {
        Self::EXTENSIONS
            .iter()
            .position(|name| *name == extension)
            .map(|index| Self::ALL[index])
    }

    pub(super) fn is_python(self) -> bool {
        self == Self::Python
    }

    pub(super) fn is_ruby(self) -> bool {
        self == Self::Ruby
    }

    pub(super) fn is_braced(self) -> bool {
        !(self.is_python() | self.is_ruby())
    }

    pub(super) fn is_one_of(self, languages: &[Self]) -> bool {
        languages.contains(&self)
    }

    pub(super) fn has_typed_conditions(self) -> bool {
        Self::TYPED_CONDITIONS.contains(&self)
    }

    pub(super) fn wraps_conditions(self) -> bool {
        !Self::BARE_CONDITIONS.contains(&self)
    }

    pub(super) fn has_block_scope(self) -> bool {
        Self::BLOCK_SCOPES.contains(&self)
    }

    pub(super) fn true_word(self) -> &'static str {
        ["true", "True"][usize::from(self.is_python())]
    }

    pub(super) fn false_word(self) -> &'static str {
        ["false", "False"][usize::from(self.is_python())]
    }

    pub(super) fn not_word(self) -> &'static str {
        ["!", "not "][usize::from(self.is_python())]
    }

    pub(super) fn and_word(self) -> &'static str {
        ["&&", "and"][usize::from(self.is_python())]
    }

    pub(super) fn looser_than_and(self) -> &'static [&'static str] {
        [
            &Self::BRACED_LOOSE[..],
            &Self::PYTHON_LOOSE,
            &Self::RUBY_LOOSE,
        ][self.family()]
    }

    pub(super) fn exit_words(self) -> &'static [&'static str] {
        [&Self::BRACED_EXITS, &Self::PYTHON_EXITS, &Self::RUBY_EXITS][self.family()]
    }

    fn family(self) -> usize {
        usize::from(self.is_python()) + 2 * usize::from(self.is_ruby())
    }
}

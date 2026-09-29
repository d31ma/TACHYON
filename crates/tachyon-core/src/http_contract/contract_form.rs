#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContractForm {
    pub(crate) language: &'static str,
    marker_prefix: &'static str,
    indented: bool,
    contract: &'static str,
    refused: &'static str,
    words: &'static str,
}

impl ContractForm {
    const INTERFACE_WORDS: &'static str = "an `interface`";
    const EMPTY_METHOD_WORDS: &'static str = "a class whose methods have empty bodies";
    const TYPESCRIPT_WORDS: &'static str = "a class whose methods have empty bodies (TypeScript allows annotations only on methods with a body)";
    const FORMS: [Self; 13] = [
        Self::braced("java", "@", "interface", Self::INTERFACE_WORDS),
        Self::braced("groovy", "@", "interface", Self::INTERFACE_WORDS),
        Self::braced("kt", "@", "interface", Self::INTERFACE_WORDS),
        Self::braced("cs", "[", "interface", Self::INTERFACE_WORDS),
        Self::braced("php", "#[", "interface", Self::INTERFACE_WORDS),
        Self::braced(
            "go",
            "@",
            "interface",
            "a Go interface: `type CatalogClient interface { … }`",
        ),
        Self::braced(
            "dart",
            "@",
            "abstract interface class",
            "an `abstract interface class`",
        ),
        Self::braced("rs", "#[", "trait", "a `trait`"),
        Self::braced("swift", "@", "protocol", "a `protocol`"),
        Self {
            language: "py",
            marker_prefix: "@",
            indented: true,
            contract: "Protocol",
            refused: "",
            words: "a `Protocol` class: `class CatalogClient(Protocol):` with `...` as each method body",
        },
        Self::class_form("js", "@", false, Self::EMPTY_METHOD_WORDS),
        Self::class_form("ts", "@", false, Self::TYPESCRIPT_WORDS),
        Self::class_form("rb", "#[", true, Self::EMPTY_METHOD_WORDS),
    ];
    const CLASS: &'static str = "class";
    const ABSTRACT: &'static str = "abstract";
    const SEMICOLON_ENDED: [&'static str; 5] = ["java", "cs", "php", "dart", "rs"];
    const WORD_BREAKS: [char; 7] = ['(', ')', ',', ':', '{', '.', ';'];

    const fn braced(
        language: &'static str,
        marker_prefix: &'static str,
        contract: &'static str,
        words: &'static str,
    ) -> Self {
        Self {
            language,
            marker_prefix,
            indented: false,
            contract,
            refused: "",
            words,
        }
    }

    const fn class_form(
        language: &'static str,
        marker_prefix: &'static str,
        indented: bool,
        words: &'static str,
    ) -> Self {
        Self {
            language,
            marker_prefix,
            indented,
            contract: Self::CLASS,
            refused: Self::ABSTRACT,
            words,
        }
    }

    pub(crate) fn of(extension: &str) -> Option<Self> {
        Self::FORMS
            .into_iter()
            .find(|form| form.language == extension)
    }

    pub(crate) fn has_contract(self) -> bool {
        self.contract != Self::CLASS
    }

    pub(crate) fn ends_with_semicolon(self) -> bool {
        Self::SEMICOLON_ENDED.contains(&self.language)
    }

    pub(crate) fn is_indented(self) -> bool {
        self.indented
    }

    pub(crate) fn marker_prefix(self) -> &'static str {
        self.marker_prefix
    }

    pub(crate) fn contract_words(self) -> &'static str {
        self.words
    }

    pub(crate) fn declares_contract(self, header: &str) -> bool {
        let words = Self::spaced_words(header);
        let named = words.contains(&format!(" {} ", self.contract));
        let refused = words.contains(&format!(" {} ", self.refused));
        [named, refused] == [true, false]
    }

    fn spaced_words(header: &str) -> String {
        let spaced = header.replace(Self::WORD_BREAKS, " ");
        let words: Vec<&str> = spaced.split_whitespace().collect();
        format!(" {} ", words.join(" "))
    }
}

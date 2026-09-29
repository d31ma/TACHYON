#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContractForm {
    pub(crate) language: &'static str,
}

impl ContractForm {
    const LANGUAGES: [&'static str; 13] = [
        "java", "groovy", "kt", "cs", "php", "go", "dart", "rs", "swift", "py", "js", "ts", "rb",
    ];
    const CLASS_FORM: [&'static str; 3] = ["js", "ts", "rb"];
    const SEMICOLON_ENDED: [&'static str; 5] = ["java", "cs", "php", "dart", "rs"];

    pub(crate) fn of(extension: &str) -> Option<Self> {
        Self::LANGUAGES
            .into_iter()
            .find(|language| *language == extension)
            .map(|language| Self { language })
    }

    pub(crate) fn has_contract(self) -> bool {
        !Self::CLASS_FORM.contains(&self.language)
    }

    pub(crate) fn ends_with_semicolon(self) -> bool {
        Self::SEMICOLON_ENDED.contains(&self.language)
    }

    pub(crate) fn is_indented(self) -> bool {
        matches!(self.language, "py" | "rb")
    }

    pub(crate) fn marker_prefix(self) -> &'static str {
        match self.language {
            "rs" | "php" | "rb" => "#[",
            "cs" => "[",
            _ => "@",
        }
    }

    pub(crate) fn declares_contract(self, header: &str) -> bool {
        let words: Vec<&str> = header.split_whitespace().collect();
        match self.language {
            "go" => words.get(2) == Some(&"interface") || header.contains(" interface{"),
            "dart" => words
                .windows(3)
                .any(|window| window == ["abstract", "interface", "class"]),
            "rs" => words.contains(&"trait"),
            "swift" => words.contains(&"protocol"),
            "py" => Self::names_protocol(header),
            "js" | "ts" | "rb" => words.contains(&"class") && !words.contains(&"abstract"),
            _ => words.contains(&"interface"),
        }
    }

    fn names_protocol(header: &str) -> bool {
        let Some(open) = header.find('(') else {
            return false;
        };
        let bases = header[open + 1..].split(')').next().unwrap_or("");
        bases
            .split(',')
            .map(str::trim)
            .any(|base| base == "Protocol" || base == "typing.Protocol")
    }

    pub(crate) fn contract_words(self) -> &'static str {
        match self.language {
            "go" => "a Go interface: `type CatalogClient interface { … }`",
            "dart" => "an `abstract interface class`",
            "rs" => "a `trait`",
            "swift" => "a `protocol`",
            "py" => {
                "a `Protocol` class: `class CatalogClient(Protocol):` with `...` as each method body"
            }
            "js" | "rb" => "a class whose methods have empty bodies",
            "ts" => {
                "a class whose methods have empty bodies (TypeScript allows annotations only on methods with a body)"
            }
            _ => "an `interface`",
        }
    }
}

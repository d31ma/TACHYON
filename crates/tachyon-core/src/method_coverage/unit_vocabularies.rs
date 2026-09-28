use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub(crate) struct UnitVocabularies {
    vocabularies: Vec<BTreeSet<String>>,
}

impl UnitVocabularies {
    const REALMS: [&'static str; 2] = ["client", "server"];

    pub(crate) fn read(project: &Path) -> Self {
        let vocabularies = Self::REALMS
            .iter()
            .flat_map(|realm| crate::client_tests::units(project, realm))
            .filter_map(|unit| std::fs::read_to_string(project.join(unit)).ok())
            .map(|source| Self::words(&source))
            .collect();
        Self { vocabularies }
    }

    pub(crate) fn names(&self, type_name: &str, method: &str) -> bool {
        self.vocabularies
            .iter()
            .any(|vocabulary| Self::names_both(vocabulary, type_name, method))
    }

    fn names_both(vocabulary: &BTreeSet<String>, type_name: &str, method: &str) -> bool {
        vocabulary.contains(type_name) && vocabulary.contains(method)
    }

    fn words(source: &str) -> BTreeSet<String> {
        source
            .split(|value: char| !Self::is_word_character(value))
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn is_word_character(value: char) -> bool {
        value.is_ascii_alphanumeric() || value == '_' || value == '$'
    }
}

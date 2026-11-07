use std::path::Path;

pub(super) struct UtilityName;

impl UtilityName {
    const FALLBACK: &'static str = "Shared";
    const ENTRY_NAMES: [&'static str; 2] = ["tac", "yon"];
    const ROLE_WORDS: [&'static str; 16] = [
        "Client",
        "Controller",
        "Converter",
        "Entity",
        "Error",
        "Gateway",
        "Handler",
        "Delegate",
        "Middleware",
        "Model",
        "Service",
        "Setup",
        "Store",
        "Test",
        "Utils",
        "Worker",
    ];

    pub(super) fn suggest(first: &str, second: &str) -> String {
        let first_words = Self::topic_words(first);
        let second_words = Self::topic_words(second);
        let shared = Self::common_start(&first_words, &second_words);
        Self::name_or_fallback(shared)
    }

    fn topic_words(path: &str) -> Vec<String> {
        Self::topic(Path::new(path))
            .split(|character: char| !character.is_ascii_alphanumeric())
            .flat_map(Self::camel_words)
            .filter(|word| !Self::ROLE_WORDS.contains(&word.as_str()))
            .collect()
    }

    fn topic(path: &Path) -> &str {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let folder = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|value| value.to_str())
            .unwrap_or("");
        [stem, folder][usize::from(Self::ENTRY_NAMES.contains(&stem))]
    }

    fn camel_words(piece: &str) -> Vec<String> {
        let mut words: Vec<String> = Vec::new();
        for character in piece.chars() {
            let starts_word = character.is_ascii_uppercase() | words.is_empty();
            words.extend(starts_word.then(String::new));
            let last = words.len() - 1;
            words[last].push(character);
        }
        words.iter().map(|word| Self::capitalized(word)).collect()
    }

    fn capitalized(word: &str) -> String {
        let mut characters = word.chars();
        let first = characters.next().map(|value| value.to_ascii_uppercase());
        first.into_iter().chain(characters).collect()
    }

    fn common_start(first: &[String], second: &[String]) -> String {
        first
            .iter()
            .zip(second)
            .take_while(|(left, right)| left == right)
            .map(|(left, _)| left.as_str())
            .collect()
    }

    fn name_or_fallback(shared: String) -> String {
        Some(shared)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| Self::FALLBACK.to_owned())
    }
}

#![allow(clippy::needless_bitwise_bool)]

pub(super) struct NameParts;

impl NameParts {
    pub(super) fn split(name: &str) -> Vec<String> {
        let characters: Vec<char> = name.chars().collect();
        let parts = Self::collect(&characters);
        parts
            .into_iter()
            .filter(|part| part.chars().any(char::is_alphabetic))
            .collect()
    }

    pub(super) fn lower(name: &str) -> String {
        Self::split(name).join("_")
    }

    fn collect(characters: &[char]) -> Vec<String> {
        let mut parts = vec![String::new()];
        for at in 0..characters.len() {
            Self::place(characters, at, &mut parts);
        }
        parts
    }

    fn place(characters: &[char], at: usize, parts: &mut Vec<String>) {
        let character = characters[at];
        let separator = !character.is_alphanumeric();
        if separator | Self::starts_part(characters, at) {
            parts.push(String::new());
        }
        let kept = (!separator).then(|| character.to_ascii_lowercase());
        let last = parts.len() - 1;
        parts[last].extend(kept);
    }

    fn starts_part(characters: &[char], at: usize) -> bool {
        let previous = Self::character(characters, at.wrapping_sub(1));
        let current = Self::character(characters, at);
        let next = Self::character(characters, at + 1);
        let camel = previous.is_lowercase() & current.is_uppercase();
        let acronym_end = previous.is_uppercase() & current.is_uppercase() & next.is_lowercase();
        let digits = previous.is_ascii_digit() != current.is_ascii_digit();
        camel | acronym_end | (digits & previous.is_alphanumeric())
    }

    fn character(characters: &[char], at: usize) -> char {
        characters.get(at).copied().unwrap_or(' ')
    }
}

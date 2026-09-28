use crate::structure::Structure;
use std::path::Path;

#[cfg(test)]
mod tests;

pub(crate) struct FileName;

impl FileName {
    const MODULE_FILES: [&'static str; 4] = ["mod", "lib", "main", "build"];
    const NO_CHARACTER: char = ' ';

    pub(crate) fn wanted(relative: &str, structure: &Structure) -> Option<String> {
        let stem = Path::new(relative)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let names: Vec<String> = structure
            .types
            .iter()
            .map(|name| Self::snake_case(name))
            .collect();
        let passes = [
            Self::MODULE_FILES.contains(&stem.as_ref()),
            names.is_empty(),
            names.contains(&stem.to_string()),
        ];
        (!passes.contains(&true)).then(|| format!("{}.rs", names[0]))
    }

    pub(crate) fn snake_case(name: &str) -> String {
        let characters: Vec<char> = name.chars().collect();
        (0..characters.len())
            .map(|index| Self::snake_piece(&characters, index))
            .collect()
    }

    fn snake_piece(characters: &[char], index: usize) -> String {
        let lower = characters[index].to_ascii_lowercase().to_string();
        if Self::starts_word(characters, index) {
            return format!("_{lower}");
        }
        lower
    }

    fn starts_word(characters: &[char], index: usize) -> bool {
        let current = characters[index];
        let previous = Self::character_at(characters, index.checked_sub(1));
        let next = Self::character_at(characters, Some(index + 1));
        let after_lower =
            [previous.is_ascii_lowercase(), previous.is_ascii_digit()].contains(&true);
        let ends_acronym =
            [previous.is_ascii_uppercase(), next.is_ascii_lowercase()] == [true, true];
        let new_word = [after_lower, ends_acronym].contains(&true);
        [current.is_ascii_uppercase(), new_word] == [true, true]
    }

    fn character_at(characters: &[char], index: Option<usize>) -> char {
        let found = index.and_then(|at| characters.get(at));
        found.copied().unwrap_or(Self::NO_CHARACTER)
    }
}

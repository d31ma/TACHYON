use std::path::Path;

pub(super) struct CodeText;

impl CodeText {
    const LANGUAGES: &'static str = "js mjs cjs ts py php java kt cs rs dart go rb swift";
    const MAX_BYTES: usize = 1024 * 1024;

    pub(super) fn source<'a>(path: &Path, bytes: &'a [u8]) -> &'a str {
        let readable = [Self::is_code(path), bytes.len() <= Self::MAX_BYTES] == [true, true];
        let text = readable.then(|| std::str::from_utf8(bytes).ok()).flatten();
        text.unwrap_or("")
    }

    fn is_code(path: &Path) -> bool {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Self::LANGUAGES
            .split(' ')
            .any(|language| language == extension)
    }

    pub(super) fn of(extension: &str, source: &str) -> String {
        let mask = crate::lexical::code_mask(extension, source);
        Self::masked(source, &mask.code)
    }

    fn masked(source: &str, code: &[bool]) -> String {
        source
            .char_indices()
            .map(|(at, character)| Self::visible(character, code.get(at).copied()))
            .collect()
    }

    fn visible(character: char, code: Option<bool>) -> String {
        if [character == '\n', code == Some(true)].contains(&true) {
            return String::from(character);
        }
        " ".repeat(character.len_utf8())
    }
}

use super::MethodHeader;

pub(crate) struct BodyBraceFinder;

impl BodyBraceFinder {
    const TYPED_LANGUAGES: [&'static str; 2] = ["ts", "go"];
    const TYPE_WORDS: [&'static str; 3] = ["struct", "interface", "is"];
    const TYPE_CLOSERS: [char; 4] = ['>', ']', ')', '}'];
    const QUOTES: [char; 3] = ['\'', '"', '`'];
    const ARROW: &'static str = "=>";
    const INTERPOLATION: char = '$';
    const GENERATOR: char = '*';
    const TYPESCRIPT_MODIFIERS: [&'static str; 5] = ["public", "async", "override", "get", "set"];

    pub(crate) fn find(language: &str, masked: &str, source: &str, from: usize) -> Option<usize> {
        let typed = Self::TYPED_LANGUAGES.contains(&language);
        let mut depth = 0_i64;
        for (at, byte) in masked.bytes().enumerate().skip(from) {
            let opens_at_top = byte == b'{' && depth == 0;
            if opens_at_top && (!typed || Self::follows_type(&masked[..at], source.get(..at))) {
                return Some(at);
            }
            depth += match byte {
                b'(' | b'[' | b'{' => 1,
                b')' | b']' | b'}' => -1,
                _ => 0,
            };
            if depth < 0 || (byte == b';' && depth == 0) {
                return None;
            }
        }
        None
    }

    pub(crate) fn opens_typescript_body(
        masked_line: &str,
        source_line: &str,
        after_parameters: usize,
        modifiers: &str,
    ) -> bool {
        let line = masked_line.trim_end();
        let modified = modifiers
            .split(|character: char| character.is_whitespace() || character == Self::GENERATOR)
            .filter(|word| !word.is_empty())
            .all(|word| Self::TYPESCRIPT_MODIFIERS.contains(&word));
        let whole_line =
            Self::find("ts", line, source_line, after_parameters).is_some_and(|open| {
                let closes_line = MethodHeader::closing(line, open) == Some(line.len());
                open + 1 == line.len() || closes_line
            });
        modified && whole_line
    }

    fn follows_type(masked_before: &str, source_before: Option<&str>) -> bool {
        let quoted = source_before.is_some_and(|text| text.trim_end().ends_with(Self::QUOTES));
        let before = masked_before.trim_end();
        let word = before
            .rsplit(|character: char| !Self::word_character(character))
            .next()
            .unwrap_or_default();
        let ends_type = before
            .chars()
            .next_back()
            .is_some_and(|last| Self::word_character(last) || Self::TYPE_CLOSERS.contains(&last));
        let after_type = ends_type
            && !before.ends_with(Self::ARROW)
            && !before.ends_with(Self::INTERPOLATION)
            && !Self::TYPE_WORDS.contains(&word);
        quoted || after_type
    }

    fn word_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || ['_', '$'].contains(&character)
    }
}

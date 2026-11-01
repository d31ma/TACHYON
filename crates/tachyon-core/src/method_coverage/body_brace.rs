use super::MethodHeader;

pub(crate) struct BodyBrace;

impl BodyBrace {
    const TYPE_WORDS: [&'static str; 3] = ["struct", "interface", "is"];
    const TYPE_CLOSERS: [char; 4] = ['>', ']', ')', '}'];
    const ARROW: &'static str = "=>";
    const TYPESCRIPT_MODIFIERS: [&'static str; 6] =
        ["public", "async", "override", "get", "set", "*"];

    pub(crate) fn find(text: &str, from: usize) -> Option<usize> {
        let mut depth = 0_i64;
        for (at, byte) in text.bytes().enumerate().skip(from) {
            if [byte == b'{', depth == 0] == [true, true] && Self::follows_type(&text[..at]) {
                return Some(at);
            }
            depth += match byte {
                b'(' | b'[' | b'{' => 1,
                b')' | b']' | b'}' => -1,
                _ => 0,
            };
            if depth < 0 || [byte == b';', depth == 0] == [true, true] {
                return None;
            }
        }
        None
    }

    pub(crate) fn opens_typescript_body(
        line: &str,
        after_parameters: usize,
        modifiers: &str,
    ) -> bool {
        let line = line.trim_end();
        let modified = modifiers
            .split_whitespace()
            .all(|word| Self::TYPESCRIPT_MODIFIERS.contains(&word));
        let whole_line = Self::find(line, after_parameters).is_some_and(|open| {
            [
                open + 1,
                MethodHeader::closing(line, open).unwrap_or_default(),
            ]
            .contains(&line.len())
        });
        [modified, whole_line] == [true, true]
    }

    fn follows_type(before: &str) -> bool {
        let before = before.trim_end();
        let word = before
            .rsplit(|character: char| !Self::word_character(character))
            .next()
            .unwrap_or_default();
        let ends_type = before
            .chars()
            .next_back()
            .is_some_and(|last| Self::word_character(last) || Self::TYPE_CLOSERS.contains(&last));
        [
            ends_type,
            !before.ends_with(Self::ARROW),
            !Self::TYPE_WORDS.contains(&word),
        ] == [true; 3]
    }

    fn word_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || ['_', '$'].contains(&character)
    }
}

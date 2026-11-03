pub(super) struct LuaTypes;

impl LuaTypes {
    const PLAIN: [(&'static str, &'static str); 9] = [
        ("string", "String"),
        ("integer", "Integer"),
        ("number", "Float"),
        ("boolean", "Boolean"),
        ("nil", "nil"),
        ("any", "Object"),
        ("table", "Hash"),
        ("function", "Proc"),
        ("self", "self"),
    ];
    const OPENERS: [char; 3] = ['<', '(', '{'];
    const CLOSERS: [char; 3] = ['>', ')', '}'];
    const LIST_SUFFIX: &'static str = "[]";
    const OPTIONAL_SUFFIX: char = '?';
    const MAP_START: &'static str = "table<";
    const MAP_END: char = '>';
    const FUNCTION_START: &'static str = "fun(";
    const FUNCTION_TYPE: &'static str = "Proc";
    const NOTHING: &'static str = "void";

    pub(super) fn yard(lua: &str) -> String {
        Self::written_type(lua.trim())
    }

    fn written_type(text: &str) -> String {
        if text.is_empty() {
            return String::from(Self::NOTHING);
        }
        Self::union(text)
    }

    pub(super) fn type_word(text: &str) -> (&str, &str) {
        let end = Self::word_end(text);
        (&text[..end], text[end..].trim_start())
    }

    fn union(text: &str) -> String {
        Self::split_top(text, '|')
            .iter()
            .map(|part| Self::single(part.trim()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn word_end(text: &str) -> usize {
        let mut depth = 0;
        text.char_indices()
            .find(|(_, character)| Self::ends_word(*character, &mut depth))
            .map_or(text.len(), |(at, _)| at)
    }

    fn ends_word(character: char, depth: &mut i32) -> bool {
        *depth += Self::depth_change(character);
        let separates = [character.is_whitespace(), character == ','].contains(&true);
        [*depth == 0, separates] == [true, true]
    }

    fn depth_change(character: char) -> i32 {
        i32::from(Self::OPENERS.contains(&character))
            - i32::from(Self::CLOSERS.contains(&character))
    }

    fn single(text: &str) -> String {
        Self::optional(text)
            .or_else(|| Self::list(text))
            .or_else(|| Self::map_type(text))
            .or_else(|| Self::function(text))
            .unwrap_or_else(|| Self::plain(text))
    }

    fn optional(text: &str) -> Option<String> {
        text.strip_suffix(Self::OPTIONAL_SUFFIX)
            .map(|inner| format!("{}, nil", Self::single(inner)))
    }

    fn list(text: &str) -> Option<String> {
        text.strip_suffix(Self::LIST_SUFFIX)
            .map(|item| format!("Array<{}>", Self::single(item)))
    }

    fn map_type(text: &str) -> Option<String> {
        text.strip_prefix(Self::MAP_START)
            .and_then(|rest| rest.strip_suffix(Self::MAP_END))
            .map(Self::map)
    }

    fn function(text: &str) -> Option<String> {
        text.starts_with(Self::FUNCTION_START)
            .then(|| String::from(Self::FUNCTION_TYPE))
    }

    fn plain(text: &str) -> String {
        Self::PLAIN
            .iter()
            .find(|(lua, _)| *lua == text)
            .map_or_else(|| text.to_owned(), |(_, ruby)| (*ruby).to_owned())
    }

    fn map(inner: &str) -> String {
        let parts = Self::split_top(inner, ',');
        let key = parts.first().map_or("", |part| part.trim());
        let value = parts.get(1).map_or("", |part| part.trim());
        format!("Hash{{{} => {}}}", Self::single(key), Self::single(value))
    }

    fn split_top(text: &str, separator: char) -> Vec<&str> {
        let mut depth = 0;
        let points = text
            .char_indices()
            .filter_map(|(at, character)| Self::split_point(at, character, separator, &mut depth))
            .collect::<Vec<_>>();
        Self::parts(text, &points)
    }

    fn split_point(at: usize, character: char, separator: char, depth: &mut i32) -> Option<usize> {
        *depth += Self::depth_change(character);
        ([*depth == 0, character == separator] == [true, true]).then_some(at)
    }

    fn parts<'t>(text: &'t str, points: &[usize]) -> Vec<&'t str> {
        let mut parts = Vec::with_capacity(points.len() + 1);
        let mut start = 0;
        for at in points {
            parts.push(&text[start..*at]);
            start = at + 1;
        }
        parts.push(&text[start..]);
        parts
    }
}

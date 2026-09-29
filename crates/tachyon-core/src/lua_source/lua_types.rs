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
    const LIST_SUFFIX: &'static str = "[]";
    const OPTIONAL_SUFFIX: char = '?';
    const MAP_START: &'static str = "table<";
    const FUNCTION_START: &'static str = "fun(";
    const NOTHING: &'static str = "void";

    pub(super) fn yard(lua: &str) -> String {
        let text = lua.trim();
        if text.is_empty() {
            return String::from(Self::NOTHING);
        }
        Self::split_top(text, '|')
            .iter()
            .map(|part| Self::single(part.trim()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(super) fn type_word(text: &str) -> (&str, &str) {
        let mut depth = 0_i32;
        for (at, character) in text.char_indices() {
            match character {
                '<' | '(' | '{' => depth += 1,
                '>' | ')' | '}' => depth -= 1,
                ',' if depth == 0 => return (&text[..at], text[at..].trim_start()),
                space if space.is_whitespace() && depth == 0 => {
                    return (&text[..at], text[at..].trim_start());
                }
                _ => {}
            }
        }
        (text, "")
    }

    fn single(text: &str) -> String {
        if let Some(inner) = text.strip_suffix(Self::OPTIONAL_SUFFIX) {
            return format!("{}, nil", Self::single(inner));
        }
        if let Some(item) = text.strip_suffix(Self::LIST_SUFFIX) {
            return format!("Array<{}>", Self::single(item));
        }
        if let Some(inner) = text
            .strip_prefix(Self::MAP_START)
            .and_then(|rest| rest.strip_suffix('>'))
        {
            return Self::map(inner);
        }
        if text.starts_with(Self::FUNCTION_START) {
            return String::from("Proc");
        }
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
        let mut parts = Vec::new();
        let mut depth = 0_i32;
        let mut start = 0;
        for (at, character) in text.char_indices() {
            match character {
                '<' | '(' | '{' => depth += 1,
                '>' | ')' | '}' => depth -= 1,
                found if found == separator && depth == 0 => {
                    parts.push(&text[start..at]);
                    start = at + character.len_utf8();
                }
                _ => {}
            }
        }
        parts.push(&text[start..]);
        parts
    }
}

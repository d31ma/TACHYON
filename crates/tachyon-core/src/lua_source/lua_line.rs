use super::LuaTypes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LuaLine {
    Blank,
    ClassDoc { name: String },
    FieldDoc { name: String, yard: String },
    Doc(String),
    Marker(String),
    Comment,
    ClassTable { name: String, base: Option<String>, open: bool },
    IndexSelf,
    Constant { class: String, text: String },
    Method { class: String, name: String, is_static: bool, rest: String },
    TableEnd,
    Other,
}

impl LuaLine {
    const MARKER: &'static str = "-- @";
    const DOC: &'static str = "---";
    const COMMENT: &'static str = "--";
    const METHOD: &'static str = "function ";
    const SUPER_TABLE: &'static str = "setmetatable({}, { __index = ";

    pub(super) fn read(line: &str) -> Self {
        let text = line.trim_end();
        if text.trim().is_empty() {
            return Self::Blank;
        }
        if let Some(rest) = text.strip_prefix(Self::DOC) {
            return Self::doc(rest.trim());
        }
        if let Some(rest) = text.strip_prefix(Self::MARKER)
            && rest.starts_with(|first: char| first.is_ascii_uppercase())
        {
            return Self::Marker(format!("#[{rest}]"));
        }
        if text.starts_with(Self::COMMENT) {
            return Self::Comment;
        }
        if text.starts_with('}') {
            return Self::TableEnd;
        }
        if let Some(rest) = text.strip_prefix(Self::METHOD) {
            return Self::method(rest).unwrap_or(Self::Other);
        }
        Self::assignment(text).unwrap_or(Self::Other)
    }

    fn doc(rest: &str) -> Self {
        if let Some(name) = rest.strip_prefix("@class ") {
            let name = name.split([':', ' ']).next().unwrap_or_default();
            return Self::ClassDoc { name: name.to_owned() };
        }
        if let Some(field) = rest.strip_prefix("@field ") {
            let (name, lua) = Self::word(field.trim());
            let (lua, _) = LuaTypes::type_word(lua);
            return Self::FieldDoc {
                name: name.to_owned(),
                yard: LuaTypes::yard(lua),
            };
        }
        if let Some(parameter) = rest.strip_prefix("@param ") {
            let (name, lua) = Self::word(parameter.trim());
            let (lua, _) = LuaTypes::type_word(lua);
            let name = name.trim_end_matches('?');
            return Self::Doc(format!("# @param {name} [{}]", LuaTypes::yard(lua)));
        }
        if let Some(lua) = rest
            .strip_prefix("@return ")
            .or_else(|| rest.strip_prefix("@type "))
        {
            let (lua, _) = LuaTypes::type_word(lua.trim());
            return Self::Doc(format!("# @return [{}]", LuaTypes::yard(lua)));
        }
        Self::Comment
    }

    fn word(text: &str) -> (&str, &str) {
        text.split_once(char::is_whitespace)
            .map_or((text, ""), |(word, rest)| (word, rest.trim_start()))
    }

    fn method(rest: &str) -> Option<Self> {
        let open = rest.find('(')?;
        let head = rest[..open].trim();
        let (class, name, is_static) = match (head.split_once(':'), head.split_once('.')) {
            (Some((class, name)), _) => (class, name, false),
            (None, Some((class, name))) => (class, name, true),
            _ => return None,
        };
        Self::is_name(class).then_some(())?;
        Self::is_name(name).then_some(())?;
        Some(Self::Method {
            class: class.to_owned(),
            name: name.to_owned(),
            is_static,
            rest: rest[open..].to_owned(),
        })
    }

    fn assignment(text: &str) -> Option<Self> {
        let (target, value) = text.split_once('=')?;
        let target = target.trim();
        let value = value.trim();
        if let Some((class, member)) = target.split_once('.') {
            return Self::member(class, member, value);
        }
        Self::is_name(target).then_some(())?;
        let name = target.to_owned();
        if value == "{}" {
            return Some(Self::ClassTable { name, base: None, open: false });
        }
        if value == "{" {
            return Some(Self::ClassTable { name, base: None, open: true });
        }
        let base = value.strip_prefix(Self::SUPER_TABLE)?.strip_suffix(" })")?;
        Self::is_name(base).then_some(())?;
        Some(Self::ClassTable {
            name,
            base: Some(base.to_owned()),
            open: false,
        })
    }

    fn member(class: &str, member: &str, value: &str) -> Option<Self> {
        Self::is_name(class).then_some(())?;
        if member == "__index" && value == class {
            return Some(Self::IndexSelf);
        }
        let constant = member
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
        (constant && !member.is_empty()).then(|| Self::Constant {
            class: class.to_owned(),
            text: format!("{member} = {value}"),
        })
    }

    pub(super) fn is_name(text: &str) -> bool {
        text.starts_with(|first: char| first.is_ascii_alphabetic() || first == '_')
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    }
}

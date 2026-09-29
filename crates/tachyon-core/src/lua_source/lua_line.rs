use super::LuaTypes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LuaLine {
    Blank,
    ClassDoc,
    FieldDoc {
        name: String,
        yard: String,
    },
    Doc(String),
    Marker(String),
    Comment,
    ClassTable {
        name: String,
        base: Option<String>,
        open: bool,
    },
    IndexSelf,
    Constant {
        class: String,
        text: String,
    },
    Method {
        class: String,
        name: String,
        is_static: bool,
        rest: String,
    },
    TableEnd,
    Other,
}

impl LuaLine {
    const MARKER: &'static str = "-- @";
    const DOC: &'static str = "---";
    const COMMENT: &'static str = "--";
    const METHOD: &'static str = "function ";
    const SUPER_TABLE: &'static str = "setmetatable({}, { __index = ";
    const SUPER_TABLE_END: &'static str = " })";
    const INDEX: &'static str = "__index";
    const EMPTY_TABLE: &'static str = "{}";
    const OPEN_TABLE: &'static str = "{";

    pub(super) fn read(line: &str) -> Self {
        let text = line.trim_end();
        Self::blank(text)
            .or_else(|| Self::doc_line(text))
            .or_else(|| Self::marker(text))
            .or_else(|| Self::comment(text))
            .or_else(|| Self::table_end(text))
            .or_else(|| Self::method_line(text))
            .or_else(|| Self::assignment(text))
            .unwrap_or(Self::Other)
    }

    pub(super) fn is_name(text: &str) -> bool {
        let first = text.chars().next().unwrap_or('0');
        let starts = [first.is_ascii_alphabetic(), first == '_'].contains(&true);
        [starts, Self::name_bytes(text)] == [true, true]
    }

    fn name_bytes(text: &str) -> bool {
        text.bytes()
            .all(|byte| [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true))
    }

    fn blank(text: &str) -> Option<Self> {
        text.trim().is_empty().then_some(Self::Blank)
    }

    fn doc_line(text: &str) -> Option<Self> {
        text.strip_prefix(Self::DOC)
            .map(|rest| Self::doc(rest.trim()))
    }

    fn marker(text: &str) -> Option<Self> {
        text.strip_prefix(Self::MARKER).and_then(|rest| {
            rest.starts_with(|first: char| first.is_ascii_uppercase())
                .then(|| Self::Marker(format!("#[{rest}]")))
        })
    }

    fn comment(text: &str) -> Option<Self> {
        text.starts_with(Self::COMMENT).then_some(Self::Comment)
    }

    fn table_end(text: &str) -> Option<Self> {
        text.starts_with('}').then_some(Self::TableEnd)
    }

    fn method_line(text: &str) -> Option<Self> {
        text.strip_prefix(Self::METHOD).and_then(Self::method)
    }

    fn doc(rest: &str) -> Self {
        Self::class_doc(rest)
            .or_else(|| Self::field_doc(rest))
            .or_else(|| Self::parameter_doc(rest))
            .or_else(|| Self::return_doc(rest))
            .unwrap_or(Self::Comment)
    }

    fn class_doc(rest: &str) -> Option<Self> {
        rest.starts_with("@class ").then_some(Self::ClassDoc)
    }

    fn field_doc(rest: &str) -> Option<Self> {
        rest.strip_prefix("@field ").map(Self::field)
    }

    fn parameter_doc(rest: &str) -> Option<Self> {
        rest.strip_prefix("@param ").map(Self::parameter)
    }

    fn return_doc(rest: &str) -> Option<Self> {
        rest.strip_prefix("@return ")
            .or_else(|| rest.strip_prefix("@type "))
            .map(Self::returned)
    }

    fn field(text: &str) -> Self {
        let (name, lua) = Self::word(text.trim());
        let (lua, _) = LuaTypes::type_word(lua);
        Self::FieldDoc {
            name: name.to_owned(),
            yard: LuaTypes::yard(lua),
        }
    }

    fn parameter(text: &str) -> Self {
        let (name, lua) = Self::word(text.trim());
        let (lua, _) = LuaTypes::type_word(lua);
        let name = name.trim_end_matches('?');
        Self::Doc(format!("# @param {name} [{}]", LuaTypes::yard(lua)))
    }

    fn returned(text: &str) -> Self {
        let (lua, _) = LuaTypes::type_word(text.trim());
        Self::Doc(format!("# @return [{}]", LuaTypes::yard(lua)))
    }

    fn word(text: &str) -> (&str, &str) {
        text.split_once(char::is_whitespace)
            .map_or((text, ""), |(word, rest)| (word, rest.trim_start()))
    }

    fn method(rest: &str) -> Option<Self> {
        let (head, after) = rest.split_once('(')?;
        let (class, name, is_static) = Self::receiver(head.trim())?;
        let named = [Self::is_name(class), Self::is_name(name)] == [true, true];
        named.then(|| Self::Method {
            class: class.to_owned(),
            name: name.to_owned(),
            is_static,
            rest: format!("({after}"),
        })
    }

    fn receiver(head: &str) -> Option<(&str, &str, bool)> {
        Self::instance_receiver(head).or_else(|| Self::static_receiver(head))
    }

    fn instance_receiver(head: &str) -> Option<(&str, &str, bool)> {
        head.split_once(':')
            .map(|(class, name)| (class, name, false))
    }

    fn static_receiver(head: &str) -> Option<(&str, &str, bool)> {
        head.split_once('.')
            .map(|(class, name)| (class, name, true))
    }

    fn assignment(text: &str) -> Option<Self> {
        let (target, value) = text.split_once('=')?;
        let (target, value) = (target.trim(), value.trim());
        target.split_once('.').map_or_else(
            || Self::class_table(target, value),
            |(class, member)| Self::member(class, member, value),
        )
    }

    fn class_table(name: &str, value: &str) -> Option<Self> {
        Self::is_name(name)
            .then(|| Self::table_value(name, value))
            .flatten()
    }

    fn table_value(name: &str, value: &str) -> Option<Self> {
        Self::plain_table(name, value).or_else(|| Self::derived_table(name, value))
    }

    fn plain_table(name: &str, value: &str) -> Option<Self> {
        let open = value == Self::OPEN_TABLE;
        [value == Self::EMPTY_TABLE, open]
            .contains(&true)
            .then(|| Self::ClassTable {
                name: name.to_owned(),
                base: None,
                open,
            })
    }

    fn derived_table(name: &str, value: &str) -> Option<Self> {
        value
            .strip_prefix(Self::SUPER_TABLE)
            .and_then(|rest| rest.strip_suffix(Self::SUPER_TABLE_END))
            .and_then(|base| {
                Self::is_name(base).then(|| Self::ClassTable {
                    name: name.to_owned(),
                    base: Some(base.to_owned()),
                    open: false,
                })
            })
    }

    fn member(class: &str, member: &str, value: &str) -> Option<Self> {
        Self::is_name(class)
            .then(|| {
                Self::index_self(class, member, value)
                    .or_else(|| Self::constant(class, member, value))
            })
            .flatten()
    }

    fn index_self(class: &str, member: &str, value: &str) -> Option<Self> {
        ([member == Self::INDEX, value == class] == [true, true]).then_some(Self::IndexSelf)
    }

    fn constant(class: &str, member: &str, value: &str) -> Option<Self> {
        Self::is_constant(member).then(|| Self::Constant {
            class: class.to_owned(),
            text: format!("{member} = {value}"),
        })
    }

    fn is_constant(member: &str) -> bool {
        let capitals = member.bytes().all(|byte| {
            [
                byte.is_ascii_uppercase(),
                byte.is_ascii_digit(),
                byte == b'_',
            ]
            .contains(&true)
        });
        [!member.is_empty(), capitals] == [true, true]
    }
}

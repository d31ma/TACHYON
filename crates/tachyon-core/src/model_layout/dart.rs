use super::piece::Piece;
use crate::ascii_name::AsciiName;

struct DartModel;

impl DartModel {
    const CLASS_MODIFIERS: [&'static str; 5] = ["abstract", "base", "final", "sealed", "interface"];
    const MEMBER_MODIFIERS: [&'static str; 5] = ["static", "late", "final", "const", "covariant"];
    const ACCESSOR_WORDS: [&'static str; 3] = ["get", "set", "operator"];
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(index) = code.text.find('@') {
        return Some(code.at + index);
    }
    for item in code.items() {
        if let Some(at) = refused_item(item) {
            return Some(at);
        }
    }
    None
}

fn refused_item(item: Piece<'_>) -> Option<usize> {
    if matches!(item.first_word(), "import" | "typedef") && item.ends_with(";") {
        return None;
    }
    if let Some(rest) = item.after_word("enum") {
        return refused_enum(rest);
    }
    let declaration = item.after_words(&DartModel::CLASS_MODIFIERS);
    if let Some(rest) = declaration.after_word("class") {
        return refused_class(rest);
    }
    Some(item.at)
}

fn refused_class(declaration: Piece<'_>) -> Option<usize> {
    let name = declaration.first_word();
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.rest.text.is_empty() {
        return Some(body.rest.at);
    }
    refused_members(body.inner, name)
}

fn refused_enum(declaration: Piece<'_>) -> Option<usize> {
    let name = declaration.first_word();
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.rest.text.is_empty() {
        return Some(body.rest.at);
    }
    let (entries, members) = body.inner.split_first_top(b';');
    for entry in entries.split_top(b',') {
        if !entry.text.is_empty() && entry.first_word().is_empty() {
            return Some(entry.at);
        }
    }
    refused_members(members, name)
}

fn refused_members(members: Piece<'_>, type_name: &str) -> Option<usize> {
    for member in members.items() {
        if let Some(at) = refused_member(member, type_name) {
            return Some(at);
        }
    }
    None
}

fn refused_member(member: Piece<'_>, type_name: &str) -> Option<usize> {
    let declaration = member.after_words(&DartModel::MEMBER_MODIFIERS);
    if let Some(rest) = declaration.after_word(type_name)
        && (rest.starts_with("(") || rest.starts_with("."))
    {
        return refused_constructor(member, rest);
    }
    if is_field(declaration) {
        return None;
    }
    Some(member.at)
}

fn refused_constructor(member: Piece<'_>, after_name: Piece<'_>) -> Option<usize> {
    let mut parameters = after_name;
    if let Some(named) = after_name.after(".") {
        parameters = named.from(named.first_word().len()).trim();
    }
    if !parameters.starts_with("(") {
        return Some(member.at);
    }
    let Some(rest) = parameters.after_group() else {
        return Some(member.at);
    };
    let has_initializers = rest.starts_with(":") && rest.ends_with(";");
    if rest.text == ";" || has_initializers {
        return None;
    }
    Some(member.at)
}

fn is_field(declaration: Piece<'_>) -> bool {
    for word in DartModel::ACCESSOR_WORDS {
        if declaration.has_top_word(word) {
            return false;
        }
    }
    if !declaration.ends_with(";") {
        return false;
    }
    let typed = declaration.after_arguments();
    let Some(index) = first_call_or_value(typed) else {
        return true;
    };
    let value = typed.from(index);
    value.starts_with("=") && !value.starts_with("=>")
}

fn first_call_or_value(declaration: Piece<'_>) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, byte) in declaration.text.bytes().enumerate() {
        let is_target = matches!(byte, b'=' | b'{') || is_call(declaration.text, index);
        if depth == 0 && is_target {
            return Some(index);
        }
        depth = Piece::depth_after(byte, depth);
    }
    None
}

fn is_call(text: &str, index: usize) -> bool {
    if text.as_bytes()[index] != b'(' {
        return false;
    }
    let before = text[..index].trim_end();
    let Some(before_word) = before.strip_suffix("Function") else {
        return true;
    };
    before_word
        .bytes()
        .last()
        .is_some_and(AsciiName::is_name_byte)
}

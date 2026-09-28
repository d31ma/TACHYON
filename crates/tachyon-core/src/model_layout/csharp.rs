use super::piece::Piece;

struct CSharpModel;

impl CSharpModel {
    const TYPE_MODIFIERS: [&'static str; 9] = [
        "public",
        "internal",
        "sealed",
        "partial",
        "readonly",
        "abstract",
        "private",
        "protected",
        "file",
    ];
    const MEMBER_MODIFIERS: [&'static str; 10] = [
        "public",
        "private",
        "protected",
        "internal",
        "readonly",
        "static",
        "required",
        "const",
        "new",
        "volatile",
    ];
    const ACCESS_MODIFIERS: [&'static str; 4] = ["public", "private", "protected", "internal"];
    const ACCESSORS: [&'static str; 3] = ["get", "set", "init"];
    const REFUSED_MEMBER_WORDS: [&'static str; 10] = [
        "class",
        "struct",
        "record",
        "enum",
        "interface",
        "delegate",
        "event",
        "operator",
        "implicit",
        "explicit",
    ];
    const ATTRIBUTE_LIST_FOLLOWS: &'static [u8] = b"(,;{}]";
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    let without_directives = blank_directive_lines(code.text);
    let code = Piece::new(code.at, &without_directives);
    if let Some(at) = first_framework_attribute(code) {
        return Some(at);
    }
    for item in code.items() {
        if let Some(at) = refused_item(item) {
            return Some(at);
        }
    }
    None
}

fn blank_directive_lines(text: &str) -> String {
    let mut lines = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if !line.trim_start().starts_with('#') {
            lines.push_str(line);
            continue;
        }
        let content = line.trim_end_matches('\n');
        lines.push_str(&" ".repeat(content.len()));
        lines.push_str(&line[content.len()..]);
    }
    lines
}

fn first_framework_attribute(code: Piece<'_>) -> Option<usize> {
    for (index, _) in code.text.match_indices('[') {
        if !starts_attribute_list(code.text, index) {
            continue;
        }
        let list = code.from(index);
        let Some(close) = list.closing(0) else {
            continue;
        };
        for attribute in list.slice(1, close).split_top(b',') {
            if crate::behavior::is_framework_marker(attribute_name(attribute)) {
                return Some(attribute.at);
            }
        }
    }
    None
}

fn starts_attribute_list(text: &str, index: usize) -> bool {
    let Some(previous) = text[..index].trim_end().bytes().last() else {
        return true;
    };
    CSharpModel::ATTRIBUTE_LIST_FOLLOWS.contains(&previous)
}

fn attribute_name(attribute: Piece<'_>) -> &str {
    let mut name = attribute;
    let target = attribute.first_word();
    if let Some(rest) = attribute.from(target.len()).trim().after(":") {
        name = rest;
    }
    let name = Piece::last_segment(name.path());
    name.strip_suffix("Attribute").unwrap_or(name)
}

fn refused_item(item: Piece<'_>) -> Option<usize> {
    if let Some(rest) = item.after_word("using") {
        return refused_using(item, rest);
    }
    let declaration = after_attribute_lists(item).after_words(&CSharpModel::TYPE_MODIFIERS);
    if declaration.first_word() == "enum" {
        return None;
    }
    let Some(rest) = after_type_kind(declaration) else {
        return Some(declaration.at);
    };
    refused_type(rest)
}

fn refused_using(item: Piece<'_>, rest: Piece<'_>) -> Option<usize> {
    let is_statement = rest.starts_with("(") || rest.first_word() == "var";
    if is_statement || !item.ends_with(";") {
        return Some(item.at);
    }
    None
}

fn after_attribute_lists(item: Piece<'_>) -> Piece<'_> {
    let mut piece = item;
    while piece.starts_with("[") {
        let Some(rest) = piece.after_group() else {
            return piece;
        };
        piece = rest;
    }
    piece
}

fn after_type_kind(declaration: Piece<'_>) -> Option<Piece<'_>> {
    if let Some(rest) = declaration.after_word("record") {
        return Some(rest.after_words(&["struct", "class"]));
    }
    if let Some(rest) = declaration.after_word("class") {
        return Some(rest);
    }
    declaration.after_word("struct")
}

fn refused_type(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        if declaration.ends_with(";") {
            return None;
        }
        return Some(declaration.at);
    };
    if !body.has_plain_end() {
        return Some(body.rest.at);
    }
    for member in body.inner.items() {
        if let Some(at) = refused_member(member) {
            return Some(at);
        }
    }
    None
}

fn refused_member(member: Piece<'_>) -> Option<usize> {
    let declaration = after_attribute_lists(member).after_words(&CSharpModel::MEMBER_MODIFIERS);
    if is_refused_member_kind(declaration) {
        return Some(declaration.at);
    }
    let typed = declaration.after_arguments();
    let index = typed.find_top(b"(={")?;
    let from_index = typed.from(index);
    if from_index.starts_with("=>") || from_index.starts_with("(") {
        return Some(declaration.at);
    }
    if from_index.starts_with("{") {
        return refused_property(from_index);
    }
    None
}

fn is_refused_member_kind(declaration: Piece<'_>) -> bool {
    if declaration.starts_with("~") || declaration.has_top_word("this") {
        return true;
    }
    for word in CSharpModel::REFUSED_MEMBER_WORDS {
        if declaration.has_top_word(word) {
            return true;
        }
    }
    false
}

fn refused_property(accessors: Piece<'_>) -> Option<usize> {
    let Some(close) = accessors.closing(0) else {
        return Some(accessors.at);
    };
    let rest = accessors.from(close + 1).trim();
    let has_initializer = rest.starts_with("=") && rest.ends_with(";");
    if !rest.text.is_empty() && !has_initializer {
        return Some(rest.at);
    }
    let mut parts = accessors.slice(1, close).split_top(b';');
    let last = parts.pop()?;
    if !last.text.is_empty() {
        return Some(last.at);
    }
    for part in parts {
        let accessor = part.after_words(&CSharpModel::ACCESS_MODIFIERS);
        if !CSharpModel::ACCESSORS.contains(&accessor.text) {
            return Some(part.at);
        }
    }
    None
}

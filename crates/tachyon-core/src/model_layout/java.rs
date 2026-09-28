use super::piece::Piece;

struct JavaModel;

impl JavaModel {
    const TYPE_MODIFIERS: [&'static str; 3] = ["public", "final", "abstract"];
    const FIELD_MODIFIERS: [&'static str; 7] = [
        "public",
        "private",
        "protected",
        "static",
        "final",
        "transient",
        "volatile",
    ];
    const NESTED_KINDS: [&'static str; 4] = ["class", "interface", "enum", "record"];
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(at) = code.first_framework_annotation() {
        return Some(at);
    }
    for item in code.items() {
        if let Some(at) = refused_item(item) {
            return Some(at);
        }
    }
    None
}

fn refused_item(item: Piece<'_>) -> Option<usize> {
    if item.first_word() == "import" && item.ends_with(";") {
        return None;
    }
    let declaration = item
        .after_annotations()
        .after_words(&JavaModel::TYPE_MODIFIERS);
    if let Some(rest) = declaration.after_word("record") {
        return refused_record(rest);
    }
    if let Some(rest) = declaration.after_word("enum") {
        return refused_enum(rest);
    }
    if let Some(rest) = declaration.after_word("class") {
        return refused_class(rest);
    }
    Some(declaration.at)
}

pub(super) fn refused_record(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.inner.text.is_empty() {
        return Some(body.inner.at);
    }
    if !body.has_plain_end() {
        return Some(body.rest.at);
    }
    None
}

pub(super) fn refused_enum(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.has_plain_end() {
        return Some(body.rest.at);
    }
    let (constants, members) = body.inner.split_first_top(b';');
    if !members.text.is_empty() {
        return Some(members.at);
    }
    for constant in constants.split_top(b',') {
        let name = constant.after_annotations();
        if name.text != name.first_word() {
            return Some(constant.at);
        }
    }
    None
}

fn refused_class(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.has_plain_end() {
        return Some(body.rest.at);
    }
    for member in body.inner.items() {
        if let Some(at) = refused_field(member) {
            return Some(at);
        }
    }
    None
}

pub(super) fn refused_field(member: Piece<'_>) -> Option<usize> {
    let declaration = member
        .after_annotations()
        .after_words(&JavaModel::FIELD_MODIFIERS);
    if JavaModel::NESTED_KINDS.contains(&declaration.first_word()) {
        return Some(declaration.at);
    }
    let index = declaration.find_top(b"({=")?;
    if declaration.from(index).starts_with("=") {
        return None;
    }
    Some(declaration.at)
}

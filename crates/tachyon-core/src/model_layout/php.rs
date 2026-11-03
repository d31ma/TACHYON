use super::piece::Piece;

struct PhpModel;

impl PhpModel {
    const CLASS_MODIFIERS: [&'static str; 3] = ["final", "readonly", "abstract"];
    const PROPERTY_MODIFIERS: [&'static str; 5] =
        ["public", "protected", "private", "static", "readonly"];
    const VISIBILITY: [&'static str; 3] = ["public", "protected", "private"];
    const REFUSED_TEXT: [&'static str; 3] = ["<?", "?>", "#["];
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    let code = code.trim();
    let code = code.after("<?php").unwrap_or(code);
    if let Some(at) = first_refused_text(code) {
        return Some(at);
    }
    for item in code.items() {
        if let Some(at) = refused_item(item) {
            return Some(at);
        }
    }
    None
}

fn first_refused_text(code: Piece<'_>) -> Option<usize> {
    let mut first = None;
    for text in PhpModel::REFUSED_TEXT {
        let Some(index) = code.text.find(text) else {
            continue;
        };
        if first.is_none_or(|earlier| index < earlier) {
            first = Some(index);
        }
    }
    let index = first?;
    Some(code.at + index)
}

fn refused_item(item: Piece<'_>) -> Option<usize> {
    if let Some(rest) = item.after_word("use") {
        return refused_use(item, rest);
    }
    if let Some(rest) = item.after_word("enum") {
        return refused_enum(rest);
    }
    let declaration = item.after_words(&PhpModel::CLASS_MODIFIERS);
    if let Some(rest) = declaration.after_word("class") {
        return refused_class(rest);
    }
    Some(item.at)
}

fn refused_use(item: Piece<'_>, rest: Piece<'_>) -> Option<usize> {
    let imports_code = matches!(rest.first_word(), "function" | "const");
    if imports_code || !item.ends_with(";") {
        return Some(item.at);
    }
    None
}

fn refused_enum(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if let Some(index) = body.head.find_top_word("implements") {
        return Some(body.head.at + index);
    }
    if !body.rest.text.is_empty() {
        return Some(body.rest.at);
    }
    for member in body.inner.items() {
        if member.first_word() != "case" || !member.ends_with(";") {
            return Some(member.at);
        }
    }
    None
}

fn refused_class(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.rest.text.is_empty() {
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
    let declaration = member.after_words(&PhpModel::PROPERTY_MODIFIERS);
    if let Some(rest) = declaration.after_word("function") {
        return refused_constructor(member, rest);
    }
    let has_modifier = declaration.at != member.at;
    if !has_modifier || !is_typed_property(declaration) {
        return Some(member.at);
    }
    None
}

fn is_typed_property(declaration: Piece<'_>) -> bool {
    let Some(dollar) = declaration.text.find('$') else {
        return false;
    };
    let property_type = declaration.slice(0, dollar).trim();
    if property_type.text.is_empty() || property_type.text.contains(['=', '{', ';']) {
        return false;
    }
    let name = declaration.from(dollar + 1);
    let rest = name.from(name.first_word().len()).trim();
    let has_default = rest.starts_with("=") && rest.ends_with(";");
    rest.text == ";" || has_default
}

fn refused_constructor(member: Piece<'_>, rest: Piece<'_>) -> Option<usize> {
    let Some(parameters) = rest.after_word("__construct") else {
        return Some(member.at);
    };
    if !parameters.starts_with("(") {
        return Some(member.at);
    }
    let Some(close) = parameters.closing(0) else {
        return Some(member.at);
    };
    for parameter in parameters.slice(1, close).split_top(b',') {
        let promoted = PhpModel::VISIBILITY.contains(&parameter.first_word());
        if !parameter.text.is_empty() && !promoted {
            return Some(parameter.at);
        }
    }
    let body = parameters.from(close + 1).trim();
    if body.after("{").is_some_and(|inside| inside.text == "}") {
        return None;
    }
    Some(body.at)
}

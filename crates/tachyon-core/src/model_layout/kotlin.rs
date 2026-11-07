use super::piece::Piece;

struct KotlinModel;

impl KotlinModel {
    const TYPE_MODIFIERS: [&'static str; 7] = [
        "public", "internal", "private", "data", "sealed", "abstract", "open",
    ];
    const PARAMETER_MODIFIERS: [&'static str; 4] = ["public", "internal", "private", "protected"];
    const PROPERTY_MODIFIERS: [&'static str; 5] =
        ["public", "internal", "private", "protected", "lateinit"];
    const PROPERTY_WORDS: [&'static str; 2] = ["val", "var"];
    const ACCESSOR_WORDS: [&'static str; 3] = ["by", "get", "set"];
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(at) = code.first_framework_annotation() {
        return Some(at);
    }
    for statement in code.statements() {
        if let Some(at) = refused_statement(statement) {
            return Some(at);
        }
    }
    None
}

fn refused_statement(statement: Piece<'_>) -> Option<usize> {
    let declaration = statement.after_annotations();
    if declaration.text.is_empty() || declaration.first_word() == "import" {
        return None;
    }
    let declaration = declaration.after_words(&KotlinModel::TYPE_MODIFIERS);
    if declaration.first_word() == "typealias" {
        return None;
    }
    if let Some(rest) = declaration.after_word("enum") {
        let Some(class) = rest.after_word("class") else {
            return Some(declaration.at);
        };
        return refused_class(class, true);
    }
    if let Some(rest) = declaration.after_word("class") {
        return refused_class(rest, false);
    }
    Some(declaration.at)
}

fn refused_class(declaration: Piece<'_>, is_enum: bool) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return refused_header(declaration);
    };
    if !body.rest.text.is_empty() {
        return Some(body.rest.at);
    }
    if let Some(at) = refused_header(body.head) {
        return Some(at);
    }
    if is_enum {
        return refused_entries(body.inner);
    }
    refused_properties(body.inner)
}

fn refused_header(header: Piece<'_>) -> Option<usize> {
    let name = header.first_word();
    let Some(mut rest) = header.from(name.len()).trim().after_generics() else {
        return Some(header.at);
    };
    if rest.starts_with("(") {
        let Some(close) = rest.closing(0) else {
            return Some(rest.at);
        };
        for parameter in rest.slice(1, close).split_top(b',') {
            if let Some(at) = refused_parameter(parameter) {
                return Some(at);
            }
        }
        rest = rest.from(close + 1).trim();
    }
    if rest.text.is_empty() || rest.starts_with(":") || rest.first_word() == "where" {
        return None;
    }
    Some(rest.at)
}

fn refused_parameter(parameter: Piece<'_>) -> Option<usize> {
    if parameter.text.is_empty() {
        return None;
    }
    let declaration = parameter
        .after_annotations()
        .after_words(&KotlinModel::PARAMETER_MODIFIERS);
    if KotlinModel::PROPERTY_WORDS.contains(&declaration.first_word()) {
        return None;
    }
    Some(parameter.at)
}

fn refused_properties(inner: Piece<'_>) -> Option<usize> {
    for member in inner.statements() {
        let declaration = member
            .after_annotations()
            .after_words(&KotlinModel::PROPERTY_MODIFIERS);
        if declaration.text.is_empty() {
            continue;
        }
        if !KotlinModel::PROPERTY_WORDS.contains(&declaration.first_word()) {
            return Some(member.at);
        }
        for word in KotlinModel::ACCESSOR_WORDS {
            if declaration.has_top_word(word) {
                return Some(member.at);
            }
        }
    }
    None
}

fn refused_entries(inner: Piece<'_>) -> Option<usize> {
    let (entries, members) = inner.split_first_top(b';');
    if !members.text.is_empty() {
        return Some(members.at);
    }
    for entry in entries.split_top(b',') {
        let declaration = entry.after_annotations();
        if declaration.text.is_empty() {
            continue;
        }
        let name = declaration.first_word();
        let arguments = declaration.from(name.len()).trim();
        if name.is_empty() || !arguments.is_empty_or_group() {
            return Some(entry.at);
        }
    }
    None
}

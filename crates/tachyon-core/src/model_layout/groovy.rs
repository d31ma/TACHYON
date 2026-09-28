use super::java::{refused_enum, refused_field, refused_record};
use super::piece::Piece;

struct GroovyModel;

impl GroovyModel {
    const TYPE_MODIFIERS: [&'static str; 3] = ["public", "final", "abstract"];
    const IMPORT: &'static str = "import";
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(at) = code.first_framework_annotation() {
        return Some(at);
    }
    for item in code.statements() {
        if let Some(at) = refused_item(item) {
            return Some(at);
        }
    }
    None
}

fn refused_item(item: Piece<'_>) -> Option<usize> {
    if item.first_word() == GroovyModel::IMPORT {
        return None;
    }
    let declaration = item
        .after_annotations()
        .after_words(&GroovyModel::TYPE_MODIFIERS);
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

fn refused_class(declaration: Piece<'_>) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.has_plain_end() {
        return Some(body.rest.at);
    }
    for member in body.inner.statements() {
        if let Some(at) = refused_field(member) {
            return Some(at);
        }
    }
    None
}

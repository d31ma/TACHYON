use super::piece::Piece;

struct SwiftModel;

impl SwiftModel {
    const MODIFIERS: [&'static str; 7] = [
        "public",
        "internal",
        "fileprivate",
        "private",
        "package",
        "static",
        "indirect",
    ];
    const SYNTHESIZED: [&'static str; 9] = [
        "Codable",
        "Encodable",
        "Decodable",
        "Equatable",
        "Hashable",
        "Comparable",
        "Sendable",
        "CaseIterable",
        "Identifiable",
    ];
    const RAW_TYPES: [&'static str; 14] = [
        "String",
        "Character",
        "Int",
        "Int8",
        "Int16",
        "Int32",
        "Int64",
        "UInt",
        "UInt8",
        "UInt16",
        "UInt32",
        "UInt64",
        "Double",
        "Float",
    ];
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(index) = code.text.find('@') {
        return Some(code.at + index);
    }
    for statement in code.statements() {
        if let Some(at) = refused_statement(statement) {
            return Some(at);
        }
    }
    None
}

fn after_modifiers(piece: Piece<'_>) -> Piece<'_> {
    let mut rest = piece;
    while SwiftModel::MODIFIERS.contains(&rest.first_word()) {
        rest = rest.from(rest.first_word().len()).trim().after_arguments();
    }
    rest
}

fn refused_statement(statement: Piece<'_>) -> Option<usize> {
    let declaration = after_modifiers(statement);
    if matches!(declaration.first_word(), "import" | "typealias") {
        return None;
    }
    if let Some(rest) = declaration.after_word("struct") {
        return refused_type(rest, false);
    }
    if let Some(rest) = declaration.after_word("enum") {
        return refused_type(rest, true);
    }
    Some(declaration.at)
}

fn refused_type(declaration: Piece<'_>, is_enum: bool) -> Option<usize> {
    let Some(body) = declaration.body() else {
        return Some(declaration.at);
    };
    if !body.rest.text.is_empty() {
        return Some(body.rest.at);
    }
    if let Some(at) = refused_conformances(body.head, is_enum) {
        return Some(at);
    }
    for member in body.inner.statements() {
        if let Some(at) = refused_member(member, is_enum) {
            return Some(at);
        }
    }
    None
}

fn refused_conformances(head: Piece<'_>, is_enum: bool) -> Option<usize> {
    let name = head.first_word();
    let Some(rest) = head.from(name.len()).trim().after_generics() else {
        return Some(head.at);
    };
    let Some(list) = rest.after(":") else {
        return refused_unless_where(rest);
    };
    let mut list = list;
    if let Some(end) = list.find_top_word("where") {
        list = list.slice(0, end).trim();
    }
    for group in list.split_top(b',') {
        for conformance in group.split_top(b'&') {
            if !is_synthesized(conformance.text, is_enum) {
                return Some(conformance.at);
            }
        }
    }
    None
}

fn refused_unless_where(rest: Piece<'_>) -> Option<usize> {
    if rest.text.is_empty() || rest.first_word() == "where" {
        return None;
    }
    Some(rest.at)
}

fn is_synthesized(conformance: &str, is_enum: bool) -> bool {
    if SwiftModel::SYNTHESIZED.contains(&conformance) {
        return true;
    }
    is_enum && SwiftModel::RAW_TYPES.contains(&conformance)
}

fn refused_member(member: Piece<'_>, is_enum: bool) -> Option<usize> {
    if member.text.contains('{') {
        return Some(member.at);
    }
    let declaration = after_modifiers(member);
    let word = declaration.first_word();
    if is_enum && word == "case" {
        return None;
    }
    if !is_enum && matches!(word, "let" | "var") {
        return None;
    }
    Some(member.at)
}

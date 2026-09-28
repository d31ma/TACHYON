use super::piece::Piece;

struct RustModel;

impl RustModel {
    const KINDS: [&'static str; 4] = ["struct", "enum", "type", "use"];
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(at) = first_framework_attribute(code) {
        return Some(at);
    }
    for item in code.items() {
        let declaration = after_visibility(after_attributes(item));
        if !RustModel::KINDS.contains(&declaration.first_word()) {
            return Some(declaration.at);
        }
    }
    None
}

fn first_framework_attribute(code: Piece<'_>) -> Option<usize> {
    for (index, _) in code.text.match_indices("#[") {
        let name = code.from(index + 2).path();
        if crate::behavior::is_framework_marker(name) {
            return Some(code.at + index);
        }
    }
    None
}

fn after_attributes(item: Piece<'_>) -> Piece<'_> {
    let mut piece = item;
    while piece.starts_with("#[") {
        let Some(rest) = piece.from(1).after_group() else {
            return piece;
        };
        piece = rest;
    }
    piece
}

fn after_visibility(item: Piece<'_>) -> Piece<'_> {
    let Some(rest) = item.after_word("pub") else {
        return item;
    };
    rest.after_arguments()
}

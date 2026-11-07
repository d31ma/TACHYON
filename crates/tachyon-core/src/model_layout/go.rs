use super::piece::Piece;

pub(super) fn other_package(code: Piece<'_>) -> Option<usize> {
    let first = *code.statements().first()?;
    let name = first.after_word("package")?;
    if name.text == "main" {
        return None;
    }
    Some(first.at)
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(index) = code.text.find('@') {
        return Some(code.at + index);
    }
    let mut has_type = false;
    for (index, statement) in code.statements().into_iter().enumerate() {
        if index == 0 && statement.first_word() == "package" {
            continue;
        }
        if let Some(rest) = statement.after_word("import") {
            if let Some(at) = blank_import(rest) {
                return Some(at);
            }
            continue;
        }
        let Some(rest) = statement.after_word("type") else {
            return Some(statement.at);
        };
        if has_type {
            return Some(statement.at);
        }
        has_type = true;
        if let Some(at) = refused_type(statement, rest) {
            return Some(at);
        }
    }
    None
}

fn blank_import(imports: Piece<'_>) -> Option<usize> {
    let Some(group) = imports.after("(") else {
        return blank_import_line(imports);
    };
    for line in group.split_top(b'\n') {
        for import in line.split_top(b';') {
            if let Some(at) = blank_import_line(import) {
                return Some(at);
            }
        }
    }
    None
}

fn blank_import_line(line: Piece<'_>) -> Option<usize> {
    if line.first_word() == "_" {
        return Some(line.at);
    }
    None
}

fn refused_type(statement: Piece<'_>, declaration: Piece<'_>) -> Option<usize> {
    if declaration.starts_with("(") {
        return Some(statement.at);
    }
    let name = declaration.first_word();
    let definition = declaration.from(name.len());
    let at = definition.find_top_word("interface")?;
    Some(definition.at + at)
}

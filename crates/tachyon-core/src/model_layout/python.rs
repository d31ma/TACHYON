use super::piece::Piece;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClassKind {
    Fields,
    Members,
}

struct Statement<'a> {
    indent: usize,
    piece: Piece<'a>,
}

#[derive(Default)]
struct PythonModel {
    dataclass: bool,
    unique: bool,
    class: Option<ClassKind>,
    body_indent: Option<usize>,
}

impl PythonModel {
    const DATACLASS_NAMES: [&'static str; 2] = ["dataclass", "dataclasses.dataclass"];
    const UNIQUE_NAMES: [&'static str; 2] = ["unique", "enum.unique"];
    const FIELD_BASES: [&'static str; 2] = ["TypedDict", "NamedTuple"];
    const ENUM_BASES: [&'static str; 5] = ["Enum", "IntEnum", "StrEnum", "Flag", "IntFlag"];
    const KEYWORDS_BEFORE_COLON: [&'static str; 5] = ["else", "try", "finally", "except", "lambda"];

    fn read(&mut self, statement: &Statement<'_>) -> Option<usize> {
        if statement.indent > 0 {
            return self.read_member(statement);
        }
        self.class = None;
        let piece = statement.piece;
        if piece.starts_with("@") {
            return self.read_decorator(piece);
        }
        if matches!(piece.first_word(), "import" | "from") {
            return None;
        }
        if let Some(rest) = piece.after_word("class") {
            return self.read_class(piece, rest);
        }
        Some(piece.at)
    }

    fn read_decorator(&mut self, piece: Piece<'_>) -> Option<usize> {
        let rest = piece.from(1).trim();
        let name = rest.path();
        let arguments = rest.from(name.len()).trim();
        if Self::DATACLASS_NAMES.contains(&name) && arguments.is_empty_or_group() {
            self.dataclass = true;
            return None;
        }
        if Self::UNIQUE_NAMES.contains(&name) && arguments.text.is_empty() {
            self.unique = true;
            return None;
        }
        Some(piece.at)
    }

    fn read_class(&mut self, piece: Piece<'_>, rest: Piece<'_>) -> Option<usize> {
        let Some(colon) = rest.find_top(b":") else {
            return Some(piece.at);
        };
        let kind = self.class_kind(rest.slice(0, colon).trim());
        self.dataclass = false;
        self.unique = false;
        let Some(kind) = kind else {
            return Some(piece.at);
        };
        self.class = Some(kind);
        self.body_indent = None;
        let inline_body = rest.from(colon + 1).trim();
        if inline_body.text.is_empty() {
            return None;
        }
        Self::refused_member(inline_body, kind)
    }

    fn class_kind(&self, header: Piece<'_>) -> Option<ClassKind> {
        let mut kind = None;
        if self.dataclass {
            kind = Some(ClassKind::Fields);
        }
        let bases = header.from(header.first_word().len()).trim();
        if bases.starts_with("(") {
            let close = bases.closing(0)?;
            for base in bases.slice(1, close).split_top(b',') {
                kind = Self::base_kind(base).or(kind);
            }
        }
        if self.unique && kind != Some(ClassKind::Members) {
            return None;
        }
        kind
    }

    fn base_kind(base: Piece<'_>) -> Option<ClassKind> {
        if base.text.contains('=') {
            return None;
        }
        let name = Piece::last_segment(base.path());
        if Self::ENUM_BASES.contains(&name) {
            return Some(ClassKind::Members);
        }
        if Self::FIELD_BASES.contains(&name) {
            return Some(ClassKind::Fields);
        }
        None
    }

    fn read_member(&mut self, statement: &Statement<'_>) -> Option<usize> {
        let Some(kind) = self.class else {
            return Some(statement.piece.at);
        };
        let body_indent = *self.body_indent.get_or_insert(statement.indent);
        if statement.indent != body_indent {
            return Some(statement.piece.at);
        }
        Self::refused_member(statement.piece, kind)
    }

    fn refused_member(piece: Piece<'_>, kind: ClassKind) -> Option<usize> {
        if matches!(piece.text, "pass" | "...") {
            return None;
        }
        let name = piece.first_word();
        if name.is_empty() || Self::KEYWORDS_BEFORE_COLON.contains(&name) {
            return Some(piece.at);
        }
        let rest = piece.from(name.len()).trim();
        if rest.starts_with(":") && !rest.starts_with(":=") {
            return None;
        }
        let is_member = rest.starts_with("=") && !rest.starts_with("==");
        if kind == ClassKind::Members && is_member {
            return None;
        }
        Some(piece.at)
    }
}

pub(super) fn first_refused(code: Piece<'_>) -> Option<usize> {
    if let Some(index) = code.text.find(';') {
        return Some(code.at + index);
    }
    let mut model = PythonModel::default();
    for statement in statements(code) {
        if let Some(at) = model.read(&statement) {
            return Some(at);
        }
    }
    None
}

fn statements(code: Piece<'_>) -> Vec<Statement<'_>> {
    let mut statements = Vec::new();
    let mut depth = 0_usize;
    let mut start = 0;
    for (index, byte) in code.text.bytes().enumerate() {
        depth = Piece::depth_after(byte, depth);
        if byte == b'\n' && depth == 0 && !ends_with_backslash(code.text, index) {
            push_statement(&mut statements, code.slice(start, index));
            start = index + 1;
        }
    }
    push_statement(&mut statements, code.from(start));
    statements
}

fn ends_with_backslash(text: &str, newline: usize) -> bool {
    text[..newline].trim_end_matches('\r').ends_with('\\')
}

fn push_statement<'a>(statements: &mut Vec<Statement<'a>>, line: Piece<'a>) {
    let piece = line.trim();
    if piece.text.is_empty() {
        return;
    }
    statements.push(Statement {
        indent: piece.at - line.at,
        piece,
    });
}

use super::{CppMemberKind, CppShape, CppTokens};
use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CppDeclaration {
    Skipped,
    Nested,
    Operator,
    Function(usize),
    Field(usize),
}

impl CppDeclaration {
    const NESTED: [&'static str; 4] = ["class", "struct", "enum", "union"];
    const OTHER: [&'static str; 5] = ["using", "friend", "static_assert", "typedef", "template"];

    pub(crate) fn of(
        tokens: &CppTokens,
        declaration: &Range<usize>,
        specifiers: usize,
        shape: &CppShape,
    ) -> Self {
        let first = tokens.text(declaration.start);
        let skipped = [Self::OTHER.contains(&first), declaration.is_empty()].contains(&true);
        let nested = Self::NESTED.contains(&tokens.text(specifiers));
        let cut = shape.cut.unwrap_or(declaration.end);
        Self::classify([skipped, nested, shape.operator], shape.parameters, cut)
    }

    pub(crate) fn function_kind(destructor: bool, constructor: bool) -> CppMemberKind {
        Self::choose_function_kind(destructor, constructor)
    }

    fn classify(flags: [bool; 3], parameters: Option<usize>, cut: usize) -> Self {
        match (flags, parameters) {
            ([true, _, _], _) => Self::Skipped,
            ([false, true, _], _) => Self::Nested,
            ([false, false, true], _) => Self::Operator,
            ([false, false, false], Some(open)) => Self::Function(open),
            ([false, false, false], None) => Self::Field(cut),
        }
    }

    fn choose_function_kind(destructor: bool, constructor: bool) -> CppMemberKind {
        match (destructor, constructor) {
            (true, _) => CppMemberKind::Destructor,
            (false, true) => CppMemberKind::Constructor,
            (false, false) => CppMemberKind::Method,
        }
    }
}

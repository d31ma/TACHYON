#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CppMemberKind {
    Field,
    Method,
    Constructor,
    Destructor,
    Operator,
    NestedType,
    Other,
}

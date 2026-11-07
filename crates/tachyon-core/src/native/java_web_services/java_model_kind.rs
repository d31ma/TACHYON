#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum JavaModelKind {
    Record,
    Class,
    Enumeration { constants: Vec<String> },
}

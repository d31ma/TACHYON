#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum JavaMemberKind {
    Field { read_only: bool },
    Method,
    Hook { destroy: bool },
    Listener { key: String, description: String },
}

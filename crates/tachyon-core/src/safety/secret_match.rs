#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SecretMatch {
    pub(super) line: usize,
    pub(super) kind: &'static str,
    pub(super) value: String,
}

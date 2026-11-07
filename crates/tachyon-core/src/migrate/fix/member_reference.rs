#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MemberReference {
    Rename,
    Keep,
    Unsure,
}

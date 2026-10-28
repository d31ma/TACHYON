use super::ArgumentRange;

pub(super) struct Overload {
    pub(super) name: String,
    pub(super) renamed_at: usize,
    pub(super) own: ArgumentRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct BudgetLimit {
    pub(super) name: &'static str,
    pub(super) setting: &'static str,
    pub(super) extensions: &'static [&'static str],
    pub(super) bytes: u64,
}

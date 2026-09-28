#[derive(Clone, Debug)]
pub(super) struct Finding {
    pub(super) at: usize,
    pub(super) word: String,
    pub(super) counts: bool,
    pub(super) branches: bool,
}

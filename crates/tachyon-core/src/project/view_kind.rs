#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewKind {
    Tac,
}

impl ViewKind {
    pub(super) const TAC_FILE_NAME: &'static str = "tac.html";
    pub(super) const TAC_STEM: &'static str = "tac.";
}

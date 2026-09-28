#[allow(clippy::struct_excessive_bools)]
pub(super) struct Word<'a> {
    pub(super) text: &'a str,
    pub(super) start: usize,
    pub(super) literal: bool,
    pub(super) depth: usize,
    pub(super) partner: Option<usize>,
    pub(super) pair: String,
    pub(super) touches_previous: bool,
    pub(super) spaced: bool,
    pub(super) line_start: bool,
    pub(super) column: usize,
}

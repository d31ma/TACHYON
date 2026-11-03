#[derive(Clone, Copy, Debug)]
pub(super) struct Piece<'a> {
    pub(super) text: &'a str,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) depth: usize,
    pub(super) partner: Option<usize>,
    pub(super) line_start: bool,
    pub(super) column: usize,
    pub(super) literal: bool,
}

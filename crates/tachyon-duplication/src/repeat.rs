#[derive(Clone, Copy, Debug)]
pub(crate) struct Repeat {
    pub(crate) earlier: usize,
    pub(crate) later: usize,
    pub(crate) length: usize,
}

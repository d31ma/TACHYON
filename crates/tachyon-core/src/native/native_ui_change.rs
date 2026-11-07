#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeUiChange {
    Unchanged,
    PrepareStyles,
    RestartRequired,
}

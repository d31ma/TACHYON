#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodState {
    Called(String),
    Named,
    Missing,
}

impl MethodState {
    #[must_use]
    pub fn is_covered(&self) -> bool {
        *self != Self::Missing
    }
}

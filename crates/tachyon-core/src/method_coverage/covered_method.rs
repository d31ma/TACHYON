use super::MethodState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveredMethod {
    pub name: String,
    pub state: MethodState,
}

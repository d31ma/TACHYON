#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uncovered {
    pub file: String,
    pub type_name: String,
    pub method: String,
    pub recorded: bool,
}

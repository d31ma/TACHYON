use super::ProjectChoices;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdditionResult {
    pub choices: ProjectChoices,
    pub written: Vec<String>,
    pub notes: Vec<String>,
}

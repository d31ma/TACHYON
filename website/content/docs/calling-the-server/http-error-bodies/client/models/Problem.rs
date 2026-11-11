use crate::models::ProblemEntry;
#[derive(Clone, Debug, Default)]
pub struct Problem {
    pub title: String,
    pub status: i64,
    pub errors: Vec<ProblemEntry>,
}

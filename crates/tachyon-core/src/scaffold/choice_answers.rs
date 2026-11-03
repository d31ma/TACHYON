#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ChoiceAnswers {
    pub realm: Option<String>,
    pub tac: Option<String>,
    pub yon: Option<String>,
    pub targets: Option<String>,
    pub features: Option<String>,
}

impl ChoiceAnswers {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let answers = [
            &self.realm,
            &self.tac,
            &self.yon,
            &self.targets,
            &self.features,
        ];
        answers.iter().all(|answer| answer.is_none())
    }
}

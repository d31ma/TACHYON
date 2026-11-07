use super::{
    AppFeature, AppTarget, Choice, ChoiceAnswers, ChoiceError, ChoiceList, RealmChoice,
    TacLanguage, YonLanguage,
};
use crate::Failure;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectChoices {
    pub realm: RealmChoice,
    pub tac: Vec<TacLanguage>,
    pub yon: Vec<YonLanguage>,
    pub targets: Vec<AppTarget>,
    pub features: Vec<AppFeature>,
}

impl ProjectChoices {
    const REALM: &'static str = "realm";
    const TAC: &'static str = "Tac language";
    const YON: &'static str = "Yon language";
    const TARGET: &'static str = "target";
    const FEATURE: &'static str = "feature";
    const BOTH_HELP: &'static str = "Choose --realm both, or remove the option.";

    #[must_use]
    pub fn defaults() -> Self {
        Self {
            realm: RealmChoice::Both,
            tac: vec![TacLanguage::JavaScript],
            yon: vec![YonLanguage::JavaScript],
            targets: vec![AppTarget::Web],
            features: Vec::new(),
        }
    }

    pub fn from_answers(answers: &ChoiceAnswers) -> Result<Self, Failure> {
        let realm = Self::realm_or_both(answers.realm.as_deref())?;
        let choices = Self {
            realm,
            tac: Self::list_or(answers.tac.as_deref(), Self::TAC, Self::default_tac(realm))?,
            yon: Self::list_or(answers.yon.as_deref(), Self::YON, Self::default_yon(realm))?,
            targets: Self::list_or(
                answers.targets.as_deref(),
                Self::TARGET,
                Self::default_targets(realm),
            )?,
            features: Self::list_or(answers.features.as_deref(), Self::FEATURE, Vec::new())?,
        };
        choices.checked()
    }

    pub fn checked(self) -> Result<Self, Failure> {
        let problems = [
            self.tac_without_client(),
            self.yon_without_server(),
            self.targets_without_client(),
            self.feature_without_server(),
            self.client_without_tac(),
            self.server_without_yon(),
        ];
        problems.into_iter().flatten().next().map_or(Ok(self), Err)
    }

    #[must_use]
    pub fn client_uses_typescript(&self) -> bool {
        let languages = [
            self.tac.contains(&TacLanguage::TypeScript),
            self.tac.contains(&TacLanguage::JavaScript),
        ];
        languages == [true, false]
    }

    #[must_use]
    pub fn server_uses_typescript(&self) -> bool {
        let languages = [
            self.yon.contains(&YonLanguage::TypeScript),
            self.yon.contains(&YonLanguage::JavaScript),
        ];
        languages == [true, false]
    }

    #[must_use]
    pub fn native_targets(&self) -> Vec<AppTarget> {
        self.targets
            .iter()
            .copied()
            .filter(|target| target.is_native())
            .collect()
    }

    fn realm_or_both(text: Option<&str>) -> Result<RealmChoice, Failure> {
        text.map_or(Ok(RealmChoice::Both), |value| {
            ChoiceList::one(value, Self::REALM)
        })
    }

    fn list_or<T: Choice>(
        text: Option<&str>,
        topic: &str,
        fallback: Vec<T>,
    ) -> Result<Vec<T>, Failure> {
        text.map_or(Ok(fallback), |value| ChoiceList::parse(value, topic))
    }

    fn default_tac(realm: RealmChoice) -> Vec<TacLanguage> {
        if realm.has_client() {
            return vec![TacLanguage::JavaScript];
        }
        Vec::new()
    }

    fn default_yon(realm: RealmChoice) -> Vec<YonLanguage> {
        if realm.has_server() {
            return vec![YonLanguage::JavaScript];
        }
        Vec::new()
    }

    fn default_targets(realm: RealmChoice) -> Vec<AppTarget> {
        if realm.has_client() {
            return vec![AppTarget::Web];
        }
        Vec::new()
    }

    fn tac_without_client(&self) -> Option<Failure> {
        if self.realm.has_client() {
            return None;
        }
        self.tac.first().map(|language| {
            ChoiceError::conflict(
                format!(
                    "{} is a Tac language, but this project has no client.",
                    language.label()
                ),
                Self::BOTH_HELP,
            )
        })
    }

    fn yon_without_server(&self) -> Option<Failure> {
        if self.realm.has_server() {
            return None;
        }
        self.yon.first().map(|language| {
            ChoiceError::conflict(
                format!(
                    "{} is a Yon language, but this project has no server.",
                    language.label()
                ),
                Self::BOTH_HELP,
            )
        })
    }

    fn targets_without_client(&self) -> Option<Failure> {
        if self.realm.has_client() {
            return None;
        }
        self.targets.first().map(|target| {
            ChoiceError::conflict(
                format!(
                    "The {} target builds pages, but this project has no client.",
                    target.flag_name()
                ),
                Self::BOTH_HELP,
            )
        })
    }

    fn feature_without_server(&self) -> Option<Failure> {
        if self.realm.has_server() {
            return None;
        }
        let feature = self
            .features
            .iter()
            .copied()
            .find(|feature| feature.needs_server())?;
        Some(ChoiceError::conflict(
            format!(
                "The {} example needs a server, but this project has no server.",
                feature.flag_name()
            ),
            Self::BOTH_HELP,
        ))
    }

    fn client_without_tac(&self) -> Option<Failure> {
        let missing = [self.realm.has_client(), self.tac.is_empty()] == [true, true];
        missing.then(|| {
            ChoiceError::conflict(
                String::from("A project with a client needs at least one Tac language."),
                "Choose one, for example --tac js.",
            )
        })
    }

    fn server_without_yon(&self) -> Option<Failure> {
        let missing = [self.realm.has_server(), self.yon.is_empty()] == [true, true];
        missing.then(|| {
            ChoiceError::conflict(
                String::from("A project with a server needs at least one Yon language."),
                "Choose one, for example --yon js.",
            )
        })
    }
}

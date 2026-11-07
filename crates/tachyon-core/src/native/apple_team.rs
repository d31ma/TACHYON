use crate::Failure;
use crate::failure::diagnostic;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AppleTeam {
    id: String,
}

impl AppleTeam {
    pub(crate) const VARIABLE: &'static str = "TAC_APPLE_TEAM_ID";
    const LENGTH: usize = 10;
    const HELP: &'static str = "Correct TAC_APPLE_TEAM_ID in the environment that runs ty, or unset it. It never goes in client/manifest.json.";
    const PROBLEM: &'static str = "TAC_APPLE_TEAM_ID must be your Apple Developer team ID: 10 uppercase letters and digits, such as A1B2C3D4E5. Find it under Membership details in your Apple Developer account.";

    pub(crate) fn from_environment() -> Result<Option<Self>, Failure> {
        Self::parse(std::env::var(Self::VARIABLE).ok().as_deref())
    }

    pub(crate) fn parse(value: Option<&str>) -> Result<Option<Self>, Failure> {
        value
            .filter(|text| !text.is_empty())
            .map(Self::checked)
            .transpose()
    }

    pub(crate) fn app_id(&self, bundle_id: &str) -> String {
        format!("{}.{bundle_id}", self.id)
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    fn refusal() -> Failure {
        Failure::one(diagnostic(
            1601,
            Self::PROBLEM,
            Some(String::from(Self::HELP)),
            None,
        ))
    }

    fn checked(text: &str) -> Result<Self, Failure> {
        let fits = text.len() == Self::LENGTH;
        let characters = text
            .bytes()
            .all(|byte| [byte.is_ascii_uppercase(), byte.is_ascii_digit()].contains(&true));
        [fits, characters]
            .eq(&[true, true])
            .then(|| Self {
                id: String::from(text),
            })
            .ok_or_else(Self::refusal)
    }
}

#[cfg(test)]
mod tests {
    use super::AppleTeam;

    #[test]
    fn a_ten_character_team_id_names_the_app() {
        let team = AppleTeam::parse(Some("A1B2C3D4E5")).ok().flatten();
        assert_eq!(
            team.map(|team| team.app_id("com.example.app")),
            Some(String::from("A1B2C3D4E5.com.example.app"))
        );
    }

    #[test]
    fn an_unset_or_blank_team_id_means_no_team() {
        assert_eq!(AppleTeam::parse(None).ok(), Some(None));
        assert_eq!(AppleTeam::parse(Some("")).ok(), Some(None));
    }

    #[test]
    fn a_malformed_team_id_is_refused() {
        for value in [
            "a1b2c3d4e5",
            "A1B2C3D4E",
            "A1B2C3D4E5F",
            "A1B2C3D4E-",
            "A1B2C3D4É",
        ] {
            assert!(AppleTeam::parse(Some(value)).is_err(), "{value}");
        }
    }
}

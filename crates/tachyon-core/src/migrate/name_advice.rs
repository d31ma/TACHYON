use crate::application::NameRefusal;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum NameAdvice {
    Rename(String),
    Setting(&'static str),
    Unnamed,
}

impl NameAdvice {
    const SETTINGS: [(&'static str, &'static str); 5] = [
        (
            "YON_PORT",
            "set server.port in server/application.json, or pass --port to ty start",
        ),
        ("YON_HOST", "set server.host in server/application.json"),
        ("YON_HOSTNAME", "set server.host in server/application.json"),
        (
            "YON_PROFILE",
            "set YON_PROFILE in the shell that runs ty start",
        ),
        (
            "YON_SKIP_BUNDLE",
            "pass --no-bundle to ty preview, or set YON_SKIP_BUNDLE in the shell that runs it",
        ),
    ];
    const OTHER_SETTING: &'static str = "set this framework setting in server/application.json or a profile, or in the shell that runs ty";

    pub(super) fn of(name: &str, refusal: NameRefusal) -> Self {
        Self::advised(name, refusal)
    }

    fn advised(name: &str, refusal: NameRefusal) -> Self {
        match refusal {
            NameRefusal::Reserved => Self::Setting(Self::setting(name)),
            NameRefusal::NotSecret => Self::renamed(name),
            NameRefusal::Segments => Self::Unnamed,
        }
    }

    fn setting(name: &str) -> &'static str {
        Self::SETTINGS
            .iter()
            .find(|(reserved, _)| *reserved == name)
            .map_or(Self::OTHER_SETTING, |(_, setting)| *setting)
    }

    fn renamed(name: &str) -> Self {
        let upper = name.to_ascii_uppercase().replace(['-', '.'], "_");
        let rest = upper.strip_prefix("TAC_").unwrap_or(&upper);
        let bare = rest.strip_prefix("YON_").unwrap_or(rest);
        Self::checked(format!("YON_{bare}"))
    }

    fn checked(candidate: String) -> Self {
        match NameRefusal::of(&candidate) {
            None => Self::Rename(candidate),
            Some(NameRefusal::Reserved) => Self::Setting(Self::setting(&candidate)),
            Some(_) => Self::Unnamed,
        }
    }
}

use crate::Failure;
use crate::failure::diagnostic;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct IosSigning {
    pub(super) team: String,
    pub(super) profile: PathBuf,
    pub(super) identity: Option<String>,
}

impl IosSigning {
    pub(super) const TEAM: &'static str = "TAC_APPLE_TEAM_ID";
    pub(super) const PROFILE: &'static str = "TAC_IOS_PROVISIONING_PROFILE";
    pub(super) const IDENTITY: &'static str = "TAC_IOS_SIGNING_IDENTITY";
    const REQUIRED: [&'static str; 2] = [Self::TEAM, Self::PROFILE];
    const TEAM_LENGTH: usize = 10;
    pub(super) const HELP: &'static str = "Set TAC_APPLE_TEAM_ID to your Apple Developer team ID \
         and TAC_IOS_PROVISIONING_PROFILE to the app's .mobileprovision file: a development or \
         ad hoc profile for --device, an App Store profile for --release. The signing \
         certificate is the one in the keychain that the profile names; set \
         TAC_IOS_SIGNING_IDENTITY to a certificate name or SHA-1 only to choose another. To \
         build for the simulator instead, remove --device and --release.";

    pub(super) fn from_process_environment() -> Result<Self, Failure> {
        let working_directory = super::host::native_io(std::env::current_dir(), Path::new("."))?;
        Self::from_environment(|name| std::env::var_os(name), &working_directory)
    }

    pub(super) fn from_environment(
        lookup: impl Fn(&str) -> Option<OsString>,
        working_directory: &Path,
    ) -> Result<Self, Failure> {
        Self::require_variables(&lookup)?;
        let team = lookup(Self::TEAM).unwrap_or_default();
        let team = team.to_string_lossy().trim().to_owned();
        Self::require_team(&team)?;
        let named = PathBuf::from(lookup(Self::PROFILE).unwrap_or_default());
        let profile = working_directory.join(named);
        Self::require_file(&profile)?;
        let identity = lookup(Self::IDENTITY)
            .map(|value| value.to_string_lossy().trim().to_owned())
            .filter(|value| !value.is_empty());
        Ok(Self {
            team,
            profile,
            identity,
        })
    }

    fn require_variables(lookup: &impl Fn(&str) -> Option<OsString>) -> Result<(), Failure> {
        let missing = Self::REQUIRED.map(|name| (name, lookup(name).unwrap_or_default()));
        let names: Vec<&str> = missing
            .iter()
            .filter(|(_, value)| value.is_empty())
            .map(|(name, _)| *name)
            .collect();
        let message = format!(
            "A device or App Store iOS build is signed with a provisioning profile and a \
             certificate from the build machine, but these variables are missing: {}.",
            names.join(", ")
        );
        names
            .is_empty()
            .then_some(())
            .ok_or_else(|| Self::failure(&message))
    }

    fn require_team(team: &str) -> Result<(), Failure> {
        let well_formed = [
            team.len() == Self::TEAM_LENGTH,
            team.chars().all(|character| {
                [character.is_ascii_uppercase(), character.is_ascii_digit()].contains(&true)
            }),
        ] == [true, true];
        well_formed.then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "TAC_APPLE_TEAM_ID is '{team}', but an Apple team ID is 10 capital letters and \
                 digits, such as A1B2C3D4E5. Find it under Membership in your Apple Developer \
                 account."
            ))
        })
    }

    fn require_file(profile: &Path) -> Result<(), Failure> {
        let message = format!(
            "TAC_IOS_PROVISIONING_PROFILE names '{}', which is not a provisioning profile file.",
            profile.display()
        );
        profile
            .is_file()
            .then_some(())
            .ok_or_else(|| Self::failure(&message))
    }

    pub(super) fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            1605,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::IosSigning;
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    fn lookup<'values>(
        values: &'values BTreeMap<&'static str, String>,
    ) -> impl Fn(&str) -> Option<OsString> + 'values {
        |name| values.get(name).map(OsString::from)
    }

    #[test]
    fn a_signed_ios_build_names_every_missing_variable() {
        let root = tempfile::tempdir().expect("root");
        let values = BTreeMap::from([(IosSigning::IDENTITY, String::from("Apple Development"))]);
        let text = IosSigning::from_environment(lookup(&values), root.path())
            .expect_err("missing")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(
            text.contains("missing: TAC_APPLE_TEAM_ID, TAC_IOS_PROVISIONING_PROFILE."),
            "{text}"
        );
        assert!(text.contains("remove --device and --release"), "{text}");
    }

    #[test]
    fn a_team_id_that_apple_would_not_issue_is_refused() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("app.mobileprovision"), b"profile").expect("profile");
        for team in ["abcde12345", "ABCDE1234", "ABCDE-2345"] {
            let values = BTreeMap::from([
                (IosSigning::TEAM, String::from(team)),
                (IosSigning::PROFILE, String::from("app.mobileprovision")),
            ]);
            let text = IosSigning::from_environment(lookup(&values), root.path())
                .expect_err("bad team")
                .to_string();
            assert!(text.contains("10 capital letters and digits"), "{text}");
        }
    }

    #[test]
    fn the_profile_is_read_from_the_working_directory() {
        let root = tempfile::tempdir().expect("root");
        let missing = BTreeMap::from([
            (IosSigning::TEAM, String::from("A1B2C3D4E5")),
            (
                IosSigning::PROFILE,
                String::from("profiles/app.mobileprovision"),
            ),
        ]);
        let text = IosSigning::from_environment(lookup(&missing), root.path())
            .expect_err("no file")
            .to_string();
        assert!(text.contains("profiles/app.mobileprovision"), "{text}");

        std::fs::create_dir_all(root.path().join("profiles")).expect("profiles");
        std::fs::write(root.path().join("profiles/app.mobileprovision"), b"profile")
            .expect("profile");
        let mut values = missing;
        values.insert(IosSigning::IDENTITY, String::from("  "));
        let signing = IosSigning::from_environment(lookup(&values), root.path()).expect("signing");
        assert_eq!(signing.team, "A1B2C3D4E5");
        assert_eq!(
            signing.profile,
            root.path().join("profiles/app.mobileprovision")
        );
        assert_eq!(signing.identity, None);

        values.insert(
            IosSigning::IDENTITY,
            String::from("Apple Distribution: Example"),
        );
        let chosen = IosSigning::from_environment(lookup(&values), root.path()).expect("chosen");
        assert_eq!(
            chosen.identity.as_deref(),
            Some("Apple Distribution: Example")
        );
    }
}

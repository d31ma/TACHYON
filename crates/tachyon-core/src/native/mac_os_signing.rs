use crate::Failure;
use crate::failure::diagnostic;
use std::ffi::OsString;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MacOsSigning {
    identity: String,
    team: String,
    notary_profile: String,
    provisioning_profile: String,
}

impl MacOsSigning {
    pub(super) const TEAM_ID: &'static str = "TAC_APPLE_TEAM_ID";
    pub(super) const NOTARY_PROFILE: &'static str = "TAC_APPLE_NOTARY_PROFILE";
    pub(super) const SIGNING_IDENTITY: &'static str = "TAC_MACOS_SIGNING_IDENTITY";
    pub(super) const PROVISIONING_PROFILE: &'static str = "TAC_MACOS_PROVISIONING_PROFILE";
    pub(super) const AD_HOC: &'static str = "-";
    const TEAM_ID_LENGTH: usize = 10;
    const TEAM_ID_BYTES: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const REQUIRED: [&'static [&'static str]; 2] = [&[Self::TEAM_ID, Self::NOTARY_PROFILE], &[]];
    const HELP: &'static str = "Set TAC_APPLE_TEAM_ID to your Apple Developer team ID and \
         TAC_APPLE_NOTARY_PROFILE to a notarytool keychain profile \
         (xcrun notarytool store-credentials). Keep the Developer ID Application certificate in \
         the keychain, or name it in TAC_MACOS_SIGNING_IDENTITY. To test on this Mac without a \
         certificate, set TAC_MACOS_SIGNING_IDENTITY=- for an ad hoc signature. To build the app \
         without a package, remove --release.";

    pub(super) fn from_environment(
        lookup: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self, Failure> {
        let signing = Self {
            identity: Self::value(&lookup, Self::SIGNING_IDENTITY),
            team: Self::value(&lookup, Self::TEAM_ID),
            notary_profile: Self::value(&lookup, Self::NOTARY_PROFILE),
            provisioning_profile: Self::value(&lookup, Self::PROVISIONING_PROFILE),
        };
        signing.require_variables()?;
        signing.require_team_id()?;
        signing.require_developer_id_for_profile()?;
        Ok(signing)
    }

    pub(super) fn for_release(release: bool) -> Result<Option<Self>, Failure> {
        release
            .then(|| Self::from_environment(|name| std::env::var_os(name)))
            .transpose()
    }

    pub(super) fn is_ad_hoc(&self) -> bool {
        self.identity == Self::AD_HOC
    }

    pub(super) fn named_identity(&self) -> Option<&str> {
        Some(self.identity.as_str()).filter(|identity| !identity.is_empty())
    }

    pub(super) fn team(&self) -> &str {
        &self.team
    }

    pub(super) fn notary_profile(&self) -> &str {
        &self.notary_profile
    }

    pub(super) fn provisioning_profile(&self) -> Option<&str> {
        Some(self.provisioning_profile.as_str()).filter(|profile| !profile.is_empty())
    }

    pub(super) fn note(&self) -> Option<String> {
        self.is_ad_hoc().then(|| {
            String::from(
                "Note: the macOS release is signed ad hoc (TAC_MACOS_SIGNING_IDENTITY=-) and not \
                 notarized, so its .dmg opens only on this Mac. Set TAC_APPLE_TEAM_ID and \
                 TAC_APPLE_NOTARY_PROFILE, and leave TAC_MACOS_SIGNING_IDENTITY unset, to build \
                 a package for other Macs.",
            )
        })
    }

    pub(super) fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            1605,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }

    fn value(lookup: &impl Fn(&str) -> Option<OsString>, name: &str) -> String {
        lookup(name)
            .map(|value| value.to_string_lossy().trim().to_owned())
            .unwrap_or_default()
    }

    fn require_variables(&self) -> Result<(), Failure> {
        let values = [
            (Self::TEAM_ID, self.team.as_str()),
            (Self::NOTARY_PROFILE, self.notary_profile.as_str()),
        ];
        let required = Self::REQUIRED[usize::from(self.is_ad_hoc())];
        let missing: Vec<&str> = values
            .into_iter()
            .filter(|(name, value)| [required.contains(name), value.is_empty()] == [true, true])
            .map(|(name, _)| name)
            .collect();
        missing.is_empty().then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "A release macOS build is signed with a Developer ID certificate and notarized, \
                 but these variables are missing: {}.",
                missing.join(", ")
            ))
        })
    }

    fn require_developer_id_for_profile(&self) -> Result<(), Failure> {
        let refused = [self.is_ad_hoc(), self.provisioning_profile().is_some()] == [true, true];
        (!refused).then_some(()).ok_or_else(|| {
            Self::failure(
                "TAC_MACOS_PROVISIONING_PROFILE gives the app associated domains, which only a \
                 Developer ID signature can carry, but TAC_MACOS_SIGNING_IDENTITY=- signs it ad \
                 hoc.",
            )
        })
    }

    fn require_team_id(&self) -> Result<(), Failure> {
        let well_formed = [
            self.team.len() == Self::TEAM_ID_LENGTH,
            self.team
                .bytes()
                .all(|byte| Self::TEAM_ID_BYTES.contains(&byte)),
        ] == [true, true];
        [self.is_ad_hoc(), well_formed]
            .contains(&true)
            .then_some(())
            .ok_or_else(|| {
                Self::failure(&format!(
                    "TAC_APPLE_TEAM_ID is '{}', but an Apple Developer team ID is 10 capital \
                     letters and digits, such as A1B2C3D4E5.",
                    self.team
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::MacOsSigning;
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    fn signing(values: &[(&'static str, &str)]) -> Result<MacOsSigning, crate::Failure> {
        let values: BTreeMap<&str, String> = values
            .iter()
            .map(|(name, value)| (*name, (*value).to_owned()))
            .collect();
        MacOsSigning::from_environment(|name| values.get(name).map(OsString::from))
    }

    #[test]
    fn a_release_build_names_every_missing_apple_variable() {
        let text = signing(&[(MacOsSigning::NOTARY_PROFILE, " ")])
            .expect_err("missing")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(
            text.contains("missing: TAC_APPLE_TEAM_ID, TAC_APPLE_NOTARY_PROFILE."),
            "{text}"
        );
        assert!(text.contains("TAC_MACOS_SIGNING_IDENTITY=-"), "{text}");
    }

    #[test]
    fn a_team_id_must_look_like_an_apple_team_id() {
        let text = signing(&[
            (MacOsSigning::TEAM_ID, "team-one"),
            (MacOsSigning::NOTARY_PROFILE, "tachyon"),
        ])
        .expect_err("malformed")
        .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("'team-one'"), "{text}");
    }

    #[test]
    fn a_developer_id_release_reads_the_team_and_the_notary_profile() {
        let release = signing(&[
            (MacOsSigning::TEAM_ID, "A1B2C3D4E5"),
            (MacOsSigning::NOTARY_PROFILE, "tachyon-notary"),
        ])
        .expect("signing");
        assert!(!release.is_ad_hoc());
        assert_eq!(release.named_identity(), None);
        assert_eq!(release.team(), "A1B2C3D4E5");
        assert_eq!(release.notary_profile(), "tachyon-notary");
        assert_eq!(release.note(), None);
        let named = signing(&[
            (MacOsSigning::TEAM_ID, "A1B2C3D4E5"),
            (MacOsSigning::NOTARY_PROFILE, "tachyon-notary"),
            (
                MacOsSigning::SIGNING_IDENTITY,
                "Developer ID Application: Example (A1B2C3D4E5)",
            ),
        ])
        .expect("named");
        assert_eq!(
            named.named_identity(),
            Some("Developer ID Application: Example (A1B2C3D4E5)")
        );
    }

    #[test]
    fn a_provisioning_profile_needs_a_developer_id_signature() {
        let release = signing(&[
            (MacOsSigning::TEAM_ID, "A1B2C3D4E5"),
            (MacOsSigning::NOTARY_PROFILE, "tachyon-notary"),
            (
                MacOsSigning::PROVISIONING_PROFILE,
                " Shop.provisionprofile ",
            ),
        ])
        .expect("signing");
        assert_eq!(
            release.provisioning_profile(),
            Some("Shop.provisionprofile")
        );
        let text = signing(&[
            (MacOsSigning::SIGNING_IDENTITY, "-"),
            (MacOsSigning::PROVISIONING_PROFILE, "Shop.provisionprofile"),
        ])
        .expect_err("ad hoc")
        .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("signs it ad hoc"), "{text}");
        let plain = signing(&[(MacOsSigning::SIGNING_IDENTITY, "-")]).expect("ad hoc");
        assert_eq!(plain.provisioning_profile(), None);
    }

    #[test]
    fn an_ad_hoc_release_needs_no_team_and_says_it_is_not_notarized() {
        let release = signing(&[(MacOsSigning::SIGNING_IDENTITY, "-")]).expect("ad hoc");
        assert!(release.is_ad_hoc());
        let note = release.note().expect("note");
        assert!(note.contains("not notarized"), "{note}");
        assert!(note.contains("only on this Mac"), "{note}");
    }
}

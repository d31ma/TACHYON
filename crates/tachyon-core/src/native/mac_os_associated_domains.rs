use super::config::NativeApplication;
use super::host::{native_io, run_tool, xml_escape};
use super::ios_provisioning_profile::IosProvisioningProfile;
use super::mac_os_signing::MacOsSigning;
use super::macos::unicode_path;
use crate::Failure;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MacOsAssociatedDomains {
    profile: PathBuf,
    entries: String,
}

impl MacOsAssociatedDomains {
    const EMBEDDED_PROFILE: &'static str = "embedded.provisionprofile";
    const DECODED_PROFILE: &'static str = "provisioning-profile.plist";
    const APPLICATION_IDENTIFIER: &'static str = "<key>com.apple.application-identifier</key>";
    const UNICODE: &'static str = "Provisioning profile path is not valid Unicode.";

    pub(super) async fn prepare(
        signing: &MacOsSigning,
        application: &NativeApplication,
        stage: &Path,
    ) -> Result<Option<Self>, Failure> {
        let profile = signing
            .provisioning_profile()
            .filter(|_| !application.links.hosts().is_empty());
        Self::prepare_with(profile, signing, application, stage).await
    }

    pub(super) fn entries(domains: Option<&Self>) -> &str {
        domains.map_or("", |domains| domains.entries.as_str())
    }

    pub(super) fn embed(domains: Option<&Self>, bundle: &Path) -> Result<(), Failure> {
        domains
            .map(|domains| {
                let embedded = bundle.join("Contents").join(Self::EMBEDDED_PROFILE);
                native_io(std::fs::copy(&domains.profile, &embedded), &embedded).map(|_| ())
            })
            .transpose()
            .map(|_| ())
    }

    pub(super) fn checked(
        profile: &IosProvisioningProfile,
        team: &str,
        application: &NativeApplication,
    ) -> Result<String, Failure> {
        let name = profile.name();
        let wanted = format!("{team}.{}", application.application_id);
        let pattern = profile
            .string_after(Self::APPLICATION_IDENTIFIER)
            .unwrap_or_default();
        let covers = pattern
            .strip_suffix('*')
            .map_or(pattern == wanted, |prefix| wanted.starts_with(prefix));
        Self::require(
            profile.team() == team,
            &format!(
                "The provisioning profile '{name}' belongs to team '{}', but TAC_APPLE_TEAM_ID is '{team}'.",
                profile.team()
            ),
        )?;
        Self::require(
            covers,
            &format!(
                "The provisioning profile '{name}' is for the app ID '{pattern}', which does not cover '{wanted}'. Make a Developer ID profile for tac.application.id '{}' in your Apple Developer account.",
                application.application_id
            ),
        )?;
        Self::require(
            profile.has_associated_domains(),
            &format!(
                "tac.application.links lists hosts, but the provisioning profile '{name}' has no Associated Domains capability. Turn on Associated Domains for the app ID in your Apple Developer account, then download the Developer ID profile again."
            ),
        )?;
        Ok(Self::entitlement_entries(team, application))
    }

    fn entitlement_entries(team: &str, application: &NativeApplication) -> String {
        let domains = application
            .links
            .hosts()
            .iter()
            .map(|host| format!("<string>applinks:{}</string>", xml_escape(host)))
            .collect::<Vec<_>>()
            .concat();
        format!(
            "{}<string>{}</string>\n<key>com.apple.developer.team-identifier</key><string>{}</string>\n<key>com.apple.developer.associated-domains</key><array>{domains}</array>\n",
            Self::APPLICATION_IDENTIFIER,
            xml_escape(&format!("{team}.{}", application.application_id)),
            xml_escape(team),
        )
    }

    async fn prepare_with(
        profile: Option<&str>,
        signing: &MacOsSigning,
        application: &NativeApplication,
        stage: &Path,
    ) -> Result<Option<Self>, Failure> {
        let Some(profile) = profile else {
            return Ok(None);
        };
        Self::read(Path::new(profile), signing, application, stage)
            .await
            .map(Some)
    }

    async fn read(
        profile: &Path,
        signing: &MacOsSigning,
        application: &NativeApplication,
        stage: &Path,
    ) -> Result<Self, Failure> {
        Self::require(
            profile.is_file(),
            &format!(
                "TAC_MACOS_PROVISIONING_PROFILE names '{}', which is not a provisioning profile file.",
                profile.display()
            ),
        )?;
        let decoded = stage.join("project").join(Self::DECODED_PROFILE);
        let arguments = [
            "cms",
            "-D",
            "-i",
            unicode_path(profile, Self::UNICODE)?,
            "-o",
            unicode_path(&decoded, Self::UNICODE)?,
        ];
        run_tool("/usr/bin/security", &arguments).await?;
        let text = native_io(std::fs::read_to_string(&decoded), &decoded)?;
        let entries = Self::checked(
            &IosProvisioningProfile::from_plist(text),
            signing.team(),
            application,
        )?;
        Ok(Self {
            profile: profile.to_path_buf(),
            entries,
        })
    }

    fn require(fits: bool, message: &str) -> Result<(), Failure> {
        fits.then_some(())
            .ok_or_else(|| MacOsSigning::failure(message))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::MacOsAssociatedDomains;
    use crate::native::config::NativeApplication;
    use crate::native::ios_provisioning_profile::IosProvisioningProfile;
    use crate::native::native_links::NativeLinks;

    fn application(hosts: &[&str]) -> NativeApplication {
        let root = tempfile::tempdir().expect("project");
        let mut application = NativeApplication::discover(root.path()).expect("application");
        application.application_id = String::from("com.example.shop");
        application.links = NativeLinks::declared(&["shop"], hosts);
        application
    }

    fn profile(application: &str, domains: bool) -> IosProvisioningProfile {
        let domains = [
            "",
            "\t\t<key>com.apple.developer.associated-domains</key>\n\t\t<string>*</string>\n",
        ][usize::from(domains)];
        IosProvisioningProfile::from_plist(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n\
             \t<key>Entitlements</key>\n\t<dict>\n\
             \t\t<key>com.apple.application-identifier</key>\n\t\t<string>{application}</string>\n\
             {domains}\
             \t\t<key>com.apple.developer.team-identifier</key>\n\t\t<string>A1B2C3D4E5</string>\n\
             \t</dict>\n\
             \t<key>Name</key>\n\t<string>Shop Developer ID</string>\n\
             \t<key>TeamIdentifier</key>\n\t<array>\n\t\t<string>A1B2C3D4E5</string>\n\t</array>\n\
             </dict>\n</plist>\n"
        ))
    }

    #[test]
    fn a_developer_id_profile_with_associated_domains_adds_one_applinks_entry_per_host() {
        let entries = MacOsAssociatedDomains::checked(
            &profile("A1B2C3D4E5.com.example.shop", true),
            "A1B2C3D4E5",
            &application(&["example.com", "www.example.com"]),
        )
        .expect("entitlements");
        assert_eq!(
            entries,
            "<key>com.apple.application-identifier</key><string>A1B2C3D4E5.com.example.shop</string>\n\
             <key>com.apple.developer.team-identifier</key><string>A1B2C3D4E5</string>\n\
             <key>com.apple.developer.associated-domains</key><array><string>applinks:example.com</string><string>applinks:www.example.com</string></array>\n"
        );
    }

    #[test]
    fn a_profile_for_another_team_app_or_without_the_capability_stops_the_release() {
        let hosts = application(&["example.com"]);
        let cases = [
            (
                profile("A1B2C3D4E5.com.example.shop", true),
                "Z9Y8X7W6V5",
                "belongs to team 'A1B2C3D4E5'",
            ),
            (
                profile("A1B2C3D4E5.com.example.other", true),
                "A1B2C3D4E5",
                "does not cover 'A1B2C3D4E5.com.example.shop'",
            ),
            (
                profile("A1B2C3D4E5.com.example.shop", false),
                "A1B2C3D4E5",
                "no Associated Domains capability",
            ),
        ];
        for (profile, team, expected) in cases {
            let text = MacOsAssociatedDomains::checked(&profile, team, &hosts)
                .expect_err("refused")
                .to_string();
            assert!(text.contains("TY1605"), "{text}");
            assert!(text.contains(expected), "{text}");
        }
    }

    #[test]
    fn without_a_profile_the_release_signs_no_associated_domains() {
        assert_eq!(MacOsAssociatedDomains::entries(None), "");
        let bundle = tempfile::tempdir().expect("bundle");
        MacOsAssociatedDomains::embed(None, bundle.path()).expect("nothing to embed");
        assert!(!bundle.path().join("Contents").exists());
    }

    #[test]
    fn a_checked_profile_is_embedded_in_the_bundle() {
        let root = tempfile::tempdir().expect("root");
        let source = root.path().join("Shop.provisionprofile");
        std::fs::write(&source, b"profile").expect("profile");
        let contents = root.path().join("Shop.app").join("Contents");
        std::fs::create_dir_all(&contents).expect("contents");
        let domains = MacOsAssociatedDomains {
            profile: source,
            entries: String::from("<key>k</key><true/>\n"),
        };
        MacOsAssociatedDomains::embed(Some(&domains), &root.path().join("Shop.app"))
            .expect("embedded");
        assert_eq!(
            std::fs::read(contents.join("embedded.provisionprofile")).expect("embedded"),
            b"profile"
        );
        assert_eq!(
            MacOsAssociatedDomains::entries(Some(&domains)),
            "<key>k</key><true/>\n"
        );
    }
}

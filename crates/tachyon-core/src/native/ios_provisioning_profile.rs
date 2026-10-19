use super::host::xml_escape;
use super::ios_signing::IosSigning;
use crate::Failure;
use base64::Engine as _;
use sha1::{Digest, Sha1};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct IosProvisioningProfile {
    text: String,
}

impl IosProvisioningProfile {
    const STRING_TAGS: [&'static str; 2] = ["<string>", "</string>"];
    const DATA_TAGS: [&'static str; 2] = ["<data>", "</data>"];
    const DEVICE_KEYS: [&'static str; 2] = [
        "<key>ProvisionedDevices</key>",
        "<key>ProvisionsAllDevices</key>",
    ];
    const KINDS: [&'static str; 2] = ["a development, ad hoc or enterprise", "an App Store"];
    const WANTED: [&'static str; 2] = [
        "--device installs on a registered iPhone or iPad, so it needs a development or ad hoc profile. Build with --release for App Store Connect.",
        "--release builds for App Store Connect, so it needs an App Store profile. Build with --device to install on a registered iPhone or iPad.",
    ];
    const DEBUGGABLE: [&'static str; 2] = ["<false/>", "<true/>"];
    const APP_STORE_KEYS: [&'static str; 2] = ["", "\n<key>beta-reports-active</key><true/>"];
    const ASSOCIATED_DOMAINS: &'static str = "<key>com.apple.developer.associated-domains</key>";

    pub(super) fn from_plist(text: String) -> Self {
        Self { text }
    }

    pub(super) fn name(&self) -> String {
        self.string_after("<key>Name</key>").unwrap_or_default()
    }

    pub(super) fn team(&self) -> String {
        self.string_after("<key>TeamIdentifier</key>")
            .unwrap_or_default()
    }

    pub(super) fn application_identifier(&self) -> String {
        self.string_after("<key>application-identifier</key>")
            .unwrap_or_default()
    }

    pub(super) fn debuggable(&self) -> bool {
        self.text
            .split_once("<key>get-task-allow</key>")
            .is_some_and(|(_, rest)| rest.trim_start().starts_with("<true/>"))
    }

    pub(super) fn is_app_store(&self) -> bool {
        [
            self.text.contains(Self::DEVICE_KEYS[0]),
            self.text.contains(Self::DEVICE_KEYS[1]),
            self.debuggable(),
        ] == [false; 3]
    }

    pub(super) fn certificate_fingerprints(&self) -> Vec<String> {
        let certificates = self
            .text
            .split_once("<key>DeveloperCertificates</key>")
            .and_then(|(_, rest)| rest.split_once("</array>"))
            .map_or("", |(array, _)| array);
        certificates
            .split(Self::DATA_TAGS[0])
            .skip(1)
            .filter_map(|piece| piece.split_once(Self::DATA_TAGS[1]))
            .filter_map(|(data, _)| Self::fingerprint(data))
            .collect()
    }

    pub(super) fn check(
        &self,
        signing: &IosSigning,
        bundle_identifier: &str,
        app_store: bool,
    ) -> Result<(), Failure> {
        let name = self.name();
        let team = self.team();
        Self::require(
            team == signing.team,
            &format!(
                "The provisioning profile '{name}' belongs to team '{team}', but TAC_APPLE_TEAM_ID is '{}'.",
                signing.team
            ),
        )?;
        let wanted = format!("{team}.{bundle_identifier}");
        let pattern = self.application_identifier();
        let covers = pattern
            .strip_suffix('*')
            .map_or(pattern == wanted, |prefix| wanted.starts_with(prefix));
        Self::require(
            covers,
            &format!(
                "The provisioning profile '{name}' is for the app ID '{pattern}', which does not cover '{wanted}'. Make a profile for tac.application.id '{bundle_identifier}' in your Apple Developer account."
            ),
        )?;
        let kind = usize::from(self.is_app_store());
        Self::require(
            self.is_app_store() == app_store,
            &format!(
                "The provisioning profile '{name}' is {} profile. {}",
                Self::KINDS[kind],
                Self::WANTED[usize::from(app_store)]
            ),
        )
    }

    pub(super) fn require_associated_domains(&self, wanted: bool) -> Result<(), Failure> {
        let missing = [wanted, !self.text.contains(Self::ASSOCIATED_DOMAINS)] == [true, true];
        Self::require(
            !missing,
            &format!(
                "tac.application.links lists hosts, but the provisioning profile '{}' has no Associated Domains capability. Turn on Associated Domains for the app ID in your Apple Developer account, then download the profile again.",
                self.name()
            ),
        )
    }

    pub(super) fn entitlements(
        &self,
        team: &str,
        bundle_identifier: &str,
        associated_domains: &str,
    ) -> String {
        let identifier = xml_escape(&format!("{team}.{bundle_identifier}"));
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>application-identifier</key><string>{identifier}</string>
<key>com.apple.developer.team-identifier</key><string>{team}</string>
<key>get-task-allow</key>{debuggable}{app_store}{associated_domains}
</dict></plist>
"#,
            team = xml_escape(team),
            debuggable = Self::DEBUGGABLE[usize::from(self.debuggable())],
            app_store = Self::APP_STORE_KEYS[usize::from(self.is_app_store())],
        )
    }

    fn string_after(&self, key: &str) -> Option<String> {
        let (_, rest) = self.text.split_once(key)?;
        let (_, value) = rest.split_once(Self::STRING_TAGS[0])?;
        let (value, _) = value.split_once(Self::STRING_TAGS[1])?;
        Some(Self::unescape(value.trim()))
    }

    fn unescape(value: &str) -> String {
        value
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    }

    fn fingerprint(data: &str) -> Option<String> {
        let compact: String = data.split_whitespace().collect();
        let certificate = base64::engine::general_purpose::STANDARD
            .decode(compact)
            .ok()?;
        Some(crate::compiler::hex_digest(Sha1::digest(certificate)).to_ascii_uppercase())
    }

    fn require(fits: bool, message: &str) -> Result<(), Failure> {
        fits.then_some(())
            .ok_or_else(|| IosSigning::failure(message))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::IosProvisioningProfile;
    use crate::native::ios_signing::IosSigning;
    use std::path::PathBuf;

    fn profile(application: &str, devices: bool, debuggable: bool) -> String {
        let devices = if devices {
            "\t<key>ProvisionedDevices</key>\n\t<array>\n\t\t<string>00008110-000A1B2C3D4E5F6A</string>\n\t</array>\n"
        } else {
            ""
        };
        let debuggable = if debuggable { "<true/>" } else { "<false/>" };
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n\
             \t<key>AppIDName</key>\n\t<string>Catalog</string>\n\
             \t<key>DeveloperCertificates</key>\n\t<array>\n\t\t<data>Y2VydGlm\n\t\taWNhdGU=</data>\n\t\t<data>c2Vjb25k</data>\n\t</array>\n\
             \t<key>Entitlements</key>\n\t<dict>\n\
             \t\t<key>application-identifier</key>\n\t\t<string>{application}</string>\n\
             \t\t<key>get-task-allow</key>\n\t\t{debuggable}\n\
             \t\t<key>com.apple.developer.team-identifier</key>\n\t\t<string>A1B2C3D4E5</string>\n\
             \t</dict>\n\
             \t<key>Name</key>\n\t<string>Catalog &amp; Co Profile</string>\n\
             {devices}\
             \t<key>TeamIdentifier</key>\n\t<array>\n\t\t<string>A1B2C3D4E5</string>\n\t</array>\n\
             </dict>\n</plist>\n"
        )
    }

    fn signing(team: &str) -> IosSigning {
        IosSigning {
            team: String::from(team),
            profile: PathBuf::from("app.mobileprovision"),
            identity: None,
        }
    }

    #[test]
    fn the_profile_names_its_team_app_id_and_kind() {
        let development = IosProvisioningProfile::from_plist(profile(
            "A1B2C3D4E5.dev.tachyon.catalog",
            true,
            true,
        ));
        assert_eq!(development.name(), "Catalog & Co Profile");
        assert_eq!(development.team(), "A1B2C3D4E5");
        assert_eq!(
            development.application_identifier(),
            "A1B2C3D4E5.dev.tachyon.catalog"
        );
        assert!(development.debuggable());
        assert!(!development.is_app_store());

        let store = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", false, false));
        assert!(!store.debuggable());
        assert!(store.is_app_store());
        let ad_hoc = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", true, false));
        assert!(!ad_hoc.is_app_store());
    }

    #[test]
    fn certificate_fingerprints_are_the_sha1_of_each_certificate() {
        let parsed = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", true, true));
        assert_eq!(
            parsed.certificate_fingerprints(),
            vec![
                String::from("735AD571C189D7BA84464BF4A9F1D2280175B128"),
                String::from("352F7829A2384B001CC12B0C2613C756454A1F6A"),
            ]
        );
    }

    #[test]
    fn a_profile_must_fit_the_team_the_app_id_and_the_build() {
        let development =
            IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.dev.tachyon.*", true, true));
        development
            .check(&signing("A1B2C3D4E5"), "dev.tachyon.catalog", false)
            .expect("development profile for --device");

        let text = development
            .check(&signing("Z9Y8X7W6V5"), "dev.tachyon.catalog", false)
            .expect_err("other team")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("belongs to team 'A1B2C3D4E5'"), "{text}");

        let text = development
            .check(&signing("A1B2C3D4E5"), "com.example.catalog", false)
            .expect_err("other app")
            .to_string();
        assert!(
            text.contains("does not cover 'A1B2C3D4E5.com.example.catalog'"),
            "{text}"
        );

        let text = development
            .check(&signing("A1B2C3D4E5"), "dev.tachyon.catalog", true)
            .expect_err("not for the store")
            .to_string();
        assert!(
            text.contains("is a development, ad hoc or enterprise profile. --release"),
            "{text}"
        );

        let exact = IosProvisioningProfile::from_plist(profile(
            "A1B2C3D4E5.dev.tachyon.catalog",
            false,
            false,
        ));
        exact
            .check(&signing("A1B2C3D4E5"), "dev.tachyon.catalog", true)
            .expect("App Store profile for --release");
        let text = exact
            .check(&signing("A1B2C3D4E5"), "dev.tachyon.catalog.extra", true)
            .expect_err("exact id only")
            .to_string();
        assert!(text.contains("does not cover"), "{text}");
        let text = exact
            .check(&signing("A1B2C3D4E5"), "dev.tachyon.catalog", false)
            .expect_err("store profile on a device")
            .to_string();
        assert!(text.contains("is an App Store profile. --device"), "{text}");
    }

    #[test]
    fn entitlements_name_the_concrete_app_and_follow_the_profile() {
        let development = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", true, true));
        let entitlements = development.entitlements("A1B2C3D4E5", "dev.tachyon.catalog", "");
        assert!(entitlements.contains(
            "<key>application-identifier</key><string>A1B2C3D4E5.dev.tachyon.catalog</string>"
        ));
        assert!(
            entitlements.contains(
                "<key>com.apple.developer.team-identifier</key><string>A1B2C3D4E5</string>"
            )
        );
        assert!(entitlements.contains("<key>get-task-allow</key><true/>"));
        assert!(!entitlements.contains("beta-reports-active"));

        let store = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", false, false));
        let domains = "\n<key>com.apple.developer.associated-domains</key><array/>";
        let entitlements = store.entitlements("A1B2C3D4E5", "dev.tachyon.catalog", domains);
        assert!(entitlements.contains("<key>get-task-allow</key><false/>"));
        assert!(entitlements.contains(&format!(
            "<key>beta-reports-active</key><true/>{domains}\n</dict>"
        )));
    }

    #[test]
    fn listed_hosts_need_a_profile_with_associated_domains() {
        let without = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", true, true));
        without
            .require_associated_domains(false)
            .expect("no hosts listed");
        let text = without
            .require_associated_domains(true)
            .expect_err("no capability")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("no Associated Domains capability"), "{text}");

        let with = IosProvisioningProfile::from_plist(profile("A1B2C3D4E5.*", true, true).replace(
            "<key>Entitlements</key>\n\t<dict>\n",
            "<key>Entitlements</key>\n\t<dict>\n\t\t<key>com.apple.developer.associated-domains</key>\n\t\t<string>*</string>\n",
        ));
        with.require_associated_domains(true)
            .expect("capability on");
    }
}

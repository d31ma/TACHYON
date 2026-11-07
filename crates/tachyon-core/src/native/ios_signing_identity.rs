use super::ios_provisioning_profile::IosProvisioningProfile;
use super::ios_signing::IosSigning;
use crate::Failure;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct IosSigningIdentity {
    pub(super) fingerprint: String,
    pub(super) name: String,
}

impl IosSigningIdentity {
    const FINGERPRINT_LENGTH: usize = 40;

    pub(super) fn listed(output: &str) -> Vec<Self> {
        output.lines().filter_map(Self::parse_line).collect()
    }

    pub(super) fn choose(
        output: &str,
        profile: &IosProvisioningProfile,
        chosen: Option<&str>,
    ) -> Result<String, Failure> {
        chosen.map_or_else(
            || Self::matching(output, profile),
            |name| Ok(name.to_owned()),
        )
    }

    fn matching(output: &str, profile: &IosProvisioningProfile) -> Result<String, Failure> {
        let fingerprints = profile.certificate_fingerprints();
        let mut found: Vec<Self> = Self::listed(output)
            .into_iter()
            .filter(|identity| fingerprints.contains(&identity.fingerprint))
            .collect();
        found.sort();
        found.dedup();
        match found.as_slice() {
            [identity] => Ok(identity.fingerprint.clone()),
            [] => Err(Self::none_found(profile)),
            several => Err(Self::several_found(profile, several)),
        }
    }

    fn parse_line(line: &str) -> Option<Self> {
        let (_, rest) = line.trim().split_once(") ")?;
        let (fingerprint, name) = rest.split_once(' ')?;
        let valid = [
            fingerprint.len() == Self::FINGERPRINT_LENGTH,
            fingerprint
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
        ] == [true, true];
        valid.then(|| Self {
            fingerprint: fingerprint.to_ascii_uppercase(),
            name: name.trim().trim_matches('"').to_owned(),
        })
    }

    fn none_found(profile: &IosProvisioningProfile) -> Failure {
        IosSigning::failure(&format!(
            "No valid signing certificate in the keychain is one that the provisioning profile '{}' names. Install that certificate with its private key (Xcode > Settings > Accounts > Manage Certificates), or download a profile that names a certificate you have.",
            profile.name()
        ))
    }

    fn several_found(profile: &IosProvisioningProfile, several: &[Self]) -> Failure {
        let names: Vec<String> = several
            .iter()
            .map(|identity| format!("{} ({})", identity.name, identity.fingerprint))
            .collect();
        IosSigning::failure(&format!(
            "The provisioning profile '{}' names more than one certificate in the keychain: {}. Set TAC_IOS_SIGNING_IDENTITY to the SHA-1 of the one to use.",
            profile.name(),
            names.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::IosSigningIdentity;
    use crate::native::ios_provisioning_profile::IosProvisioningProfile;

    const FIRST: &str = "735AD571C189D7BA84464BF4A9F1D2280175B128";
    const SECOND: &str = "352F7829A2384B001CC12B0C2613C756454A1F6A";

    fn profile(certificates: &[&str]) -> IosProvisioningProfile {
        let data = certificates.join("</data><data>");
        IosProvisioningProfile::from_plist(format!(
            "<dict><key>DeveloperCertificates</key><array><data>{data}</data></array>\
             <key>Name</key><string>Catalog Profile</string></dict>"
        ))
    }

    fn listing(lines: &[(&str, &str)]) -> String {
        let rows: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(index, (fingerprint, name))| {
                format!("  {}) {fingerprint} \"{name}\"", index + 1)
            })
            .collect();
        format!(
            "  Policy: Code Signing\n  Valid identities only\n{}\n     {} valid identities found\n",
            rows.join("\n"),
            lines.len()
        )
    }

    #[test]
    fn the_keychain_listing_gives_each_identity_and_its_sha1() {
        let output = listing(&[
            (FIRST, "Apple Development: Ada Lovelace (QWERTY1234)"),
            ("not-a-fingerprint", "Broken"),
        ]);
        assert_eq!(
            IosSigningIdentity::listed(&output),
            vec![IosSigningIdentity {
                fingerprint: String::from(FIRST),
                name: String::from("Apple Development: Ada Lovelace (QWERTY1234)"),
            }]
        );
    }

    #[test]
    fn the_identity_is_the_keychain_certificate_the_profile_names() {
        let output = listing(&[
            (SECOND, "Apple Distribution: Example (A1B2C3D4E5)"),
            (FIRST, "Apple Development: Ada Lovelace (QWERTY1234)"),
            (SECOND, "Apple Distribution: Example (A1B2C3D4E5)"),
        ]);
        let only_second = profile(&["c2Vjb25k"]);
        assert_eq!(
            IosSigningIdentity::choose(&output, &only_second, None).expect("one"),
            SECOND
        );
        assert_eq!(
            IosSigningIdentity::choose(&output, &only_second, Some("Apple Distribution"))
                .expect("chosen"),
            "Apple Distribution"
        );

        let neither = profile(&["b3RoZXI="]);
        let text = IosSigningIdentity::choose(&output, &neither, None)
            .expect_err("none")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("'Catalog Profile' names"), "{text}");

        let both = profile(&["Y2VydGlmaWNhdGU=", "c2Vjb25k"]);
        let text = IosSigningIdentity::choose(&output, &both, None)
            .expect_err("several")
            .to_string();
        assert!(
            text.contains(&format!("Ada Lovelace (QWERTY1234) ({FIRST})")),
            "{text}"
        );
        assert!(text.contains("TAC_IOS_SIGNING_IDENTITY"), "{text}");
    }
}

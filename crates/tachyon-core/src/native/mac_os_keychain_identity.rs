use super::mac_os_signing::MacOsSigning;
use crate::Failure;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MacOsKeychainIdentity;

impl MacOsKeychainIdentity {
    pub(super) const FIND_ARGUMENTS: [&'static str; 4] =
        ["find-identity", "-v", "-p", "codesigning"];
    const DEVELOPER_ID: &'static str = "Developer ID Application: ";

    pub(super) fn developer_id(listing: &str, team: &str) -> Result<String, Failure> {
        let found = Self::developer_ids(listing, team);
        Self::require_one(&found, team)?;
        Ok(found.into_keys().next().unwrap_or_default())
    }

    fn developer_ids(listing: &str, team: &str) -> BTreeMap<String, String> {
        let team_suffix = format!("({team})");
        listing
            .lines()
            .filter_map(Self::hash_and_name)
            .filter(|(_, name)| {
                [
                    name.starts_with(Self::DEVELOPER_ID),
                    name.ends_with(&team_suffix),
                ] == [true, true]
            })
            .collect()
    }

    fn hash_and_name(line: &str) -> Option<(String, String)> {
        let (_, rest) = line.trim().split_once(") ")?;
        let (hash, quoted) = rest.split_once(' ')?;
        let name = quoted.trim().strip_prefix('"')?.strip_suffix('"')?;
        Some((hash.to_owned(), name.to_owned()))
    }

    fn require_one(found: &BTreeMap<String, String>, team: &str) -> Result<(), Failure> {
        let names: Vec<String> = found
            .iter()
            .map(|(hash, name)| format!("{hash} \"{name}\""))
            .collect();
        let messages = [
            format!(
                "The keychain holds no valid Developer ID Application certificate for team {team}."
            ),
            String::new(),
            format!(
                "The keychain holds several Developer ID Application certificates for team {team}: \
                 {}. Set TAC_MACOS_SIGNING_IDENTITY to the one to sign with.",
                names.join(", ")
            ),
        ];
        let message = &messages[found.len().min(2)];
        message
            .is_empty()
            .then_some(())
            .ok_or_else(|| MacOsSigning::failure(message))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::MacOsKeychainIdentity;

    fn listing() -> &'static str {
        r#"  1) 1111111111111111111111111111111111111111 "Apple Development: Example (Z9Y8X7W6V5)"
  2) 2222222222222222222222222222222222222222 "Developer ID Application: Example Ltd (A1B2C3D4E5)"
  3) 2222222222222222222222222222222222222222 "Developer ID Application: Example Ltd (A1B2C3D4E5)"
  4) 3333333333333333333333333333333333333333 "Developer ID Application: Other (Q1W2E3R4T5)"
     4 valid identities found
"#
    }

    #[test]
    fn the_developer_id_of_the_team_is_chosen_by_its_hash() {
        assert_eq!(
            MacOsKeychainIdentity::developer_id(listing(), "A1B2C3D4E5").expect("identity"),
            "2222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn a_team_without_a_developer_id_certificate_is_refused() {
        let text = MacOsKeychainIdentity::developer_id(listing(), "Z9Y8X7W6V5")
            .expect_err("only a development certificate")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("no valid Developer ID Application certificate for team Z9Y8X7W6V5"));
    }

    #[test]
    fn several_developer_id_certificates_ask_for_one_by_name() {
        let crowded = format!(
            "{}  5) 4444444444444444444444444444444444444444 \"Developer ID Application: Example Ltd (A1B2C3D4E5)\"\n",
            listing()
        );
        let text = MacOsKeychainIdentity::developer_id(&crowded, "A1B2C3D4E5")
            .expect_err("two certificates")
            .to_string();
        assert!(text.contains("several"), "{text}");
        assert!(
            text.contains("4444444444444444444444444444444444444444"),
            "{text}"
        );
        assert!(text.contains("TAC_MACOS_SIGNING_IDENTITY"), "{text}");
    }
}

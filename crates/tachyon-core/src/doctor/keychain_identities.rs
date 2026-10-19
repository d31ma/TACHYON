use super::ToolchainState;
use super::command_probe::CommandProbe;

pub(super) struct KeychainIdentities;

impl KeychainIdentities {
    const PROGRAM: &'static str = "security";
    const ARGUMENTS: [&'static str; 4] = ["find-identity", "-v", "-p", "codesigning"];

    pub(super) fn state(kinds: &[&str]) -> ToolchainState {
        CommandProbe::output(Self::PROGRAM, &Self::ARGUMENTS)
            .map_or(ToolchainState::Missing, |listing| {
                Self::listing_state(&listing, kinds)
            })
    }

    fn listing_state(listing: &str, kinds: &[&str]) -> ToolchainState {
        Self::counted_state(Self::count(listing, kinds), kinds)
    }

    fn counted_state(count: usize, kinds: &[&str]) -> ToolchainState {
        if count == 0 {
            return ToolchainState::Incomplete(format!(
                "the keychain holds no valid {} certificate",
                kinds.join(" or ")
            ));
        }
        ToolchainState::Ready(format!("{count} valid in the keychain"))
    }

    fn count(listing: &str, kinds: &[&str]) -> usize {
        listing
            .lines()
            .filter_map(|line| line.split_once('"'))
            .filter(|(_, name)| kinds.iter().any(|kind| name.starts_with(kind)))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::KeychainIdentities;
    use crate::doctor::ToolchainState;

    fn listing() -> &'static str {
        r#"  1) 1111111111111111111111111111111111111111 "Apple Development: Example (Z9Y8X7W6V5)"
  2) 2222222222222222222222222222222222222222 "Developer ID Application: Example Ltd (A1B2C3D4E5)"
  3) 3333333333333333333333333333333333333333 "Apple Distribution: Example Ltd (A1B2C3D4E5)"
  4) 4444444444444444444444444444444444444444 "iPhone Distribution: Example Ltd (A1B2C3D4E5)"
     4 valid identities found
"#
    }

    #[test]
    fn only_identities_of_the_asked_kinds_are_counted() {
        assert_eq!(
            KeychainIdentities::listing_state(listing(), &["Developer ID Application"]),
            ToolchainState::Ready(String::from("1 valid in the keychain"))
        );
        assert_eq!(
            KeychainIdentities::listing_state(
                listing(),
                &["Apple Distribution", "iPhone Distribution"]
            ),
            ToolchainState::Ready(String::from("2 valid in the keychain"))
        );
    }

    #[test]
    fn a_keychain_without_the_kind_names_what_is_missing() {
        assert_eq!(
            KeychainIdentities::listing_state(
                "     0 valid identities found\n",
                &["Developer ID Application"]
            ),
            ToolchainState::Incomplete(String::from(
                "the keychain holds no valid Developer ID Application certificate"
            ))
        );
    }
}

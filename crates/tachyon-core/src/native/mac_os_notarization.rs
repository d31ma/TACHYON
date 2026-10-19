use super::mac_os_signing::MacOsSigning;
use crate::Failure;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MacOsNotarization;

impl MacOsNotarization {
    const ACCEPTED: &'static str = "Accepted";
    const WAIT_LIMIT: &'static str = "14m";

    pub(super) fn submit_arguments<'path>(dmg: &'path str, profile: &'path str) -> Vec<&'path str> {
        vec![
            "notarytool",
            "submit",
            dmg,
            "--keychain-profile",
            profile,
            "--wait",
            "--timeout",
            Self::WAIT_LIMIT,
            "--output-format",
            "json",
        ]
    }

    pub(super) fn staple_arguments(dmg: &str) -> [&str; 3] {
        ["stapler", "staple", dmg]
    }

    pub(super) fn validate_arguments(dmg: &str) -> [&str; 3] {
        ["stapler", "validate", dmg]
    }

    pub(super) fn gatekeeper_arguments(dmg: &str) -> [&str; 6] {
        [
            "--assess",
            "--type",
            "open",
            "--context",
            "context:primary-signature",
            dmg,
        ]
    }

    pub(super) fn accepted(output: &str, profile: &str) -> Result<(), Failure> {
        let answer: serde_json::Value = serde_json::from_str(output).unwrap_or_default();
        let field = |name: &str| answer[name].as_str().unwrap_or_default().to_owned();
        let status = field("status");
        (status == Self::ACCEPTED).then_some(()).ok_or_else(|| {
            MacOsSigning::failure(&format!(
                "Apple did not notarize the macOS release: status '{status}' for submission \
                 '{id}'. {message} Read the reasons with: xcrun notarytool log {id} \
                 --keychain-profile {profile}",
                id = field("id"),
                message = field("message"),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::MacOsNotarization;

    #[test]
    fn notarytool_waits_for_the_answer_from_a_keychain_profile() {
        assert_eq!(
            MacOsNotarization::submit_arguments("dist/macos/Notes.dmg", "tachyon-notary"),
            [
                "notarytool",
                "submit",
                "dist/macos/Notes.dmg",
                "--keychain-profile",
                "tachyon-notary",
                "--wait",
                "--timeout",
                "14m",
                "--output-format",
                "json"
            ]
        );
    }

    #[test]
    fn an_accepted_submission_passes() {
        let output = r#"{"id":"2efe2717-52ef-43a5-96dc-0797e4ca1041","message":"Processing complete","status":"Accepted"}"#;
        assert!(MacOsNotarization::accepted(output, "tachyon-notary").is_ok());
    }

    #[test]
    fn an_invalid_submission_names_the_log_command() {
        let output = r#"{"id":"2efe2717-52ef-43a5-96dc-0797e4ca1041","message":"Processing complete","status":"Invalid"}"#;
        let text = MacOsNotarization::accepted(output, "tachyon-notary")
            .expect_err("invalid")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("status 'Invalid'"), "{text}");
        assert!(
            text.contains(
                "xcrun notarytool log 2efe2717-52ef-43a5-96dc-0797e4ca1041 --keychain-profile tachyon-notary"
            ),
            "{text}"
        );
        assert!(
            MacOsNotarization::accepted("not json", "tachyon-notary").is_err(),
            "unreadable output is not an acceptance"
        );
    }
}

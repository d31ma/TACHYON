use crate::Failure;
use crate::failure::diagnostic;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct AndroidSigningCertificate {
    fingerprints: Vec<String>,
}

impl AndroidSigningCertificate {
    pub(super) const VARIABLE: &'static str = "TAC_ANDROID_CERT_SHA256";
    const SEPARATOR: char = ',';
    const PAIR_SEPARATOR: char = ':';
    const PAIRS: usize = 32;
    const PROBLEM: &'static str = "TAC_ANDROID_CERT_SHA256 must list the SHA-256 fingerprints of your Android signing certificates, separated by commas: 32 hexadecimal pairs joined by ':', such as 14:6D:E9:...:5F. `keytool -list -v -keystore <keystore>` prints them.";
    const HELP: &'static str = "Correct TAC_ANDROID_CERT_SHA256 in the environment that runs ty, or unset it. It never goes in client/manifest.json.";

    pub(super) fn from_environment() -> Result<Self, Failure> {
        Self::parse(std::env::var(Self::VARIABLE).ok().as_deref())
    }

    pub(super) fn parse(value: Option<&str>) -> Result<Self, Failure> {
        let fingerprints: Vec<String> = value
            .unwrap_or_default()
            .split(Self::SEPARATOR)
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(str::to_ascii_uppercase)
            .collect();
        fingerprints
            .iter()
            .map(String::as_str)
            .all(Self::is_fingerprint)
            .then_some(Self { fingerprints })
            .ok_or_else(Self::refusal)
    }

    pub(super) fn fingerprints(&self) -> &[String] {
        &self.fingerprints
    }

    fn is_fingerprint(text: &str) -> bool {
        let pairs: Vec<&str> = text.split(Self::PAIR_SEPARATOR).collect();
        let counted = pairs.len() == Self::PAIRS;
        let hexadecimal = pairs.iter().all(|pair| {
            [
                pair.len() == 2,
                pair.bytes().all(|byte| byte.is_ascii_hexdigit()),
            ] == [true, true]
        });
        [counted, hexadecimal] == [true, true]
    }

    fn refusal() -> Failure {
        Failure::one(diagnostic(
            1601,
            Self::PROBLEM,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AndroidSigningCertificate;

    fn fingerprint(pair: &str) -> String {
        vec![pair; 32].join(":")
    }

    #[test]
    fn fingerprints_are_read_uppercase_from_a_comma_list() {
        let value = format!("{}, {}", fingerprint("ab"), fingerprint("0F"));
        let certificate = AndroidSigningCertificate::parse(Some(&value)).expect("certificate");
        assert_eq!(
            certificate.fingerprints(),
            [fingerprint("AB"), fingerprint("0F")]
        );
        let unset = AndroidSigningCertificate::parse(None).expect("unset");
        assert!(unset.fingerprints().is_empty());
    }

    #[test]
    fn malformed_fingerprints_are_refused() {
        let short = vec!["AB"; 31].join(":");
        let wrong = format!("{}:ZZ", vec!["AB"; 31].join(":"));
        for value in [short.as_str(), wrong.as_str(), "ABCDEF"] {
            assert!(
                AndroidSigningCertificate::parse(Some(value)).is_err(),
                "{value}"
            );
        }
    }
}

use super::windows_package_manifest::WindowsPackageManifest;
use super::windows_release::WindowsRelease;
use crate::Failure;
use crate::failure::diagnostic;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WindowsPackageFamily {
    name: String,
}

impl WindowsPackageFamily {
    const ALPHABET: &'static [u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
    const PUBLISHER_ID_CHARACTERS: u32 = 13;
    const BITS_PER_CHARACTER: u32 = 5;
    const PROBLEM: &'static str = "TAC_WINDOWS_CERTIFICATE_PUBLISHER must be the subject of the certificate that signs the MSIX package, with a CN= part, such as CN=Contoso Ltd, O=Contoso Ltd, C=US.";
    const HELP: &'static str = "Correct TAC_WINDOWS_CERTIFICATE_PUBLISHER in the environment that runs ty, or unset it. It never goes in client/manifest.json.";

    pub(crate) fn from_environment(application_id: &str) -> Result<Option<Self>, Failure> {
        let publisher = std::env::var(WindowsRelease::CERTIFICATE_PUBLISHER).ok();
        Self::parse(application_id, publisher.as_deref())
    }

    pub(crate) fn parse(
        application_id: &str,
        publisher: Option<&str>,
    ) -> Result<Option<Self>, Failure> {
        publisher
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Self::named(application_id, text))
            .transpose()
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn publisher_id(publisher: &str) -> String {
        let bytes: Vec<u8> = publisher
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let digest = Sha256::digest(bytes);
        let mut first = [0_u8; 8];
        first.copy_from_slice(&digest[..8]);
        let bits = u128::from(u64::from_be_bytes(first)) << 1;
        (0..Self::PUBLISHER_ID_CHARACTERS)
            .rev()
            .map(|place| {
                let index = (bits >> (place * Self::BITS_PER_CHARACTER)) & 0x1f;
                char::from(Self::ALPHABET[usize::try_from(index).unwrap_or_default()])
            })
            .collect()
    }

    fn named(application_id: &str, publisher: &str) -> Result<Self, Failure> {
        publisher.contains("CN=").then_some(()).ok_or_else(|| {
            Failure::one(diagnostic(
                1601,
                Self::PROBLEM,
                Some(String::from(Self::HELP)),
                None,
            ))
        })?;
        let identity = WindowsPackageManifest::identity_name(application_id)?;
        Ok(Self {
            name: format!("{identity}_{}", Self::publisher_id(publisher)),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::WindowsPackageFamily;

    #[test]
    fn the_publisher_id_is_the_one_windows_derives_from_the_certificate_subject() {
        assert_eq!(
            WindowsPackageFamily::publisher_id(
                "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US"
            ),
            "8wekyb3d8bbwe"
        );
        assert_eq!(
            WindowsPackageFamily::publisher_id("CN=Contoso Ltd, O=Contoso Ltd, C=US"),
            "t247dhg0dta00"
        );
    }

    #[test]
    fn the_family_name_joins_the_package_name_and_the_publisher_id() {
        let family = WindowsPackageFamily::parse(
            "ma.del.notes_app",
            Some(" CN=Contoso Ltd, O=Contoso Ltd, C=US "),
        )
        .expect("publisher")
        .expect("family");
        assert_eq!(family.name(), "ma.del.notes-app_t247dhg0dta00");
        assert_eq!(
            WindowsPackageFamily::parse("ma.del.notes", Some(" ")).expect("unset"),
            None
        );
        assert_eq!(
            WindowsPackageFamily::parse("ma.del.notes", None).expect("unset"),
            None
        );
    }

    #[test]
    fn a_publisher_without_a_common_name_stops_the_build() {
        let text = WindowsPackageFamily::parse("ma.del.notes", Some("O=Contoso"))
            .expect_err("refused")
            .to_string();
        assert!(text.contains("TY1601"), "{text}");
        assert!(text.contains("CN= part"), "{text}");
    }
}

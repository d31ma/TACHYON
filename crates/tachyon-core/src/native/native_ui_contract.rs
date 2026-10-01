use super::native_ui_problem::NativeUiProblem;
use crate::Failure;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NativeUiContract {
    application_id: String,
    target: String,
    native_sha256: String,
}

impl NativeUiContract {
    pub fn macos(application_id: String, native_sha256: String) -> Result<Self, Failure> {
        let contract = Self {
            application_id,
            target: "macos".to_owned(),
            native_sha256,
        };
        contract.validate()?;
        Ok(contract)
    }

    pub(super) fn validate(&self) -> Result<(), Failure> {
        Self::application(&self.application_id)?;
        Self::platform(&self.target)?;
        Self::digest(&self.native_sha256)
    }

    fn application(value: &str) -> Result<(), Failure> {
        NativeUiProblem::require(
            (1..=256).contains(&value.len()),
            "Native UI application identity must be bounded ASCII.",
        )?;
        NativeUiProblem::require(
            Self::application_letters(value),
            "Native UI application identity must be bounded ASCII.",
        )
    }

    fn application_letters(value: &str) -> bool {
        value.bytes().all(Self::application_letter)
    }

    fn application_letter(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), b".-_".contains(&byte)].contains(&true)
    }

    fn platform(value: &str) -> Result<(), Failure> {
        if value != "macos" {
            return Err(NativeUiProblem::failure(
                "Native UI generation validation currently supports macOS only.",
            ));
        }
        Ok(())
    }

    pub(super) fn digest(value: &str) -> Result<(), Failure> {
        NativeUiProblem::require(
            value.len() == 64,
            "Native UI fingerprints must be lowercase SHA-256 digests.",
        )?;
        NativeUiProblem::require(
            Self::digest_letters(value),
            "Native UI fingerprints must be lowercase SHA-256 digests.",
        )
    }

    fn digest_letters(value: &str) -> bool {
        value.bytes().all(Self::digest_letter)
    }

    fn digest_letter(byte: u8) -> bool {
        [byte.is_ascii_digit(), (b'a'..=b'f').contains(&byte)].contains(&true)
    }
}

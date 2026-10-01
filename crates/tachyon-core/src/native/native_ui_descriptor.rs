use super::{
    native_ui_asset::NativeUiAsset, native_ui_contract::NativeUiContract,
    native_ui_problem::NativeUiProblem,
};
use crate::Failure;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _, OpenOptionsSyncExt as _};
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write as _;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct NativeUiDescriptor {
    pub(super) schema_version: u8,
    pub(super) contract: NativeUiContract,
    pub(super) assets: Vec<NativeUiAsset>,
    pub(super) generation: String,
}

impl NativeUiDescriptor {
    pub(super) const MAX_BYTES: usize = 1024 * 1024;

    pub(super) fn from_assets(
        contract: NativeUiContract,
        assets: Vec<NativeUiAsset>,
    ) -> Result<Self, Failure> {
        contract.validate()?;
        let generation = Self::identity(&contract, &assets)?;
        let descriptor = Self {
            schema_version: 1,
            contract,
            assets,
            generation,
        };
        descriptor.bytes()?;
        Ok(descriptor)
    }

    pub(super) fn trusted(source: &[u8], expected_sha256: &str) -> Result<Self, Failure> {
        Self::bounded(source)?;
        Self::fingerprint(source, expected_sha256)?;
        let descriptor: Self = serde_json::from_slice(source).map_err(|error| {
            NativeUiProblem::failure(&format!("Invalid native UI descriptor: {error}"))
        })?;
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub(super) fn emit(&self, output: &Dir) -> Result<String, Failure> {
        let source = self.bytes()?;
        let name = format!("native-ui-{}.json", Self::sha256(&source));
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No)
            .nonblock(true);
        let mut file = output
            .open_with(&name, &options)
            .map_err(|error| NativeUiProblem::io(&error))?;
        let result = Self::write(&mut file, &source);
        Self::publication(output, &name, result)?;
        Ok(name)
    }

    fn write(file: &mut cap_std::fs::File, source: &[u8]) -> Result<(), Failure> {
        file.write_all(source)
            .map_err(|error| NativeUiProblem::io(&error))?;
        file.sync_all().map_err(|error| NativeUiProblem::io(&error))
    }

    fn publication(output: &Dir, name: &str, result: Result<(), Failure>) -> Result<(), Failure> {
        if let Err(error) = result {
            output.remove_file(name).map_err(|cleanup| {
                NativeUiProblem::failure(&format!(
                    "{error}; cannot remove failed native descriptor: {cleanup}"
                ))
            })?;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn bytes(&self) -> Result<Vec<u8>, Failure> {
        let bytes = serde_json::to_vec(self).map_err(|error| {
            NativeUiProblem::failure(&format!("Cannot serialize native UI descriptor: {error}"))
        })?;
        Self::bounded(&bytes)?;
        Ok(bytes)
    }

    pub(super) fn sha256(bytes: &[u8]) -> String {
        crate::compiler::hex_digest(Sha256::digest(bytes))
    }

    fn identity(contract: &NativeUiContract, assets: &[NativeUiAsset]) -> Result<String, Failure> {
        let bytes = serde_json::to_vec(&(contract, assets)).map_err(|error| {
            NativeUiProblem::failure(&format!("Cannot identify native UI generation: {error}"))
        })?;
        Ok(Self::sha256(&bytes))
    }

    fn validate(&self) -> Result<(), Failure> {
        self.contract.validate()?;
        Self::schema(self.schema_version)?;
        NativeUiContract::digest(&self.generation)
    }

    fn schema(version: u8) -> Result<(), Failure> {
        if version != 1 {
            return Err(NativeUiProblem::failure(
                "Unknown native UI descriptor schema.",
            ));
        }
        Ok(())
    }

    fn bounded(bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() > Self::MAX_BYTES {
            return Err(NativeUiProblem::failure(
                "Native UI descriptor exceeds 1 MiB.",
            ));
        }
        Ok(())
    }

    fn fingerprint(source: &[u8], expected: &str) -> Result<(), Failure> {
        NativeUiContract::digest(expected)?;
        if Self::sha256(source) != expected {
            return Err(NativeUiProblem::failure(
                "Native UI descriptor does not match the externally trusted fingerprint.",
            ));
        }
        Ok(())
    }
}

use super::{
    native_ui_capture::NativeUiCapture, native_ui_change::NativeUiChange,
    native_ui_contract::NativeUiContract, native_ui_descriptor::NativeUiDescriptor,
    native_ui_problem::NativeUiProblem,
};
use crate::Failure;
use cap_std::fs::Dir;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct NativeUiGeneration {
    descriptor: NativeUiDescriptor,
    assets: BTreeMap<String, Vec<u8>>,
}

impl NativeUiGeneration {
    pub fn capture(root: &Dir, contract: NativeUiContract) -> Result<Self, Failure> {
        let capture = NativeUiCapture::read(root)?;
        let descriptor = NativeUiDescriptor::from_assets(contract, capture.assets)?;
        Ok(Self {
            descriptor,
            assets: capture.bytes,
        })
    }

    pub fn trusted(root: &Dir, source: &[u8], expected_sha256: &str) -> Result<Self, Failure> {
        let descriptor = NativeUiDescriptor::trusted(source, expected_sha256)?;
        let captured = Self::capture(root, descriptor.contract.clone())?;
        captured.matches(&descriptor)?;
        Ok(captured)
    }

    pub fn descriptor_bytes(&self) -> Result<Vec<u8>, Failure> {
        self.descriptor.bytes()
    }

    pub fn descriptor_sha256(&self) -> Result<String, Failure> {
        self.descriptor_bytes()
            .map(|bytes| NativeUiDescriptor::sha256(&bytes))
    }

    #[must_use]
    pub fn asset(&self, path: &str) -> Option<&[u8]> {
        self.assets.get(path).map(Vec::as_slice)
    }

    #[must_use]
    pub fn compare(&self, installed: &Self) -> NativeUiChange {
        let unchanged = self.unchanged(installed);
        unchanged.unwrap_or_else(|| self.changed(installed))
    }

    fn unchanged(&self, installed: &Self) -> Option<NativeUiChange> {
        if self.descriptor == installed.descriptor {
            return Some(NativeUiChange::Unchanged);
        }
        None
    }

    fn changed(&self, installed: &Self) -> NativeUiChange {
        if self.descriptor.contract != installed.descriptor.contract
            || self.fixed_assets() != installed.fixed_assets()
        {
            return NativeUiChange::RestartRequired;
        }
        NativeUiChange::PrepareStyles
    }

    fn fixed_assets(&self) -> Vec<&super::native_ui_asset::NativeUiAsset> {
        self.descriptor
            .assets
            .iter()
            .filter(|asset| !asset.is_stylesheet())
            .collect()
    }

    fn matches(&self, descriptor: &NativeUiDescriptor) -> Result<(), Failure> {
        if &self.descriptor != descriptor {
            return Err(NativeUiProblem::failure(
                "Native UI generation does not match its complete trusted descriptor.",
            ));
        }
        Ok(())
    }
}

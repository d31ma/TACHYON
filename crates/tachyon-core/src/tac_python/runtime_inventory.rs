use super::runtime_assets::RuntimeAssets;
use crate::Failure;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct RuntimeInventory;

impl RuntimeInventory {
    pub(crate) const ENGINE: &'static str = ".tachyon/services/tachyon-python-services.js";

    pub(crate) fn reachable(
        loaded: &BTreeSet<String>,
        files: &BTreeMap<String, &[u8]>,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<Vec<(String, u64)>, Failure> {
        Self::runtime_files(loaded, files, sent_bytes)
    }

    pub(crate) fn profile_missing() -> Failure {
        RuntimeAssets::failure(
            "client/manifest.json",
            "a reachable Python service requires the explicit pinned offline profile",
        )
    }

    fn runtime_files(
        loaded: &BTreeSet<String>,
        files: &BTreeMap<String, &[u8]>,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<Vec<(String, u64)>, Failure> {
        if !loaded.contains(Self::ENGINE) {
            return Ok(Vec::new());
        }
        RuntimeAssets::names()
            .iter()
            .map(|name| Self::entry(name, files, sent_bytes))
            .collect()
    }

    fn entry(
        name: &str,
        files: &BTreeMap<String, &[u8]>,
        sent_bytes: &dyn Fn(&str) -> Option<u64>,
    ) -> Result<(String, u64), Failure> {
        let bytes = files.get(name).copied().ok_or_else(|| {
            RuntimeAssets::failure(name, "reachable runtime asset was not emitted")
        })?;
        RuntimeAssets::validate(name, bytes)?;
        Ok((
            name.to_owned(),
            sent_bytes(name).unwrap_or(bytes.len() as u64),
        ))
    }
}

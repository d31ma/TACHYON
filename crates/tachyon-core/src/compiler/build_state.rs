use super::route_build_state::RouteBuildState;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BuildState {
    version: u8,
    pub(super) routes: BTreeMap<String, RouteBuildState>,
}

impl BuildState {
    const VERSION: u8 = 5;
    const FILE: &'static str = ".tachyon/build-state.json";

    pub(super) fn load(output: &Path) -> Option<Self> {
        let bytes = Self::read_regular_file(&output.join(Self::FILE))?;
        let state: Self = serde_json::from_slice(&bytes).ok()?;
        Self::current(state)
    }

    pub(super) fn read_regular_file(path: &Path) -> Option<Vec<u8>> {
        Self::is_regular_file(path)
            .then(|| fs::read(path).ok())
            .flatten()
    }

    fn current(state: Self) -> Option<Self> {
        (state.version == Self::VERSION).then_some(state)
    }

    fn is_regular_file(path: &Path) -> bool {
        fs::symlink_metadata(path)
            .is_ok_and(|metadata| !metadata.file_type().is_symlink() && metadata.is_file())
    }
}

impl Default for BuildState {
    fn default() -> Self {
        Self {
            version: Self::VERSION,
            routes: BTreeMap::new(),
        }
    }
}

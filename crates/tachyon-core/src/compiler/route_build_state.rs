use super::build_state::BuildState;
use super::content_digest::ContentDigest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RouteBuildState {
    pub(super) input_sha: String,
    pub(super) artifacts: BTreeMap<String, String>,
    pub(super) islands: BTreeSet<String>,
}

impl RouteBuildState {
    pub(super) fn artifacts_are_intact(&self, output: &Path) -> bool {
        self.all_artifacts_match(output)
    }

    fn all_artifacts_match(&self, output: &Path) -> bool {
        if self.artifacts.is_empty() {
            return false;
        }
        self.artifacts.iter().all(|(relative, expected)| {
            Self::artifact_matches(output, Path::new(relative), expected)
        })
    }

    fn artifact_matches(output: &Path, relative: &Path, expected: &str) -> bool {
        Self::safe_artifact_bytes(output, relative)
            .is_some_and(|bytes| ContentDigest::sha256_bytes(&bytes) == expected)
    }

    fn safe_artifact_bytes(output: &Path, relative: &Path) -> Option<Vec<u8>> {
        Self::is_safe_artifact_path(relative)
            .then(|| BuildState::read_regular_file(&output.join(relative)))
            .flatten()
    }

    fn is_safe_artifact_path(path: &Path) -> bool {
        let checks = [
            !path.as_os_str().is_empty(),
            !path.is_absolute(),
            path.components().all(Self::is_normal),
        ];
        checks.iter().all(|passed| *passed)
    }

    fn is_normal(component: Component<'_>) -> bool {
        matches!(component, Component::Normal(_))
    }
}

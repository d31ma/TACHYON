use crate::Failure;
use crate::template::ServiceRegistry;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) struct BrowserServicePeers {
    selected: BTreeMap<String, PathBuf>,
}

impl BrowserServicePeers {
    pub(super) fn read(root: &Path, candidates: &BTreeSet<String>) -> Result<Self, Failure> {
        Self::capture(root, candidates)
    }

    pub(super) fn matches(&self, name: &str, path: &str) -> bool {
        self.selected_path(name, path)
    }

    fn selected_path(&self, name: &str, path: &str) -> bool {
        self.selected
            .get(name)
            .is_some_and(|selected| selected == Path::new(path))
    }

    fn capture(root: &Path, candidates: &BTreeSet<String>) -> Result<Self, Failure> {
        let registry = candidates
            .first()
            .map(|_| ServiceRegistry::discover(root))
            .transpose()?;
        let selected = registry
            .into_iter()
            .flat_map(|registry| Self::scripts(root, &registry))
            .collect();
        Ok(Self { selected })
    }

    fn scripts(root: &Path, registry: &ServiceRegistry) -> Vec<(String, PathBuf)> {
        registry
            .iter()
            .filter_map(|(_, service)| Self::script(root, service.class(), service.script_path()))
            .collect()
    }

    fn script(root: &Path, name: &str, path: Option<&Path>) -> Option<(String, PathBuf)> {
        let path = path?;
        let extension = path.extension()?.to_str()?;
        matches!(extension, "js" | "ts")
            .then(|| {
                path.strip_prefix(root)
                    .ok()
                    .map(|path| (name.to_owned(), path.to_path_buf()))
            })
            .flatten()
    }
}

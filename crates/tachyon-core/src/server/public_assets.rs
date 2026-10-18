use super::StaticFiles;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::SystemTime;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::server) struct PublicAssets {
    stamp: Option<(SystemTime, u64)>,
    paths: BTreeSet<String>,
}

impl PublicAssets {
    const WORKER_FILE: &'static str = "tachyon-sw.js";
    const TABLE_START: &'static str = "const ASSETS = ";

    pub(in crate::server) fn read(root: &Path) -> Self {
        let worker = root.join(Self::WORKER_FILE);
        Self {
            stamp: Self::stamp(&worker),
            paths: Self::read_paths(&worker),
        }
    }

    pub(in crate::server) fn is_current(&self, root: &Path) -> bool {
        let worker = root.join(Self::WORKER_FILE);
        self.stamp == Self::stamp(&worker)
    }

    pub(in crate::server) fn lists(&self, path: &str) -> bool {
        self.paths.contains(path)
    }

    fn stamp(worker: &Path) -> Option<(SystemTime, u64)> {
        let metadata = std::fs::symlink_metadata(worker).ok()?;
        let modified = metadata.modified().ok()?;
        Some((modified, metadata.len()))
    }

    fn read_paths(worker: &Path) -> BTreeSet<String> {
        Self::worker_text(worker)
            .and_then(|text| Self::table_paths(&text))
            .unwrap_or_default()
    }

    fn worker_text(worker: &Path) -> Option<String> {
        let metadata = std::fs::symlink_metadata(worker).ok()?;
        Some(metadata).filter(StaticFiles::is_small_regular_file)?;
        std::fs::read_to_string(worker).ok()
    }

    fn table_paths(text: &str) -> Option<BTreeSet<String>> {
        let table = text
            .lines()
            .find_map(|line| line.strip_prefix(Self::TABLE_START))?;
        let assets: BTreeMap<String, Value> = serde_json::from_str(table).ok()?;
        Some(assets.into_keys().collect())
    }
}

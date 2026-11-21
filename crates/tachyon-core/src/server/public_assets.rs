use super::StaticFiles;
use crate::routing::MountPath;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::server) struct PublicAssets {
    tables: Vec<PathBuf>,
    stamps: Vec<Option<(SystemTime, u64)>>,
    paths: BTreeSet<String>,
}

impl PublicAssets {
    const WORKER_FILE: &'static str = "tachyon-sw.js";
    const APP_LIST: &'static str = ".tachyon/apps.json";
    const TABLE_START: &'static str = "const ASSETS = ";

    pub(in crate::server) fn read(root: &Path) -> Self {
        let apps = Self::app_tables(root);
        let mut paths = Self::read_paths(&root.join(Self::WORKER_FILE));
        paths.extend(apps.iter().flat_map(|table| Self::app_paths(table)));
        let tables = Self::watched(root, apps);
        Self {
            stamps: Self::stamps(&tables),
            tables,
            paths,
        }
    }

    pub(in crate::server) fn is_current(&self, root: &Path) -> bool {
        let tables = Self::watched(root, Self::app_tables(root));
        [tables == self.tables, Self::stamps(&tables) == self.stamps] == [true, true]
    }

    pub(in crate::server) fn lists(&self, path: &str) -> bool {
        self.paths.contains(path)
    }

    fn watched(root: &Path, apps: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut tables = vec![root.join(Self::WORKER_FILE), root.join(Self::APP_LIST)];
        tables.extend(apps);
        tables
    }

    fn stamps(tables: &[PathBuf]) -> Vec<Option<(SystemTime, u64)>> {
        tables.iter().map(|table| Self::stamp(table)).collect()
    }

    fn stamp(file: &Path) -> Option<(SystemTime, u64)> {
        let metadata = std::fs::symlink_metadata(file).ok()?;
        let modified = metadata.modified().ok()?;
        Some((modified, metadata.len()))
    }

    fn read_paths(worker: &Path) -> BTreeSet<String> {
        Self::small_text(worker)
            .and_then(|text| Self::table_paths(&text))
            .unwrap_or_default()
    }

    fn small_text(file: &Path) -> Option<String> {
        let metadata = std::fs::symlink_metadata(file).ok()?;
        Some(metadata).filter(StaticFiles::is_small_regular_file)?;
        std::fs::read_to_string(file).ok()
    }

    fn table_paths(text: &str) -> Option<BTreeSet<String>> {
        let table = text
            .lines()
            .find_map(|line| line.strip_prefix(Self::TABLE_START))?;
        let assets: BTreeMap<String, Value> = serde_json::from_str(table).ok()?;
        Some(assets.into_keys().collect())
    }

    fn app_tables(root: &Path) -> Vec<PathBuf> {
        Self::app_prefixes(root)
            .iter()
            .filter_map(|prefix| MountPath::parse(prefix).ok())
            .map(|mount| Self::app_table(root, &mount))
            .collect()
    }

    fn app_table(root: &Path, mount: &MountPath) -> PathBuf {
        root.join(mount.prefix().trim_start_matches('/'))
            .join(MountPath::APP_TABLE)
    }

    fn app_prefixes(root: &Path) -> Vec<String> {
        Self::json(&root.join(Self::APP_LIST))
            .and_then(|list| serde_json::from_value(list["apps"].clone()).ok())
            .unwrap_or_default()
    }

    fn app_paths(table: &Path) -> BTreeSet<String> {
        Self::json(table)
            .and_then(|table| {
                serde_json::from_value::<BTreeMap<String, Value>>(table["assets"].clone()).ok()
            })
            .map(|assets| assets.into_keys().collect())
            .unwrap_or_default()
    }

    fn json(file: &Path) -> Option<Value> {
        Self::small_text(file).and_then(|text| serde_json::from_str(&text).ok())
    }
}

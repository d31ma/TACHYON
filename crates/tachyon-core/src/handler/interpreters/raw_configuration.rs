use super::raw_worker::RawWorker;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawConfiguration {
    #[serde(default)]
    pub(super) interpreters: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub(super) workers: BTreeMap<String, RawWorker>,
}

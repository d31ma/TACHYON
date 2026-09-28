use serde_json::Value;
use std::path::Path;

pub(super) struct ApplicationVersion;

impl ApplicationVersion {
    const MANIFEST: &'static str = "client/manifest.json";
    const POINTER: &'static str = "/tac/application/version";
    pub(super) const DEFAULT: &'static str = "1.0.0";
    const SEPARATOR: char = '.';

    pub(super) fn current(project_root: &Path) -> String {
        let manifest: Value = std::fs::read(project_root.join(Self::MANIFEST))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null);
        let version = manifest.pointer(Self::POINTER).and_then(Value::as_str);
        String::from(version.unwrap_or(Self::DEFAULT))
    }

    pub(super) fn major(version: &str) -> u64 {
        version
            .split(Self::SEPARATOR)
            .next()
            .and_then(|part| part.trim().parse().ok())
            .unwrap_or(0)
    }
}

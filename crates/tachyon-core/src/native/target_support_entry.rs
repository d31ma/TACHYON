use super::target_reach::TargetReach;
use crate::AppTarget;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TargetSupportEntry {
    member: String,
    paths: Vec<String>,
    web: TargetReach,
    macos: TargetReach,
    windows: TargetReach,
    linux: TargetReach,
    ios: TargetReach,
    android: TargetReach,
}

impl TargetSupportEntry {
    pub(super) fn member(&self) -> &str {
        &self.member
    }

    #[cfg(test)]
    pub(super) fn paths(&self) -> &[String] {
        &self.paths
    }

    pub(super) fn reach(&self, target: AppTarget) -> &TargetReach {
        [
            &self.web,
            &self.macos,
            &self.windows,
            &self.linux,
            &self.ios,
            &self.android,
        ][target as usize]
    }

    pub(super) fn is_declared_in(&self, manifest: &Value) -> bool {
        self.paths.iter().any(|path| Self::declares(manifest, path))
    }

    fn declares(manifest: &Value, path: &str) -> bool {
        let item = Self::item(path);
        manifest
            .pointer(&Self::pointer(path))
            .is_some_and(|value| Self::holds(value, item))
    }

    fn pointer(path: &str) -> String {
        let field = path.split('[').next().unwrap_or(path);
        format!("/{}", field.replace('.', "/"))
    }

    fn item(path: &str) -> Option<&str> {
        path.split_once('[')
            .map(|(_, rest)| rest.trim_end_matches(']'))
    }

    fn holds(value: &Value, item: Option<&str>) -> bool {
        item.map_or_else(|| Self::present(value), |name| Self::lists(value, name))
    }

    fn present(value: &Value) -> bool {
        let empty = [
            value.is_null(),
            value.as_array().is_some_and(Vec::is_empty),
            value.as_object().is_some_and(serde_json::Map::is_empty),
        ];
        !empty.contains(&true)
    }

    fn lists(value: &Value, name: &str) -> bool {
        value
            .as_array()
            .is_some_and(|items| items.iter().any(|entry| entry.as_str() == Some(name)))
    }
}

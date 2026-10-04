use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct RustDependency {
    pub(super) version: String,
    pub(super) features: Vec<String>,
    pub(super) default_features: bool,
}

impl RustDependency {
    const VERSION: &'static str = "version";
    const FEATURES: &'static str = "features";
    const DEFAULT_FEATURES: &'static str = "defaultFeatures";
    const EXACT_PREFIX: char = '=';
    const VERSION_PARTS: usize = 3;
    const MAX_NAME: usize = 64;
    const MAX_FEATURES: usize = 64;
    const SHAPE: &'static str = "a dependency is an exact version such as \"=1.0.151\", or an object with \"version\", optional \"features\" and optional \"defaultFeatures\"";

    pub(super) fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= Self::MAX_NAME
            && name.starts_with(|first: char| first.is_ascii_alphabetic())
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character))
    }

    pub(super) fn problem(&self) -> Option<String> {
        if !Self::exact_version(&self.version) {
            return Some(format!(
                "version \"{}\" is not exact; write it as =MAJOR.MINOR.PATCH so the vendored crate matches",
                self.version
            ));
        }
        if self.features.len() > Self::MAX_FEATURES {
            return Some(format!("lists more than {} features", Self::MAX_FEATURES));
        }
        self.features
            .iter()
            .find(|feature| !Self::valid_feature(feature))
            .map(|feature| format!("feature \"{feature}\" is not a Cargo feature name"))
    }

    pub(super) fn toml(&self) -> String {
        let mut parts = vec![format!("version = \"{}\"", self.version)];
        if !self.features.is_empty() {
            let quoted = self
                .features
                .iter()
                .map(|feature| format!("\"{feature}\""))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("features = [{quoted}]"));
        }
        if !self.default_features {
            parts.push(String::from("default-features = false"));
        }
        format!("{{ {} }}", parts.join(", "))
    }

    fn exact_version(version: &str) -> bool {
        let Some(number) = version.strip_prefix(Self::EXACT_PREFIX) else {
            return false;
        };
        let parts = number.split('.').collect::<Vec<_>>();
        parts.len() == Self::VERSION_PARTS
            && parts.iter().all(|part| {
                !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
            })
    }

    fn valid_feature(feature: &str) -> bool {
        !feature.is_empty()
            && feature.len() <= Self::MAX_NAME
            && feature
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_+.".contains(character))
    }

    fn from_value(value: &Value) -> Result<Self, String> {
        match value {
            Value::String(version) => Ok(Self {
                version: version.clone(),
                features: Vec::new(),
                default_features: true,
            }),
            Value::Object(table) => Self::from_table(table),
            _ => Err(String::from(Self::SHAPE)),
        }
    }

    fn from_table(table: &serde_json::Map<String, Value>) -> Result<Self, String> {
        let known = [Self::VERSION, Self::FEATURES, Self::DEFAULT_FEATURES];
        if table.keys().any(|key| !known.contains(&key.as_str())) {
            return Err(String::from(Self::SHAPE));
        }
        let version = table.get(Self::VERSION).and_then(Value::as_str);
        let features = match table.get(Self::FEATURES) {
            None => Some(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|item| item.as_str().map(String::from))
                .collect(),
            Some(_) => None,
        };
        let default_features = match table.get(Self::DEFAULT_FEATURES) {
            None => Some(true),
            Some(value) => value.as_bool(),
        };
        match (version, features, default_features) {
            (Some(version), Some(features), Some(default_features)) => Ok(Self {
                version: String::from(version),
                features,
                default_features,
            }),
            _ => Err(String::from(Self::SHAPE)),
        }
    }
}

impl<'de> Deserialize<'de> for RustDependency {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Self::from_value(&value).map_err(D::Error::custom)
    }
}

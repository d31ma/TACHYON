use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

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
    const KNOWN_KEYS: [&'static str; 3] = [Self::VERSION, Self::FEATURES, Self::DEFAULT_FEATURES];
    const EXACT_PREFIX: char = '=';
    const VERSION_SEPARATOR: char = '.';
    const VERSION_PARTS: usize = 3;
    const MAX_NAME: usize = 64;
    const MAX_FEATURES: usize = 64;
    const NAME_MARKS: &'static str = "-_";
    const FEATURE_MARKS: &'static str = "-_+.";
    const SHAPE: &'static str = "a dependency is an exact version such as \"=1.0.151\", or an object with \"version\", optional \"features\" and optional \"defaultFeatures\"";

    pub(super) fn valid_name(name: &str) -> bool {
        let bounded = (1..=Self::MAX_NAME).contains(&name.len());
        let starts_with_letter = name.starts_with(|first: char| first.is_ascii_alphabetic());
        bounded & starts_with_letter & name.chars().all(Self::name_character)
    }

    pub(super) fn problem(&self) -> Option<String> {
        self.version_problem()
            .or_else(|| self.feature_count_problem())
            .or_else(|| self.feature_name_problem())
    }

    pub(super) fn toml(&self) -> String {
        let parts = [
            Some(format!("version = \"{}\"", self.version)),
            self.features_part(),
            self.default_features_part(),
        ];
        let parts = parts.into_iter().flatten().collect::<Vec<_>>();
        format!("{{ {} }}", parts.join(", "))
    }

    fn version_problem(&self) -> Option<String> {
        (!Self::exact_version(&self.version)).then(|| {
            format!(
                "version \"{}\" is not exact; write it as =MAJOR.MINOR.PATCH so the vendored crate matches",
                self.version
            )
        })
    }

    fn feature_count_problem(&self) -> Option<String> {
        (self.features.len() > Self::MAX_FEATURES)
            .then(|| format!("lists more than {} features", Self::MAX_FEATURES))
    }

    fn feature_name_problem(&self) -> Option<String> {
        self.features
            .iter()
            .find(|feature| !Self::valid_feature(feature))
            .map(|feature| format!("feature \"{feature}\" is not a Cargo feature name"))
    }

    fn features_part(&self) -> Option<String> {
        let quoted = self
            .features
            .iter()
            .map(|feature| format!("\"{feature}\""))
            .collect::<Vec<_>>();
        (!quoted.is_empty()).then(|| format!("features = [{}]", quoted.join(", ")))
    }

    fn default_features_part(&self) -> Option<String> {
        (!self.default_features).then(|| String::from("default-features = false"))
    }

    fn exact_version(version: &str) -> bool {
        let number = version.strip_prefix(Self::EXACT_PREFIX).unwrap_or_default();
        let parts = number.split(Self::VERSION_SEPARATOR).collect::<Vec<_>>();
        let exact = version.starts_with(Self::EXACT_PREFIX) & (parts.len() == Self::VERSION_PARTS);
        exact & parts.iter().all(|part| Self::number(part))
    }

    fn number(part: &str) -> bool {
        !part.is_empty() & part.chars().all(|character| character.is_ascii_digit())
    }

    fn name_character(character: char) -> bool {
        character.is_ascii_alphanumeric() | Self::NAME_MARKS.contains(character)
    }

    fn feature_character(character: char) -> bool {
        character.is_ascii_alphanumeric() | Self::FEATURE_MARKS.contains(character)
    }

    fn valid_feature(feature: &str) -> bool {
        let bounded = (1..=Self::MAX_NAME).contains(&feature.len());
        bounded & feature.chars().all(Self::feature_character)
    }

    fn from_value(value: &Value) -> Result<Self, String> {
        value
            .as_str()
            .map(Self::exact)
            .map_or_else(|| Self::from_table(value), Ok)
    }

    fn exact(version: &str) -> Self {
        Self {
            version: version.to_owned(),
            features: Vec::new(),
            default_features: true,
        }
    }

    fn from_table(value: &Value) -> Result<Self, String> {
        let table = value.as_object().ok_or_else(Self::shape)?;
        Self::known_keys(table)?;
        let version = table
            .get(Self::VERSION)
            .and_then(Value::as_str)
            .ok_or_else(Self::shape)?;
        let features = table
            .get(Self::FEATURES)
            .map_or_else(|| Some(Vec::new()), Self::feature_list)
            .ok_or_else(Self::shape)?;
        let default_features = table
            .get(Self::DEFAULT_FEATURES)
            .map_or(Some(true), Value::as_bool)
            .ok_or_else(Self::shape)?;
        Ok(Self {
            version: version.to_owned(),
            features,
            default_features,
        })
    }

    fn known_keys(table: &Map<String, Value>) -> Result<(), String> {
        table
            .keys()
            .all(|key| Self::KNOWN_KEYS.contains(&key.as_str()))
            .then_some(())
            .ok_or_else(Self::shape)
    }

    fn feature_list(value: &Value) -> Option<Vec<String>> {
        value
            .as_array()?
            .iter()
            .map(|item| item.as_str().map(String::from))
            .collect()
    }

    fn shape() -> String {
        String::from(Self::SHAPE)
    }
}

impl<'de> Deserialize<'de> for RustDependency {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Self::from_value(&value).map_err(D::Error::custom)
    }
}

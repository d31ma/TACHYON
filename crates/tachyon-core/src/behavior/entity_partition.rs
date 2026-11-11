use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};

pub(super) struct EntityPartition;

impl EntityPartition {
    const KEY: &'static str = "partition";
    const ADAPTER: &'static str = "indexeddb";
    const PLAIN_KEYS: &'static [&'static str] = &["adapter"];
    const PARTITION_KEYS: &'static [&'static str] = &["adapter", "partition"];

    pub(super) fn keys(written: Option<&Value>) -> &'static [&'static str] {
        let declared = written.is_some_and(|value| value.get(Self::KEY).is_some());
        [Self::PLAIN_KEYS, Self::PARTITION_KEYS][usize::from(declared)]
    }

    pub(super) fn check_entity(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        if !object.contains_key(Self::KEY) {
            return Ok(());
        }
        super::named(input, marker, object, Self::KEY)?;
        Self::check_adapter(input, marker, object)
    }

    fn check_adapter(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        if object.get("adapter").and_then(Value::as_str) == Some(Self::ADAPTER) {
            return Ok(());
        }
        Err(input.failure(
            marker.span.start,
            "Entity.partition needs the \"indexeddb\" adapter: only the browser Store loads a table one partition at a time",
        ))
    }

    pub(super) fn check_query(
        input: &Input<'_>,
        at: usize,
        entity: &Value,
        query: &Value,
    ) -> Result<(), Failure> {
        let Some(field) = entity.get(Self::KEY) else {
            return Ok(());
        };
        if Self::compares(field, query) {
            return Ok(());
        }
        Err(input.failure(
            at,
            format!(
                "The Entity is split by its partition field '{}', so every Store query must compare that field with Equals, for example findBy{}AndStatus: each read then names one partition. Such a Store cannot extend Repository, because findById and findAll name no partition",
                field.as_str().unwrap_or_default(),
                Self::capitalized(field.as_str().unwrap_or_default())
            ),
        ))
    }

    fn compares(field: &Value, query: &Value) -> bool {
        query["where"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|predicate| Self::equals(field, predicate))
    }

    fn equals(field: &Value, predicate: &Value) -> bool {
        predicate["field"] == *field && predicate["operator"] == "eq"
    }

    fn capitalized(field: &str) -> String {
        let mut letters = field.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        })
    }

    pub(super) fn add_to(entity: &Value, described: &mut Value) {
        if let Some(field) = entity.get(Self::KEY) {
            described[Self::KEY] = field.clone();
        }
    }
}

use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};

pub(super) struct EntityPartition;

impl EntityPartition {
    const KEY: &'static str = "partition";
    const ADAPTER: &'static str = "indexeddb";
    const KEY_SETS: [&'static [&'static str]; 2] = [&["adapter"], &["adapter", "partition"]];

    pub(super) fn keys(written: Option<&Value>) -> &'static [&'static str] {
        Self::KEY_SETS[usize::from(Self::declared(written))]
    }

    fn declared(written: Option<&Value>) -> bool {
        written.is_some_and(|value| value.get(Self::KEY).is_some())
    }

    pub(super) fn check_entity(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        Self::check_name(input, marker, object)?;
        Self::check_adapter(input, marker, object)
    }

    fn check_name(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        if !object.contains_key(Self::KEY) {
            return Ok(());
        }
        super::named(input, marker, object, Self::KEY)
    }

    fn check_adapter(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        let indexeddb = object.get("adapter").and_then(Value::as_str) == Some(Self::ADAPTER);
        if indexeddb || !object.contains_key(Self::KEY) {
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
        let missing = Self::uncompared(entity.get(Self::KEY), query);
        missing.map_or(Ok(()), |field| Err(Self::refusal(input, at, field)))
    }

    fn uncompared<'entity>(field: Option<&'entity Value>, query: &Value) -> Option<&'entity Value> {
        field.filter(|field| !Self::compares(field, query))
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

    fn refusal(input: &Input<'_>, at: usize, field: &Value) -> Failure {
        let name = field.as_str().unwrap_or_default();
        input.failure(
            at,
            format!(
                "The Entity is split by its partition field '{name}', so every Store query must compare that field with Equals, for example findBy{}AndStatus: each read then names one partition. Such a Store cannot extend Repository, because findById and findAll name no partition",
                Self::capitalized(name)
            ),
        )
    }

    fn capitalized(field: &str) -> String {
        let mut letters = field.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        })
    }

    pub(super) fn add_to(entity: &Value, described: &mut Value) {
        Self::copy_field(entity.get(Self::KEY), described);
    }

    fn copy_field(field: Option<&Value>, described: &mut Value) {
        if let Some(field) = field {
            described[Self::KEY] = field.clone();
        }
    }
}

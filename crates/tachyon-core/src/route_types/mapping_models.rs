use super::ModelTable;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) struct MappingModels;

impl MappingModels {
    pub(crate) fn mentioned(table: &ModelTable, source: &str) -> Map<String, Value> {
        let words: BTreeSet<&str> = source.split(Self::word_boundary).collect();
        let names = table
            .record_names()
            .into_iter()
            .filter(|name| words.contains(name.as_str()));
        let models = names.filter_map(|name| Self::described(table, &name));
        models
            .flatten()
            .map(|(model, description)| (model, description["fields"].clone()))
            .collect()
    }

    fn word_boundary(character: char) -> bool {
        !character.is_ascii_alphanumeric() && character != '_'
    }

    fn described(table: &ModelTable, name: &str) -> Option<Map<String, Value>> {
        table
            .event_models(name)
            .ok()
            .and_then(|value| value.as_object().cloned())
    }
}

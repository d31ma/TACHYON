use super::ModelTable;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) struct MappingModels;

impl MappingModels {
    pub(crate) fn mentioned(table: &ModelTable, source: &str) -> Map<String, Value> {
        let words: BTreeSet<&str> = source
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .collect();
        let mut models = Map::new();
        for name in table.record_names() {
            if !words.contains(name.as_str()) {
                continue;
            }
            let Ok(Value::Object(described)) = table.event_models(&name) else {
                continue;
            };
            for (model, description) in described {
                models.insert(model, description["fields"].clone());
            }
        }
        models
    }
}

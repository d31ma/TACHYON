use serde_json::{Map, Value};

pub(super) struct JsonFields;

impl JsonFields {
    pub(super) fn object<'a>(value: &'a Value, key: &str) -> &'a Map<String, Value> {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        value
            .get(key)
            .and_then(Value::as_object)
            .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
    }

    pub(super) fn names<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
        value
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect()
    }
}

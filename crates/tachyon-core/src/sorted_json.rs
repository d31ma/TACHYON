use serde_json::{Map, Value};

#[cfg(test)]
mod tests;

pub(crate) struct SortedJson;

impl SortedJson {
    pub(crate) fn value(value: &Value) -> Value {
        Self::sorted(value)
    }

    pub(crate) fn text(value: &Value) -> String {
        let mut written = String::new();
        Self::write(value, &mut written);
        written
    }

    pub(crate) fn pretty(value: &Value) -> String {
        serde_json::to_string_pretty(&Self::sorted(value)).unwrap_or_default()
    }

    pub(crate) fn map(map: &Map<String, Value>) -> Map<String, Value> {
        Self::sorted_map(map)
    }

    pub(crate) fn entries(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
        let mut entries: Vec<(&String, &Value)> = map.iter().collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));
        entries
    }

    pub(crate) fn keys(map: &Map<String, Value>) -> Vec<&String> {
        Self::entries(map).into_iter().map(|(key, _)| key).collect()
    }

    pub(crate) fn values(map: &Map<String, Value>) -> Vec<&Value> {
        Self::entries(map)
            .into_iter()
            .map(|(_, value)| value)
            .collect()
    }

    fn write(value: &Value, written: &mut String) {
        match value {
            Value::Object(map) => Self::write_object(map, written),
            Value::Array(items) => Self::write_array(items, written),
            scalar => written.push_str(&serde_json::to_string(scalar).unwrap_or_default()),
        }
    }

    fn write_object(map: &Map<String, Value>, written: &mut String) {
        written.push('{');
        for (index, (key, item)) in Self::entries(map).into_iter().enumerate() {
            written.push_str(["", ","][usize::from(index > 0)]);
            written.push_str(&serde_json::to_string(key).unwrap_or_default());
            written.push(':');
            Self::write(item, written);
        }
        written.push('}');
    }

    fn write_array(items: &[Value], written: &mut String) {
        written.push('[');
        for (index, item) in items.iter().enumerate() {
            written.push_str(["", ","][usize::from(index > 0)]);
            Self::write(item, written);
        }
        written.push(']');
    }

    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(Self::sorted_map(map)),
            Value::Array(items) => Value::Array(items.iter().map(Self::sorted).collect()),
            other => other.clone(),
        }
    }

    fn sorted_map(map: &Map<String, Value>) -> Map<String, Value> {
        Self::entries(map)
            .into_iter()
            .map(|(key, value)| (key.clone(), Self::sorted(value)))
            .collect()
    }
}

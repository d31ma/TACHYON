use serde_json::Value;
use std::collections::BTreeMap;

pub(super) struct RequestFields<'a> {
    pub(super) fields: BTreeMap<String, (bool, &'a Value)>,
}

impl<'a> RequestFields<'a> {
    const BODY: &'static str = "body";
    const OPTIONAL_MARK: char = '?';

    pub(super) fn of(request: &'a Value) -> Self {
        let places = request.as_object().into_iter().flatten();
        let fields = places
            .filter(|(place, _)| place.as_str() != Self::BODY)
            .flat_map(|(place, fields)| Self::place_fields(place, fields))
            .collect();
        Self { fields }
    }

    fn place_fields(place: &str, fields: &'a Value) -> Vec<(String, (bool, &'a Value))> {
        fields
            .as_object()
            .into_iter()
            .flatten()
            .map(|(key, schema)| Self::field(place, key, schema))
            .collect()
    }

    fn field(place: &str, key: &str, schema: &'a Value) -> (String, (bool, &'a Value)) {
        let name = key.trim_end_matches(Self::OPTIONAL_MARK);
        let required = !key.ends_with(Self::OPTIONAL_MARK);
        (format!("{place} field '{name}'"), (required, schema))
    }
}

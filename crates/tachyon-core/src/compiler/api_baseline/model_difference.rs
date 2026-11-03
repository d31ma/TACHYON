use super::JsonFields;
use serde_json::Value;

pub(super) struct ModelDifference;

impl ModelDifference {
    const MODELS: &'static str = "models";
    const PROPERTIES: &'static str = "properties";
    const REQUIRED: &'static str = "required";

    pub(super) fn breaks(old: &Value, new: &Value) -> Vec<String> {
        let current = JsonFields::object(new, Self::MODELS);
        JsonFields::object(old, Self::MODELS)
            .iter()
            .flat_map(|(name, model)| Self::one_model_breaks(name, model, current.get(name)))
            .collect()
    }

    fn one_model_breaks(name: &str, old: &Value, new: Option<&Value>) -> Vec<String> {
        new.map_or_else(
            || vec![format!("the model {name} was removed")],
            |new| Self::present_model_breaks(name, old, new),
        )
    }

    fn present_model_breaks(name: &str, old: &Value, new: &Value) -> Vec<String> {
        let old_fields = JsonFields::object(old, Self::PROPERTIES);
        let new_fields = JsonFields::object(new, Self::PROPERTIES);
        let mut found: Vec<String> = old_fields
            .iter()
            .filter(|(field, schema)| new_fields.get(*field) != Some(*schema))
            .map(|(field, _)| format!("the field {name}.{field} was removed or changed its type"))
            .collect();
        found.extend(
            JsonFields::names(new, Self::REQUIRED)
                .into_iter()
                .filter(|field| !JsonFields::names(old, Self::REQUIRED).contains(field))
                .map(|field| format!("the field {name}.{field} is now required")),
        );
        found.extend(Self::alias_breaks(name, old, new, old_fields.is_empty()));
        found
    }

    fn alias_breaks(name: &str, old: &Value, new: &Value, is_alias: bool) -> Option<String> {
        let message = format!("the model {name} changed its type");
        (is_alias && old != new).then_some(message)
    }
}

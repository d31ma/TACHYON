use crate::route_types::{ModelField, Record, Shape};

pub(crate) struct LuaModels;

impl LuaModels {
    pub(crate) fn converters(records: &[(String, Record)]) -> String {
        Self::model_lines(records)
    }

    pub(crate) fn named_converters(records: &[(String, Record)]) -> String {
        records
            .iter()
            .map(|(name, record)| Self::named_model_text(name, record))
            .collect()
    }

    pub(crate) fn descriptor(shape: &Shape) -> Option<String> {
        Self::shape_descriptor(shape)
    }

    fn model_lines(records: &[(String, Record)]) -> String {
        records
            .iter()
            .map(|(name, record)| Self::model_text(name, record))
            .collect()
    }

    fn named_model_text(name: &str, record: &Record) -> String {
        format!("{name}.__name = {name:?}\n{}", Self::model_text(name, record))
    }

    fn model_text(name: &str, record: &Record) -> String {
        format!(
            "YonModels.field_order[{name}] = {{ {} }}\nYonModels.field_types[{name}] = {{ {} }}\n",
            Self::field_order(&record.fields),
            Self::field_types(&record.fields)
        )
    }

    fn field_order(fields: &[ModelField]) -> String {
        fields
            .iter()
            .map(|field| format!("{:?}", field.name))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn field_types(fields: &[ModelField]) -> String {
        fields
            .iter()
            .filter_map(Self::field_type)
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn field_type(field: &ModelField) -> Option<String> {
        Self::shape_descriptor(&field.shape)
            .map(|descriptor| format!("{} = {descriptor}", field.name))
    }

    fn shape_descriptor(shape: &Shape) -> Option<String> {
        match shape {
            Shape::Model(name) => Some(format!("{{ model = {name} }}")),
            Shape::List(item) => Self::list_descriptor(Self::descriptor(item)),
            Shape::Optional(inner) => Self::descriptor(inner),
            _ => None,
        }
    }

    fn list_descriptor(item: Option<String>) -> Option<String> {
        item.map(|item| format!("{{ list = true, item = {item} }}"))
    }
}

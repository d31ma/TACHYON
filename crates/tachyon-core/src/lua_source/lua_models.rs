use crate::route_types::{Record, Shape};
use std::fmt::Write as _;

pub(crate) struct LuaModels;

impl LuaModels {
    pub(crate) fn converters(records: &[(String, Record)]) -> String {
        let mut output = String::new();
        for (name, record) in records {
            let order = record
                .fields
                .iter()
                .map(|field| format!("{:?}", field.name))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(output, "YonModels.field_order[{name}] = {{ {order} }}");
            let types = record
                .fields
                .iter()
                .filter_map(|field| {
                    Self::descriptor(&field.shape).map(|descriptor| format!("{} = {descriptor}", field.name))
                })
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(output, "YonModels.field_types[{name}] = {{ {types} }}");
        }
        output
    }

    pub(crate) fn descriptor(shape: &Shape) -> Option<String> {
        match shape {
            Shape::Model(name) => Some(format!("{{ model = {name} }}")),
            Shape::List(item) => {
                Self::descriptor(item).map(|item| format!("{{ list = true, item = {item} }}"))
            }
            Shape::Optional(inner) => Self::descriptor(inner),
            _ => None,
        }
    }
}

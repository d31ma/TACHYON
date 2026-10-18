use crate::behavior::LuaBehaviorPlan;
use crate::lua_source::LuaFields;
use serde_json::Value;

pub(super) struct LuaConverterProviders;

impl LuaConverterProviders {
    const PROVIDED_METHODS: [&'static str; 2] = ["query", "httpExchange"];

    pub(super) fn written(
        class: &str,
        source: &str,
        plan: &LuaBehaviorPlan,
    ) -> Result<String, String> {
        let fields = Self::bindings(class, source, plan)?;
        let entries = Self::entries(&fields);
        Ok(format!("{class}.__tachyonContracts = {{ {entries} }}\n"))
    }

    pub(super) fn hide_fields(
        metadata: &mut Value,
        class: &str,
        source: &str,
        plan: &LuaBehaviorPlan,
    ) -> Result<(), String> {
        let fields = Self::bindings(class, source, plan)?;
        Self::remove_fields(metadata, &fields);
        Ok(())
    }

    fn bindings(
        class: &str,
        source: &str,
        plan: &LuaBehaviorPlan,
    ) -> Result<Vec<(String, String)>, String> {
        let records: Value =
            serde_json::from_str(&plan.records).map_err(|error| error.to_string())?;
        Ok(Self::converter_fields(class, source, &records))
    }

    fn converter_fields(class: &str, source: &str, records: &Value) -> Vec<(String, String)> {
        LuaFields::without_values(source, class)
            .into_iter()
            .filter(|(_, name, marked)| Self::is_provider(name, *marked, records))
            .map(|(field, name, _)| (field, name))
            .collect()
    }

    fn is_provider(name: &str, marked: bool, records: &Value) -> bool {
        let record = &records[name];
        marked && [record["mapper"] == true, Self::answers_calls(record)].contains(&true)
    }

    fn answers_calls(record: &Value) -> bool {
        record["methods"].as_object().is_some_and(|methods| {
            methods.values().any(|method| {
                Self::PROVIDED_METHODS
                    .iter()
                    .any(|key| method.get(key).is_some())
            })
        })
    }

    fn entries(fields: &[(String, String)]) -> String {
        fields
            .iter()
            .map(|(field, name)| format!("[{field:?}] = {name:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn remove_fields(metadata: &mut Value, fields: &[(String, String)]) {
        let names: Vec<String> = fields
            .iter()
            .map(|(field, _)| crate::companion::camel_case(field))
            .collect();
        for key in ["fields", "writable", "required"] {
            Self::remove_array_fields(&mut metadata[key], &names);
        }
        Self::remove_named_fields(&mut metadata["names"], &names);
    }

    fn remove_array_fields(metadata: &mut Value, names: &[String]) {
        if let Some(values) = metadata.as_array_mut() {
            values.retain(|value| {
                value
                    .as_str()
                    .is_none_or(|value| !names.iter().any(|name| name == value))
            });
        }
    }

    fn remove_named_fields(metadata: &mut Value, names: &[String]) {
        if let Some(values) = metadata.as_object_mut() {
            values.retain(|name, _| !names.iter().any(|field| field == name));
        }
    }
}

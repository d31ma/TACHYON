use crate::behavior::LuaBehaviorPlan;
use crate::lua_source::LuaFields;
use serde_json::Value;

pub(super) struct LuaConverterProviders;

impl LuaConverterProviders {
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
            .filter(|(_, name, marked)| *marked && records[name]["mapper"] == true)
            .map(|(field, name, _)| (field, name))
            .collect()
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
            if let Some(values) = metadata[key].as_array_mut() {
                values.retain(|value| {
                    value
                        .as_str()
                        .is_none_or(|value| !names.iter().any(|name| name == value))
                });
            }
        }
        if let Some(values) = metadata["names"].as_object_mut() {
            values.retain(|name, _| !names.contains(name));
        }
    }
}

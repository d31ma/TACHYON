use super::module_code::ModuleCode;
use serde_json::Value;

pub(super) struct SetupRegistrations;

impl SetupRegistrations {
    const CALL: &'static str = "__tachyon_create_behaviors__(";
    const TYPE_FIELD: &'static str = "type:";
    const NAME_END: char = ',';
    const ENTRY_START: u8 = b'{';

    pub(super) fn names(module: &ModuleCode) -> Vec<String> {
        let start = module.code.find(Self::CALL).unwrap_or(module.code.len());
        module.code[start..]
            .match_indices(Self::TYPE_FIELD)
            .filter_map(|(offset, _)| Self::setup_name(module, start + offset))
            .collect()
    }

    fn setup_name(module: &ModuleCode, at: usize) -> Option<String> {
        let open = module.code[..at].trim_end().len().checked_sub(1)?;
        let open = Some(open).filter(|open| module.code.as_bytes()[*open] == Self::ENTRY_START)?;
        let name_start = at + Self::TYPE_FIELD.len();
        let name_end = name_start + module.code[name_start..].find(Self::NAME_END)?;
        let close = module.matching_brace(open)?;
        let fields = format!("{{{}", &module.text[name_end + 1..=close]);
        let entry = serde_json::from_str::<Value>(&fields).ok()?;
        let name = module.code[name_start..name_end].trim();
        Self::is_setup_without_methods(&entry, name).then(|| name.to_owned())
    }

    fn is_setup_without_methods(entry: &Value, name: &str) -> bool {
        let checks = [
            entry["configuration"] == Value::Bool(true),
            entry["methods"] == Value::Array(Vec::new()),
            entry["name"] == name,
            entry.get("moduleOwned").is_none(),
        ];
        !checks.contains(&false)
    }
}

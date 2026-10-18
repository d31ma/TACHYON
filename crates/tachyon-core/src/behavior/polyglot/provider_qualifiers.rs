use crate::Failure;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) struct ProviderQualifiers;

impl ProviderQualifiers {
    pub(crate) fn read(path: &Path, source: &str, class: &str) -> Result<Value, Failure> {
        let prepared = super::prepare(path, source, "tac")?;
        Ok(Self::prepared(prepared.as_ref(), class))
    }

    fn prepared(prepared: Option<&super::Prepared>, class: &str) -> Value {
        let Some(super::Prepared::Annotated(module)) = prepared else {
            return json!({});
        };
        module
            .types
            .iter()
            .find(|ty| ty.name == class)
            .and_then(|ty| ty.policies.get("qualifiers"))
            .cloned()
            .unwrap_or_else(|| json!({}))
    }
}

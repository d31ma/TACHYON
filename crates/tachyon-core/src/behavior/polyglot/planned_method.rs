use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub(crate) struct PlannedMethod {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) parameters: Vec<String>,
    pub(crate) metadata: Map<String, Value>,
    pub(crate) declared: bool,
    pub(crate) asynchronous: bool,
    pub(crate) mapped_argument: Option<usize>,
}

impl PlannedMethod {
    pub(crate) fn rust_mapping_types(
        path: &std::path::Path,
        source: &str,
    ) -> Result<Vec<String>, crate::Failure> {
        Self::mapping_types(path, source)
    }

    fn mapping_types(path: &std::path::Path, source: &str) -> Result<Vec<String>, crate::Failure> {
        let source = crate::behavior::ServiceRoles::without_roles(path, source)?;
        let prepared = super::prepare(path, &source, "tac")?;
        Ok(prepared
            .into_iter()
            .flat_map(Self::prepared_mapping_types)
            .collect())
    }

    fn prepared_mapping_types(prepared: super::prepared::Prepared) -> Vec<String> {
        match prepared {
            super::prepared::Prepared::Erased(_) => Vec::new(),
            super::prepared::Prepared::Annotated(module) => module
                .types
                .into_iter()
                .flat_map(|kind| kind.methods)
                .flat_map(|method| Self::method_mapping_types(&method))
                .collect(),
        }
    }

    fn method_mapping_types(method: &super::Method) -> Vec<String> {
        method
            .metadata
            .get("mapping")
            .and_then(|policy| policy.get("result"))
            .and_then(Value::as_str)
            .map(|result| {
                method
                    .parameters
                    .iter()
                    .map(|parameter| parameter.type_source.clone())
                    .chain(std::iter::once(result.to_owned()))
                    .filter(|kind| crate::companion::RustModelType::is_model(kind))
                    .collect()
            })
            .unwrap_or_default()
    }
}

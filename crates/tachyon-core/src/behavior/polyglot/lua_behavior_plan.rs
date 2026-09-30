use super::ruby::RubyLowering;
use super::{LuaBehaviorMethod, Method, Module, Prepared, render};
use crate::Failure;
use std::path::Path;

pub(crate) struct LuaBehaviorPlan {
    pub(crate) records: String,
    pub(crate) module_id: String,
    pub(crate) erased: String,
    pub(crate) types: Vec<String>,
    pub(crate) methods: Vec<LuaBehaviorMethod>,
}

impl LuaBehaviorPlan {
    const RESOURCES: [(&'static str, &'static str); 2] =
        [("gateway", "Gateway"), ("httpExchange", "HTTPClient")];

    pub(crate) fn read(
        path: &Path,
        source: &str,
        environment: &str,
    ) -> Result<Option<Self>, Failure> {
        let prepared = super::prepare(path, source, environment)?;
        Self::of_prepared(prepared.as_ref())
            .map_err(|message| super::module_failure(path, source, message))
    }

    fn of_prepared(prepared: Option<&Prepared>) -> Result<Option<Self>, String> {
        match prepared {
            Some(Prepared::Annotated(module)) => Self::of(module).map(Some),
            _ => Ok(None),
        }
    }

    fn of(module: &Module) -> Result<Self, String> {
        Ok(Self {
            records: RubyLowering::records(module)?,
            module_id: module.module_id.clone(),
            erased: render(module, Vec::new(), "")?,
            types: module.types.iter().map(|ty| ty.name.clone()).collect(),
            methods: module
                .types
                .iter()
                .flat_map(|ty| {
                    ty.methods
                        .iter()
                        .map(|method| Self::method(&ty.name, method))
                })
                .collect(),
        })
    }

    fn method(owner: &str, method: &Method) -> LuaBehaviorMethod {
        LuaBehaviorMethod {
            owner: owner.to_owned(),
            name: method.name.clone(),
            parameters: method
                .parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect(),
            validated: Self::validated(method),
            resource: Self::resource(method),
        }
    }

    fn validated(method: &Method) -> Option<(usize, String)> {
        let policy = method
            .metadata
            .get("valid")
            .filter(|_| method.metadata.len() == 1)?;
        let index = policy["argument"]
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())?;
        Some((index, policy["schema"].to_string()))
    }

    fn resource(method: &Method) -> Option<&'static str> {
        Self::RESOURCES
            .iter()
            .find(|(key, _)| method.metadata.contains_key(*key))
            .map(|(_, name)| *name)
    }
}

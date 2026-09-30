use super::ruby::RubyLowering;
use super::{Method, Module, render};

pub(crate) struct LuaBehaviorMethod {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) parameters: Vec<String>,
    pub(crate) validated: Option<(usize, String)>,
    pub(crate) resource: Option<&'static str>,
}

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

    pub(super) fn of(module: &Module) -> Result<Self, String> {
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

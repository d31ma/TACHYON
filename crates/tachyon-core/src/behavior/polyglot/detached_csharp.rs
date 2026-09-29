use super::super::mapper;
use super::{Method, Module, PlannedMethod, PlannedType, Type, csharp};
use serde_json::{Map, Value};

pub(crate) struct DetachedCsharp {
    pub(crate) lowered: String,
    pub(crate) runtime: String,
    pub(crate) setups: String,
    pub(crate) methods: Vec<PlannedMethod>,
    pub(crate) types: Vec<PlannedType>,
}

impl DetachedCsharp {
    const CONFIGURE: &'static str = "    private static BehaviorAdapters Configure(";
    const CONFIGURATIONS_FIELD: &'static str = "    public static System.Func<System.Type, BehaviorExecution, BehaviorAdapters> Configurations;\n";
    const CONFIGURATIONS_CALL: &'static str = "if (Configurations != null) { var adapters = Configurations(configuration, execution); if (adapters != null) return adapters; }";
    const WRITTEN_BODIES: [&'static str; 3] = ["gateway", "query", "mapping"];
    const CONTEXT: &'static str = "behaviors";
    const TAC: &'static str = "tac";
    const NO_STORAGE: &'static str = "";
    const PAGE_STORAGE: &'static str = " BehaviorStorage.Host ??= TacWeb.Storage;";

    pub(super) fn of(module: &Module) -> Result<Self, String> {
        let lowered = csharp::lower(module)?;
        Ok(Self {
            lowered,
            runtime: Self::runtime(module),
            setups: Self::setups(module),
            methods: Self::methods(module),
            types: Self::types(module),
        })
    }

    pub(crate) fn with_lowered(self, lowered: String) -> Self {
        Self { lowered, ..self }
    }

    pub(crate) fn plain(lowered: String) -> Self {
        Self {
            lowered,
            runtime: String::new(),
            setups: String::new(),
            methods: Vec::new(),
            types: Vec::new(),
        }
    }

    fn runtime(module: &Module) -> String {
        let runtime = csharp::runtime(
            &module.module_id,
            &module.environment,
            &Self::registry(module),
            Self::CONFIGURATIONS_CALL,
            &Self::entities(module),
            &Self::policies(module),
        );
        runtime.replacen(
            Self::CONFIGURE,
            &format!("{}{}", Self::CONFIGURATIONS_FIELD, Self::CONFIGURE),
            1,
        )
    }

    fn policies(module: &Module) -> String {
        module.types.iter().map(Self::policy_line).collect()
    }

    fn policy_line(type_declaration: &Type) -> String {
        Self::named_line(
            "type",
            &type_declaration.name,
            &Value::Object(type_declaration.policies.clone()),
        )
    }

    fn entities(module: &Module) -> String {
        module.types.iter().filter_map(Self::entity_line).collect()
    }

    fn entity_line(type_declaration: &Type) -> Option<String> {
        let entity = type_declaration.entity.as_ref()?;
        let mapping = serde_json::json!({
            "entity": type_declaration.name,
            "adapter": entity["adapter"],
            "idField": entity["id"],
            "table": type_declaration.table
        });
        Some(Self::named_line("entity", &type_declaration.name, &mapping))
    }

    fn registry(module: &Module) -> String {
        let registrations: Map<String, Value> = module
            .types
            .iter()
            .map(|type_declaration| {
                (
                    type_declaration.name.clone(),
                    serde_json::json!({"cache": type_declaration.cache, "methods": Self::registered(type_declaration)}),
                )
            })
            .collect();
        Value::Object(registrations).to_string()
    }

    fn named_line(parameter: &str, name: &str, value: &Value) -> String {
        format!(
            "if ({parameter}.Name == {}) return BehaviorJSON.Parse({});\n",
            Self::quote(name),
            Self::quote(&value.to_string())
        )
    }

    fn quote(value: &str) -> String {
        Value::String(value.to_owned()).to_string()
    }

    fn registered(type_declaration: &Type) -> Map<String, Value> {
        type_declaration
            .methods
            .iter()
            .filter(|method| Self::invoked(method))
            .map(|method| (method.name.clone(), Value::Object(method.metadata.clone())))
            .collect()
    }

    fn invoked(method: &Method) -> bool {
        let written = Self::WRITTEN_BODIES
            .iter()
            .any(|key| method.metadata.contains_key(*key));
        [method.metadata.is_empty(), written] == [false, false]
    }

    fn setups(module: &Module) -> String {
        let lines: String = module
            .types
            .iter()
            .filter(|type_declaration| type_declaration.configuration)
            .map(csharp::configuration_line)
            .collect();
        let storage = Self::storage(module);
        format!(
            "static class BehaviorSetups {{\n    \
             [System.Runtime.CompilerServices.ModuleInitializer]\n    \
             internal static void Register() {{ Behaviors.Configurations = Configure;{storage} }}\n    \
             static BehaviorAdapters Configure(System.Type configuration, BehaviorExecution execution) {{\n{lines}return null;\n    }}\n}}\n"
        )
    }

    fn storage(module: &Module) -> &'static str {
        let tac = module.environment == Self::TAC;
        [Self::NO_STORAGE, Self::PAGE_STORAGE][usize::from(tac)]
    }

    fn methods(module: &Module) -> Vec<PlannedMethod> {
        module
            .types
            .iter()
            .flat_map(|type_declaration| {
                type_declaration
                    .methods
                    .iter()
                    .filter(|method| !method.metadata.is_empty())
                    .map(move |method| Self::planned_method(module, type_declaration, method))
            })
            .collect()
    }

    fn planned_method(module: &Module, type_declaration: &Type, method: &Method) -> PlannedMethod {
        PlannedMethod {
            owner: type_declaration.name.clone(),
            name: method.name.clone(),
            line: Self::line(module, method.signature.start),
            parameters: method
                .parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect(),
            metadata: method.metadata.clone(),
            declared: method.declared,
            asynchronous: method.asynchronous,
            mapped_argument: Self::mapped_argument(method),
        }
    }

    fn mapped_argument(method: &Method) -> Option<usize> {
        let policy = method.metadata.get("mapping")?;
        mapper::plan(policy).ok().map(|(argument, _)| argument)
    }

    fn types(module: &Module) -> Vec<PlannedType> {
        module
            .types
            .iter()
            .map(|type_declaration| PlannedType {
                name: type_declaration.name.clone(),
                line: Self::line(module, type_declaration.keyword.start),
                declared: type_declaration.declared,
                takes_context: Self::takes_context(type_declaration),
            })
            .collect()
    }

    fn takes_context(type_declaration: &Type) -> bool {
        type_declaration
            .constructor
            .as_ref()
            .is_some_and(|fields| fields.iter().any(|field| field == Self::CONTEXT))
    }

    fn line(module: &Module, offset: usize) -> usize {
        module
            .source
            .get(..offset)
            .map_or(0, |before| before.matches('\n').count())
    }
}

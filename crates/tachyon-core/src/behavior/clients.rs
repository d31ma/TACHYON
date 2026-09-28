use super::{Failure, Input, Registration};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(super) struct ClientRuntime;

impl ClientRuntime {
    pub(super) const SOURCE: &str = include_str!("client_runtime.js");
}

pub(super) fn plan(
    input: &Input<'_>,
    registrations: &mut [Registration],
    environment: &str,
    resources: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Option<String>, Failure> {
    let mut providers = BTreeSet::new();
    for item in registrations.iter() {
        if is_provider(item, environment) {
            providers.insert(item.name.clone());
        }
    }
    if providers.is_empty() {
        return Ok(None);
    }
    if environment != "tac" {
        return Err(input.failure(0, "Managed browser clients require Tac"));
    }
    let root = default_class_name(input, registrations)?;
    let mut graph = BTreeMap::new();
    let mut arguments = BTreeMap::new();
    let mut adapters = BTreeMap::new();
    for item in registrations.iter_mut() {
        if !providers.contains(&item.name) && item.name != root {
            continue;
        }
        if item.markers.contains("Client") {
            record_adapters(input, item, &mut adapters)?;
        }
        let is_root = item.name == root;
        let (dependencies, call) =
            client_dependencies(input, item, &providers, resources.get(&item.name), is_root)?;
        arguments.insert(item.name.clone(), call);
        graph.insert(item.name.clone(), dependencies);
    }
    let mut checked = BTreeSet::new();
    for name in graph.keys() {
        acyclic(input, name, &graph, &mut BTreeSet::new(), &mut checked)?;
    }
    factory(input, registrations, &root, &graph, &arguments, &adapters).map(Some)
}

fn is_provider(item: &Registration, environment: &str) -> bool {
    if item.markers.contains("Client") {
        return true;
    }
    if environment != "tac" {
        return false;
    }
    item.markers.contains("Helper") || item.data.get("repositoryRole").is_some()
}

fn default_class_name(
    input: &Input<'_>,
    registrations: &[Registration],
) -> Result<String, Failure> {
    for item in registrations {
        if super::identifier(&item.name) && is_default_export(input, item) {
            return Ok(item.name.clone());
        }
    }
    Err(input.failure(0, "Tac client injection requires a named default class"))
}

fn is_default_export(input: &Input<'_>, item: &Registration) -> bool {
    let last_two_words = input.code[..item.start].split_whitespace().rev().take(2);
    last_two_words.eq(["default", "export"])
}

fn record_adapters(
    input: &Input<'_>,
    item: &Registration,
    adapters: &mut BTreeMap<String, String>,
) -> Result<(), Failure> {
    for name in client_adapters(input, item)? {
        let policy = item.data["clientPolicy"]
            .as_str()
            .unwrap_or("no-store")
            .to_owned();
        let previous = adapters.insert(name, policy.clone());
        if previous.is_some_and(|previous| previous != policy) {
            return Err(input.failure(
                item.start,
                "Clients that share one HTTP adapter must use the same cache policy",
            ));
        }
    }
    Ok(())
}

fn client_adapters(input: &Input<'_>, item: &Registration) -> Result<BTreeSet<String>, Failure> {
    let mut adapters = BTreeSet::new();
    for method in item.methods.values() {
        let client = method
            .get("httpExchange")
            .and_then(|exchange| exchange.get("client"))
            .and_then(serde_json::Value::as_str);
        if let Some(client) = client {
            adapters.insert(client.to_owned());
        }
    }
    let streams = item.data.get("clientStreams") == Some(&serde_json::Value::Bool(true));
    if adapters.is_empty() && !streams {
        return Err(input.failure(
            item.start,
            "An @HTTPClient class needs at least one method with @GetRequest, @PostRequest, @PutRequest, @PatchRequest, @DeleteRequest, @StreamRequest or @SocketRequest: @GetRequest means calling the method sends a GET request",
        ));
    }
    Ok(adapters)
}

fn client_dependencies(
    input: &Input<'_>,
    item: &mut Registration,
    providers: &BTreeSet<String>,
    resources: Option<&BTreeSet<String>>,
    is_root: bool,
) -> Result<(Vec<String>, String), Failure> {
    let members = super::direct_members(input, item);
    if members.iter().any(|(name, _)| name == "behaviors") {
        return Err(input.failure(item.start, "Managed Tac clients own their behavior context; do not declare a behaviors member on the managed client or root"));
    }
    let fields = injectable_fields(input, item, resources)?;
    if item.constructor.is_none() && !fields.is_empty() {
        return Err(input.failure(
            item.start,
            "Uninitialized managed client fields require NeedsAll",
        ));
    }
    let selected = selected_fields(item.constructor.as_ref(), &fields);
    if selected.len() > 32 {
        return Err(input.failure(
            item.start,
            "Tac Client constructors permit at most 32 dependencies",
        ));
    }
    let mut dependencies = Vec::new();
    let mut calls = Vec::new();
    for field in &selected {
        if is_resource(resources, field) {
            return Err(input.failure(
                item.start,
                format!("Needs does not select the '{field}' resource; the runtime supplies it"),
            ));
        }
        match field_provider(input, item, providers, &fields, field, is_root)? {
            Some(provider) => {
                calls.push(format!("resolve('{provider}')"));
                dependencies.push(provider);
            }
            None => calls.push(format!("properties?.['{field}']")),
        }
    }
    if item.constructor.is_some() || providers.contains(&item.name) {
        item.constructor = Some(selected);
    }
    Ok((dependencies, calls.join(",")))
}

fn is_resource(resources: Option<&BTreeSet<String>>, name: &String) -> bool {
    resources.is_some_and(|fields| fields.contains(name))
}

fn injectable_fields(
    input: &Input<'_>,
    item: &Registration,
    resources: Option<&BTreeSet<String>>,
) -> Result<Vec<(String, String)>, Failure> {
    let body = item.open + 1..item.close;
    let mut fields = Vec::new();
    for (name, field_type) in crate::yon::required_fields(input.path, input.source, body)? {
        if !is_resource(resources, &name) && !is_fake(item, &name) {
            fields.push((name, field_type));
        }
    }
    Ok(fields)
}

fn is_fake(item: &Registration, name: &str) -> bool {
    item.data
        .get("mockBeans")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|fakes| fakes.iter().any(|fake| fake.as_str() == Some(name)))
}

fn selected_fields(constructor: Option<&Vec<String>>, fields: &[(String, String)]) -> Vec<String> {
    match constructor {
        Some(selected) if selected.is_empty() => {
            fields.iter().map(|(name, _)| name.clone()).collect()
        }
        Some(selected) => selected.clone(),
        None => Vec::new(),
    }
}

fn field_provider(
    input: &Input<'_>,
    item: &Registration,
    providers: &BTreeSet<String>,
    fields: &[(String, String)],
    field: &str,
    is_root: bool,
) -> Result<Option<String>, Failure> {
    let (_, field_type) = fields
        .iter()
        .find(|(name, _)| name == field)
        .ok_or_else(|| {
            input.failure(
                item.start,
                "Client injection requires direct uninitialized fields",
            )
        })?;
    let qualifier = item
        .data
        .get("qualifiers")
        .and_then(|qualifiers| qualifiers.get(field))
        .and_then(serde_json::Value::as_str);
    let candidates = matching_providers(providers, qualifier, field_type, field);
    if is_root && qualifier.is_none() && candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() != 1 {
        return Err(input.failure(
            item.start,
            format!(
                "Client dependency '{field}' must identify one concrete Client in this feature"
            ),
        ));
    }
    Ok(Some(candidates[0].clone()))
}

fn matching_providers<'providers>(
    providers: &'providers BTreeSet<String>,
    qualifier: Option<&str>,
    field_type: &str,
    field: &str,
) -> Vec<&'providers String> {
    let mut candidates = Vec::new();
    for name in providers {
        let matches = match qualifier {
            Some(provider) => name.as_str() == provider,
            None if field_type.is_empty() => lower_camel(name) == field,
            None => field_type == name,
        };
        if matches {
            candidates.push(name);
        }
    }
    candidates
}

fn factory(
    input: &Input<'_>,
    registrations: &[Registration],
    root: &str,
    graph: &BTreeMap<String, Vec<String>>,
    calls: &BTreeMap<String, String>,
    adapters: &BTreeMap<String, String>,
) -> Result<String, Failure> {
    let mut cases = String::new();
    for name in graph.keys() {
        let Some(registration) = registrations.iter().find(|item| &item.name == name) else {
            return Err(input.failure(0, "Managed client graph is incomplete"));
        };
        let arguments = if name == root && registration.constructor.is_none() {
            String::from("properties")
        } else {
            calls.get(name).cloned().unwrap_or_default()
        };
        let hidden = hidden_fields(registration);
        let _ = write!(
            cases,
            "case '{name}': instance = new {name}({arguments}); {hidden} break;"
        );
    }
    let adapters = serde_json::to_string(adapters)
        .map_err(|_| input.failure(0, "Cannot encode client adapters"))?;
    Ok(owner_factory_source(&cases, root, &adapters))
}

fn owner_factory_source(cases: &str, root: &str, adapters: &str) -> String {
    format!(
        r"
const __tachyon_owner_graphs__ = new WeakMap();
export function __tachyon_event_owners__(root) {{ return __tachyon_owner_graphs__.get(root) || [root]; }}
export async function __tachyon_create_owner__(properties, signal, transport) {{
  __tachyon_ClientConfiguration.bindTransport(transport);
  const context = await Behaviors.create(__tachyon_ClientConfiguration, {{partition:'tac-client',signal}});
  const instances = new Map();
  function resolve(name) {{
    if (instances.has(name)) return instances.get(name);
    let instance;
    switch(name) {{ {cases} default: throw new TypeError('Unknown managed Tac client.'); }}
    Object.defineProperty(instance, 'behaviors', {{value:context, configurable:true}});
    instances.set(name,instance);
    return instance;
  }}
  try {{
    const root = resolve('{root}');
    __tachyon_owner_graphs__.set(root, Object.freeze([...instances.values()]));
    return root;
  }} catch(error) {{ context.dispose(); throw error; }}
}}
__tachyon_ClientConfiguration.adapters = {adapters};
"
    )
}

fn hidden_fields(registration: &Registration) -> String {
    let mut hidden = String::new();
    for field in registration.constructor.iter().flatten() {
        let _ = write!(
            hidden,
            "Object.defineProperty(instance,'{field}',{{enumerable:false}});"
        );
    }
    hidden
}

fn acyclic(
    input: &Input<'_>,
    name: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    checked: &mut BTreeSet<String>,
) -> Result<(), Failure> {
    if checked.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_owned()) {
        return Err(input.failure(0, "Tac Client dependencies contain a cycle"));
    }
    for dependency in &graph[name] {
        acyclic(input, dependency, graph, visiting, checked)?;
    }
    visiting.remove(name);
    checked.insert(name.to_owned());
    Ok(())
}

fn lower_camel(name: &str) -> String {
    let uppercase_count = name.chars().take_while(char::is_ascii_uppercase).count();
    let end = if uppercase_count > 1 && uppercase_count < name.len() {
        uppercase_count - 1
    } else {
        uppercase_count.max(1)
    };
    format!("{}{}", name[..end].to_ascii_lowercase(), &name[end..])
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use std::path::Path;

    fn lower_file(
        file_name: &str,
        source: &str,
        environment: &str,
    ) -> Result<Option<String>, crate::Failure> {
        super::super::lower(Path::new(file_name), source, environment)
    }

    fn lower_tac_js(source: &str) -> Result<Option<String>, crate::Failure> {
        lower_file("tac.js", source, "tac")
    }

    fn lowered_tac_js(source: &str) -> String {
        lower_tac_js(source).unwrap().unwrap()
    }

    #[test]
    fn a_test_unit_fakes_a_client_without_needing_it() {
        let source = "@Setup\nclass HelloTestSetup {\n  static configure() { return {}; }\n}\n@HTTPClient\nclass GreetingClient {\n  @GetRequest(\"/api/greeting\")\n  async getGreeting() {}\n}\n@Test(HelloTestSetup)\nexport default class CounterServiceTest {\n  @Fake\n  greetingClient;\n\n  async testAsks() {}\n}\n";
        let lowered = lower_tac_js(source);
        assert!(lowered.is_ok(), "{lowered:?}");
    }

    #[test]
    fn tac_providers_are_clients_bare_repositories_and_helpers() {
        let source = "@Store()\nclass BasketRepository {\nitems = [];\n}\n@Helper()\nclass PricingHelper {\ntotal(items) { return items.length; }\n}\n@Client()\nclass ProductsClient {\n@WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/api/products\"})\nasync all() {return {}}\n}\n@Service()\n@NeedsAll()\nexport default class BasketService {\nbasketRepository;\npricingHelper;\nproductsClient;\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains(
                "new BasketService(resolve('BasketRepository'),resolve('PricingHelper'),resolve('ProductsClient'))"
            ),
            "{lowered}"
        );
        assert!(
            lowered.contains("constructor(basketRepository, pricingHelper, productsClient)"),
            "{lowered}"
        );
        assert!(lowered.contains("case 'BasketRepository': instance = new BasketRepository()"));
        assert!(lowered.contains("case 'PricingHelper': instance = new PricingHelper()"));
        assert!(!lowered.contains("@Store") && !lowered.contains("@Helper"));
        let queried = source.replace(
            "@Store()",
            "@Store(BasketEntity, {\"fields\":[\"id\"],\"defaultSize\":20,\"maxSize\":100})",
        );
        assert!(lower_tac_js(&queried).is_err());
        assert!(
            lower_tac_js(&source.replace("@Helper()", "@Helper({\"name\":\"pricing\"})")).is_err()
        );
    }

    #[test]
    fn tac_inject_binds_provider_types_to_lower_camel_fields() {
        let source = "@Store()\nclass BasketRepository {}\n@Helper()\nclass PricingHelper {}\n@Service()\n@Needs(BasketRepository, PricingHelper)\nexport default class BasketService {\nbasketRepository;\npricingHelper;\ntotal() { return this.pricingHelper; }\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains("constructor(basketRepository, pricingHelper)"),
            "{lowered}"
        );
        assert!(
            lowered.contains(
                "new BasketService(resolve('BasketRepository'),resolve('PricingHelper'))"
            ),
            "{lowered}"
        );
        assert!(!lowered.contains("@Needs"), "{lowered}");
        let typed = "@Helper()\nclass PricingHelper {}\n@Service()\n@Needs(PricingHelper)\nexport default class BasketService {\npricing!: PricingHelper;\n}";
        assert!(
            lower_file("tac.ts", typed, "tac").is_err(),
            "Needs names the lower-camel field of its provider type"
        );
        let bound = "@Helper()\nclass PricingHelper {}\n@Service()\n@Needs(PricingHelper)\nexport default class BasketService {\npricingHelper!: PricingHelper;\n}";
        assert!(
            lower_file("tac.ts", bound, "tac")
                .unwrap()
                .unwrap()
                .contains("constructor(pricingHelper)")
        );
        for arguments in ["", "'BasketRepository'", "Basket Repository", "1"] {
            let invalid = source.replace(
                "@Needs(BasketRepository, PricingHelper)",
                &format!("@Needs({arguments})"),
            );
            assert!(lower_tac_js(&invalid).is_err(), "{arguments}");
        }
    }

    fn products_source() -> &'static str {
        "@Client()\nclass ProductsClient {\n@WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/api/products\"})\nasync all() {return {}}\n}\n@NeedsAll()\nexport default class Products {\nproductsClient;\n}"
    }

    #[test]
    fn constructs_required_client_fields() {
        let source = products_source();
        let lowered = lowered_tac_js(source);
        assert!(lowered.contains("new Products(resolve('ProductsClient'))"));
        assert!(lowered.contains("constructor(productsClient)"));
        assert!(lowered.contains("export async function __tachyon_create_owner__"));
    }

    #[test]
    fn an_http_client_lowers_like_a_client_with_request_methods() {
        let source = "@HTTPClient({\"path\":\"/api/products\",\"adapter\":\"api\",\"cache\":\"network-first\"})\nclass ProductsClient {\n@GetRequest(\"/{id}\")\nasync find(@PathVariable id) {}\n}\n@NeedsAll()\nexport default class Products {\nproductsClient;\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains("new Products(resolve('ProductsClient'))"),
            "{lowered}"
        );
        assert!(lowered.contains("\"api\":\"network-first\""), "{lowered}");
        assert!(
            lowered.contains("\"pathVariables\":{\"id\":0}"),
            "{lowered}"
        );
        assert!(!lowered.contains("@PathVariable"), "{lowered}");
    }

    #[test]
    fn a_root_field_that_names_no_client_comes_from_the_props() {
        let source = "@HTTPClient({\"adapter\":\"api\"})\nclass ProductsClient {\n@GetRequest(\"/api/products\")\nasync all() {}\n}\n@NeedsAll()\nexport default class Products {\nproductsClient;\nopen;\nlabel = 'Menu';\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains("new Products(resolve('ProductsClient'),properties?.['open'])"),
            "{lowered}"
        );
    }

    #[test]
    fn client_cache_policies_reach_the_adapters() {
        let source = products_source();
        for policy in ["cache-first", "network-first", "reload", "no-store"] {
            let cached =
                source.replace("@Client()", &format!("@Client({{\"cache\":\"{policy}\"}})"));
            let output = lowered_tac_js(&cached);
            assert!(output.contains(&format!("\"api\":\"{policy}\"")));
        }
    }

    #[test]
    fn rejects_invalid_client_options_and_cycles_and_reads_other_fields_from_props() {
        let source = products_source();
        for options in [
            "{\"cache\":null}",
            "{\"cache\":true}",
            "{\"cache\":\"unknown\"}",
            "{\"key\":\"alias\"}",
        ] {
            assert!(
                lower_tac_js(&source.replace("@Client()", &format!("@Client({options})"))).is_err()
            );
        }
        let cycle = source.replace(
            "class ProductsClient {",
            "@NeedsAll()\nclass ProductsClient {\nproductsClient;\n",
        );
        assert!(lower_tac_js(&cycle).is_err());
        let prop = lowered_tac_js(&source.replace("productsClient;", "unknown;"));
        assert!(prop.contains("properties?.['unknown']"), "{prop}");
    }

    #[test]
    fn managed_clients_preserve_defaults_and_fail_closed_at_language_boundaries() {
        let source = "@Client()\nclass HTTPClient {\n@WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/api\"})\nasync all() {return {}}\n}\n@NeedsAll()\nexport default class Page {\nhttpClient;\nitems = [];\nstatic version = 1;\n}";
        let lowered = lowered_tac_js(source);
        assert!(lowered.contains("constructor(httpClient)"));
        assert!(lowered.contains("items = []"));
        for invalid in [
            source.replace("class Page", "class"),
            source.replace("httpClient;", "httpClient;\nconstructor() {}"),
            source.replace(
                "@WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/api\"})",
                "",
            ),
        ] {
            assert!(lower_tac_js(&invalid).is_err());
        }
        for (extension, annotation) in [
            ("swift", "@Client()"),
            ("kt", "@Client()"),
            ("cs", "[Client()]"),
            ("rs", "#[tac::client]"),
        ] {
            let source = format!("{annotation}\nclass HTTPClient {{}}");
            assert!(lower_file(&format!("tac.{extension}"), &source, "tac").is_err());
        }
        let yon = lower_file("yon.js", source, "yon").unwrap().unwrap();
        assert!(
            yon.contains("@Client()"),
            "Yon role handling stays in the Yon frontend"
        );
    }

    fn browser_client_source(extension: &str) -> String {
        let field = if extension == "ts" {
            "productsClient: ProductsClient;"
        } else {
            "productsClient;"
        };
        format!(
            "@Client()\nclass ProductsClient {{\n@WebCall({{\"client\":\"api\",\"method\":\"GET\",\"path\":\"/products\"}})\nasync all(mode) {{return {{query:{{mode}}}}}}\n}}\n@NeedsAll()\nexport default class Products {{\n{field}\n}}"
        )
    }

    async fn run_client_contract(extension: &str, module_source: String) -> std::process::Output {
        let directory = tempfile::tempdir().unwrap();
        let module = directory.path().join("client.mjs");
        let contract = directory.path().join("contract.mjs");
        std::fs::write(&module, module_source).unwrap();
        std::fs::write(&contract, include_str!("client_contract.mjs")).unwrap();
        let mut command = tokio::process::Command::new("node");
        command
            .args([
                contract.as_os_str(),
                module.as_os_str(),
                std::ffi::OsStr::new(extension),
            ])
            .kill_on_drop(true);
        tokio::time::timeout(std::time::Duration::from_secs(30), command.output())
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn browser_clients_execute_real_http_with_owned_cancellation() {
        for extension in ["js", "ts"] {
            let source = browser_client_source(extension);
            let mut lowered = lower_file(&format!("tac.{extension}"), &source, "tac")
                .unwrap()
                .unwrap();
            lowered.push_str(include_str!("runtime.js"));
            let output = run_client_contract(extension, lowered).await;
            assert!(
                output.status.success(),
                "{extension}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("managed-client-contract:passed")
            );
        }
    }
}

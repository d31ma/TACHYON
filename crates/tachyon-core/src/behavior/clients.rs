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
    providers.extend(needed_contracts(
        input,
        registrations,
        environment,
        resources,
    )?);
    if providers.is_empty() {
        return Ok(None);
    }
    if environment != "tac" {
        return Err(input.failure(0, "Managed browser clients require Tac"));
    }
    let root = default_class_name(input, registrations)?;
    let mut graph = BTreeMap::new();
    let mut arguments = BTreeMap::new();
    let mut adapters = BTreeSet::new();
    for item in registrations.iter_mut() {
        if !providers.contains(&item.name) && item.name != root {
            continue;
        }
        if item.markers.contains("Client") {
            adapters.extend(client_adapters(input, item)?);
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
    item.markers.contains("Delegate")
}

fn needed_contracts(
    input: &Input<'_>,
    registrations: &[Registration],
    environment: &str,
    resources: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeSet<String>, Failure> {
    let contracts: BTreeSet<String> = registrations
        .iter()
        .filter(|item| {
            ["repositoryRole", "mapper"]
                .iter()
                .any(|key| item.data.contains_key(*key))
        })
        .map(|item| item.name.clone())
        .collect();
    let mut needed = BTreeSet::new();
    if environment != "tac" || contracts.is_empty() {
        return Ok(needed);
    }
    for item in registrations
        .iter()
        .filter(|item| item.constructor.is_some())
    {
        let qualifiers = item.data.get("qualifiers");
        for (field, field_type) in injectable_fields(input, item, resources.get(&item.name))? {
            let qualifier = qualifiers
                .and_then(|qualifiers| qualifiers.get(&field))
                .and_then(serde_json::Value::as_str);
            let matched = matching_providers(&contracts, qualifier, &field_type, &field);
            needed.extend(matched.into_iter().cloned());
        }
    }
    Ok(needed)
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
            None => calls.push(
                [
                    String::from("undefined"),
                    format!("properties?.['{field}']"),
                ][usize::from(is_root)]
                .clone(),
            ),
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
    let unnamed = [qualifier.is_none(), candidates.is_empty()] == [true, true];
    let filled_by_the_page = [is_root, item.markers.contains("Delegate")].contains(&true);
    if [unnamed, filled_by_the_page] == [true, true] {
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
    adapters: &BTreeSet<String>,
) -> Result<String, Failure> {
    let mut cases = String::new();
    let mut stores = Vec::new();
    let mut owning = Vec::new();
    for name in graph.keys() {
        let Some(registration) = registrations.iter().find(|item| &item.name == name) else {
            return Err(input.failure(0, "Managed client graph is incomplete"));
        };
        if registration.data.contains_key("repositoryRole") {
            stores.push(format!("['{name}',{name}]"));
        }
        if super::own_context::OwnContext::opened_by(input, registration) {
            owning.push(name.as_str());
        }
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
    let stores = format!("[{}]", stores.join(","));
    let owning = serde_json::to_string(&owning)
        .map_err(|_| input.failure(0, "Cannot encode own behavior contexts"))?;
    Ok(owner_factory_source(
        &cases, root, &adapters, &stores, &owning,
    ))
}

fn owner_factory_source(
    cases: &str,
    root: &str,
    adapters: &str,
    stores: &str,
    owning: &str,
) -> String {
    format!(
        r"
const __tachyon_owner_graphs__ = new WeakMap();
export function __tachyon_event_owners__(root) {{ return __tachyon_owner_graphs__.get(root) || [root]; }}
export async function __tachyon_create_owner__(properties, signal, transport) {{
  __tachyon_ClientConfiguration.bindTransport(transport);
  const context = await Behaviors.create(__tachyon_ClientConfiguration, {{partition:'tac-client',signal}});
  const stores = new Map();
  const instances = new Map();
  function resolve(name) {{
    if (instances.has(name)) return instances.get(name);
    let instance;
    switch(name) {{ {cases} default: throw new TypeError('Unknown managed Tac client.'); }}
    if ({owning}.includes(name)) Behaviors.expectOwnContext(instance, signal);
    else Object.defineProperty(instance, 'behaviors', {{value:stores.get(name) || context, configurable:true}});
    instances.set(name,instance);
    return instance;
  }}
  try {{
    for (const [name, store] of {stores}) stores.set(name, await store.{context_method}(signal));
    const root = resolve('{root}');
    __tachyon_owner_graphs__.set(root, Object.freeze([...instances.values()]));
    return root;
  }} catch(error) {{ context.dispose(); for (const opened of stores.values()) opened.dispose(); throw error; }}
}}
__tachyon_ClientConfiguration.adapters = {adapters};
",
        context_method = crate::declared_contract::DeclaredContract::STORE_CONTEXT,
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
        let source = "@Setup\nclass HelloTestSetup {\n  static configure() { return {}; }\n}\n@HTTPClient\nclass GreetingClient {\n  @GetRequest(\"/api/greeting\")\n  async getGreeting() {}\n}\n@Test(HelloTestSetup)\nexport default class CounterServiceTest {\n  @Fake\n  greetingClient;\n\n  async testAsks() { Assert.isTrue(Boolean(this.greetingClient)); }\n}\n";
        let lowered = lower_tac_js(source);
        assert!(lowered.is_ok(), "{lowered:?}");
    }

    #[test]
    fn tac_providers_are_clients_bare_repositories_and_helpers() {
        let source = "@Store()\nclass BasketRepository {\nitems = [];\n}\n@Delegate()\nclass PricingDelegate {\ntotal(items) { return items.length; }\n}\n@Client()\nclass ProductsClient {\n@WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/api/products\"})\nasync all() {return {}}\n}\n@Service()\n@NeedsAll()\nexport default class BasketService {\nbasketRepository;\npricingDelegate;\nproductsClient;\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains(
                "new BasketService(resolve('BasketRepository'),resolve('PricingDelegate'),resolve('ProductsClient'))"
            ),
            "{lowered}"
        );
        assert!(
            lowered.contains("constructor(basketRepository, pricingDelegate, productsClient)"),
            "{lowered}"
        );
        assert!(lowered.contains("case 'BasketRepository': instance = new BasketRepository()"));
        assert!(lowered.contains("case 'PricingDelegate': instance = new PricingDelegate()"));
        assert!(!lowered.contains("@Store") && !lowered.contains("@Delegate"));
        let queried = source.replace(
            "@Store()",
            "@Store(BasketEntity, {\"fields\":[\"id\"],\"defaultSize\":20,\"maxSize\":100})",
        );
        assert!(lower_tac_js(&queried).is_err());
        assert!(
            lower_tac_js(&source.replace("@Delegate()", "@Delegate({\"name\":\"pricing\"})"))
                .is_err()
        );
    }

    #[test]
    fn tac_inject_binds_provider_types_to_lower_camel_fields() {
        let source = "@Store()\nclass BasketRepository {}\n@Delegate()\nclass PricingDelegate {}\n@Service()\n@Needs(BasketRepository, PricingDelegate)\nexport default class BasketService {\nbasketRepository;\npricingDelegate;\ntotal() { return this.pricingDelegate; }\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains("constructor(basketRepository, pricingDelegate)"),
            "{lowered}"
        );
        assert!(
            lowered.contains(
                "new BasketService(resolve('BasketRepository'),resolve('PricingDelegate'))"
            ),
            "{lowered}"
        );
        assert!(!lowered.contains("@Needs"), "{lowered}");
        let typed = "@Delegate()\nclass PricingDelegate {}\n@Service()\n@Needs(PricingDelegate)\nexport default class BasketService {\npricing!: PricingDelegate;\n}";
        assert!(
            lower_file("tac.ts", typed, "tac").is_err(),
            "Needs names the lower-camel field of its provider type"
        );
        let bound = "@Delegate()\nclass PricingDelegate {}\n@Service()\n@Needs(PricingDelegate)\nexport default class BasketService {\npricingDelegate!: PricingDelegate;\n}";
        assert!(
            lower_file("tac.ts", bound, "tac")
                .unwrap()
                .unwrap()
                .contains("constructor(pricingDelegate)")
        );
        for arguments in ["", "'BasketRepository'", "Basket Repository", "1"] {
            let invalid = source.replace(
                "@Needs(BasketRepository, PricingDelegate)",
                &format!("@Needs({arguments})"),
            );
            assert!(lower_tac_js(&invalid).is_err(), "{arguments}");
        }
    }

    fn products_source() -> &'static str {
        "@Client()\nclass ProductsClient {\n@WebCall({\"client\":\"api\",\"method\":\"GET\",\"path\":\"/api/products\"})\nasync all() {return {}}\n}\n@NeedsAll()\nexport default class Products {\nproductsClient;\n}"
    }

    #[test]
    fn a_managed_root_that_opens_its_own_context_waits_for_it() {
        let source = products_source().replace(
            "productsClient;\n}",
            "productsClient;\nasync open() { this.behaviors = await Behaviors.create(Setup, {}); }\n}",
        );
        let lowered = lowered_tac_js(&source);
        assert!(
            lowered.contains(
                "if ([\"Products\"].includes(name)) Behaviors.expectOwnContext(instance, signal);"
            ),
            "{lowered}"
        );
        let plain = lowered_tac_js(products_source());
        assert!(
            plain.contains("if ([].includes(name)) Behaviors.expectOwnContext(instance, signal);"),
            "{plain}"
        );
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
        let source = "@HTTPClient({\"path\":\"/api/products\",\"adapter\":\"api\"})\nclass ProductsClient {\n@GetRequest(\"/{id}\")\nasync find(@PathVariable id) {}\n}\n@NeedsAll()\nexport default class Products {\nproductsClient;\n}";
        let lowered = lowered_tac_js(source);
        assert!(
            lowered.contains("new Products(resolve('ProductsClient'))"),
            "{lowered}"
        );
        assert!(lowered.contains("adapters = [\"api\"]"), "{lowered}");
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
    fn a_delegate_field_that_names_no_client_waits_for_the_page_service() {
        let source = "@Delegate()\n@NeedsAll()\nclass TaxDelegate {\ntax;\nrate(region) { return this.tax.rate(region) * 2; }\n}\n@Service()\n@NeedsAll()\nexport default class QuoteService {\ntaxDelegate;\nquote() { return this.taxDelegate.rate('north'); }\n}";
        let lowered = lowered_tac_js(source);
        assert!(lowered.contains("new TaxDelegate(undefined)"), "{lowered}");
        assert!(
            lowered.contains("new QuoteService(resolve('TaxDelegate'))"),
            "{lowered}"
        );
    }

    #[test]
    fn rejects_invalid_client_options_and_cycles_and_reads_other_fields_from_props() {
        let source = products_source();
        for options in [
            "{\"cache\":\"no-store\"}",
            "{\"streams\":true,\"cache\":\"cache-first\"}",
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
            assert!(lower_file(&format!("tac.{extension}"), &source, "tac").is_ok());
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
        let mut command = crate::node_driver::NodeDriver::async_command();
        command.args([
            contract.as_os_str(),
            module.as_os_str(),
            std::ffi::OsStr::new(extension),
        ]);
        crate::node_driver::NodeDriver::run_within(&mut command, std::time::Duration::from_secs(30))
            .await
            .unwrap()
    }

    #[test]
    fn a_public_read_with_credentials_is_never_kept_in_the_browser_cache() {
        let checks = "globalThis.location = new URL('http://127.0.0.1/')
const seen = []
const transport = async (url, init, options) => { seen.push([init.cache, options.publicRead, options.revalidate]); return new Response('{}', { headers: { 'content-type': 'application/json' } }) }
const request = (extra) => ({ method: 'GET', path: '/products', query: {}, headers: {}, publicRead: true, maxResponseBytes: 64, ...extra })
await __tachyon_client_exchange__(request({}), transport)
await __tachyon_client_exchange__(request({ revalidate: true }), transport)
await __tachyon_client_exchange__(request({ headers: { Authorization: 'Bearer secret' } }), transport)
await __tachyon_client_exchange__(request({ method: 'POST' }), transport)
console.log(JSON.stringify(seen))
";
        let directory = tempfile::tempdir().unwrap();
        let script = directory.path().join("cache-mode.mjs");
        std::fs::write(
            &script,
            format!("{}\n{checks}", include_str!("client_runtime.js")),
        )
        .unwrap();
        let mut command = crate::node_driver::NodeDriver::command();
        command.arg(script);
        let output = crate::node_driver::NodeDriver::output_within(
            &mut command,
            std::time::Duration::from_secs(20),
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            r#"[["default",true,false],["no-cache",true,true],["no-store",false,false],["no-store",false,false]]"#
        );
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

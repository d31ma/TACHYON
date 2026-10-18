use super::super::mapper::Plan;
use super::super::testing::Unit;
use super::java_test_construction::JavaTestConstruction;
use super::{Edit, Method, Module, Type, block, concrete_edit, declared_fields, render};
use java_typed_client::JavaTypedClient;
use serde_json::Value;
use std::fmt::Write as _;

mod java_typed_client;

pub(super) fn lower(module: &Module) -> Result<String, String> {
    reject_reserved_names(module)?;
    let mut edits = Vec::new();
    let mut sections = RuntimeSections::default();
    for type_declaration in &module.types {
        edits.extend(concrete_edit(module, type_declaration));
        lower_type(module, type_declaration, &mut sections, &mut edits)?;
    }
    let mut runtime = sections.fill_runtime(module);
    runtime.push_str(super::JavaBuiltins::NONE);
    runtime.push_str(super::super::testing::unit_helper(
        &module.types,
        super::super::TestAssert::JAVA,
    ));
    render(module, edits, &runtime)
}

#[derive(Default)]
struct RuntimeSections {
    configurations: String,
    entities: String,
    evictions: String,
    properties: String,
    values: String,
    selections: String,
    validations: String,
}

impl RuntimeSections {
    fn fill_runtime(&self, module: &Module) -> String {
        include_str!("java_runtime.java")
            .replace("__MODULE__", &module.module_id)
            .replace("__ENVIRONMENT__", &module.environment)
            .replace("__CONFIGURATIONS__", &self.configurations)
            .replace("__ENTITIES__", &self.entities)
            .replace("__EVICTIONS__", &self.evictions)
            .replace("__PROPERTIES__", &self.properties)
            .replace("__VALUES__", &self.values)
            .replace("__SELECTIONS__", &self.selections)
            .replace("__VALIDATIONS__", &self.validations)
    }
}

fn lower_type(
    module: &Module,
    type_declaration: &Type,
    sections: &mut RuntimeSections,
    edits: &mut Vec<Edit>,
) -> Result<(), String> {
    let validation = validation_policy(type_declaration)?;
    sections.validations.push_str(&validation);
    let properties = properties_case(type_declaration)?;
    sections.properties.push_str(&properties);
    let selection = selection_case(type_declaration)?;
    sections.selections.push_str(&selection);
    let values = value_bindings(type_declaration)?;
    sections.values.push_str(&values);
    let mut members = constructor(type_declaration)?;
    if type_declaration.configuration {
        check_configuration(module, type_declaration)?;
        let configuration = configuration_case(type_declaration);
        sections.configurations.push_str(&configuration);
    }
    if let Some(entity) = &type_declaration.entity {
        let entity = entity_case(type_declaration, entity)?;
        sections.entities.push_str(&entity);
    }
    for method in &type_declaration.methods {
        if method.metadata.is_empty() {
            continue;
        }
        let helper = lower_method(module, type_declaration, method, sections, edits)?;
        members.push_str(&helper);
    }
    if let Some(unit) = super::super::testing::polyglot_unit(type_declaration) {
        let cleared_fakes = JavaTestConstruction::cleared_fakes(type_declaration, &unit);
        members.push_str(&test_members(&unit, &cleared_fakes));
        members.push_str(&JavaTestConstruction::members(
            module,
            type_declaration,
            &unit,
        ));
    }
    if !members.is_empty() {
        edits.push(Edit {
            range: type_declaration.body.end..type_declaration.body.end,
            replacement: members,
        });
    }
    Ok(())
}

fn lower_method(
    module: &Module,
    type_declaration: &Type,
    method: &Method,
    sections: &mut RuntimeSections,
    edits: &mut Vec<Edit>,
) -> Result<String, String> {
    let (body, helper) = wrap(module, type_declaration, method)?;
    edits.push(Edit {
        range: method.body.clone(),
        replacement: block(method, &body),
    });
    if let Some(cacheable) = method.metadata.get("cacheable") {
        let eviction = eviction_case(type_declaration, method, cacheable)?;
        sections.evictions.push_str(&eviction);
    }
    Ok(helper)
}

fn check_configuration(module: &Module, type_declaration: &Type) -> Result<(), String> {
    let code = visible_code(&module.source[type_declaration.body.clone()]);
    if code.contains("static Behaviors.Configuration configure(Behaviors.Execution ") {
        return Ok(());
    }
    Err("Java configuration needs static Behaviors.Configuration configure(Behaviors.Execution execution)".to_owned())
}

fn configuration_case(type_declaration: &Type) -> String {
    format!(
        "case {}: return {}.configure(execution);",
        quote(&type_declaration.name),
        type_declaration.name
    )
}

fn entity_case(type_declaration: &Type, entity: &Value) -> Result<String, String> {
    Ok(format!(
        "case {}: return new Entity({}, {}, {}, {});",
        quote(&type_declaration.name),
        quote(&type_declaration.name),
        string(entity, "adapter")?,
        quote(
            type_declaration
                .table
                .as_deref()
                .ok_or("Entity requires Table")?
        ),
        string(entity, "id")?
    ))
}

fn eviction_case(
    type_declaration: &Type,
    method: &Method,
    cacheable: &Value,
) -> Result<String, String> {
    let cache = type_declaration
        .cache
        .as_ref()
        .ok_or("Cached requires Cache")?;
    let policy = policy(cache)?;
    let indices = indices(cacheable)?;
    let normalize = eviction_page(method)?;
    Ok(format!(
        "if (type.equals({}) && method.equals({})) {{ {normalize} Policy policy = {policy}; String key = cacheKey(type, method, normalized, {indices}, policy, execution, \"cache:evict\"); CacheAdapter adapter = configuration.caches.get(policy.name); if (adapter == null) throw fail(\"cache-adapter\"); try {{ adapter.delete(key, execution); }} catch (RuntimeException error) {{ throw fail(\"cache-adapter\"); }} execution.check(); return null; }}",
        quote(&type_declaration.name),
        quote(&method.name)
    ))
}

fn property_descriptor(name: &str, descriptor: &Value, path_key: &str) -> Result<String, String> {
    Ok(format!(
        "new Property({}, {}, {}, {}, {}, {}, {})",
        quote(name),
        string(descriptor, path_key)?,
        string(descriptor, "type")?,
        literal(descriptor.get("default"))?,
        descriptor.get("default").is_some(),
        literal(descriptor.get("min"))?,
        literal(descriptor.get("max"))?
    ))
}

fn value_bindings(type_declaration: &Type) -> Result<String, String> {
    let Some(policy) = type_declaration
        .policies
        .get("values")
        .and_then(Value::as_object)
    else {
        return Ok(String::new());
    };
    let mut descriptors = Vec::new();
    for (field, descriptor) in policy {
        descriptors.push(property_descriptor(field, descriptor, "path")?);
    }
    Ok(format!(
        "case {}: return bindValues(new Property[]{{{}}}, application);",
        quote(&type_declaration.name),
        descriptors.join(", ")
    ))
}

fn properties_case(type_declaration: &Type) -> Result<String, String> {
    let Some(policy) = type_declaration.policies.get("configurationProperties") else {
        return Ok(String::new());
    };
    let fields = policy
        .get("fields")
        .and_then(Value::as_object)
        .ok_or("Properties fields missing")?;
    let descriptors = fields
        .iter()
        .map(|(name, field)| property_descriptor(name, field, "name"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!(
        "case {}: return bind({}, new Property[]{{{}}}, values);",
        quote(&type_declaration.name),
        string(policy, "prefix")?,
        descriptors.join(", ")
    ))
}

fn selection_case(type_declaration: &Type) -> Result<String, String> {
    let policies = &type_declaration.policies;
    if !policies.contains_key("profiles") && !policies.contains_key("conditionalOnProperty") {
        return Ok(String::new());
    }
    let profiles = profile_literals(type_declaration)?;
    let condition = policies.get("conditionalOnProperty");
    let name = optional_string(condition, "name")?;
    let expected = optional_string(condition, "havingValue")?;
    let missing = condition
        .and_then(|value| value.get("matchIfMissing"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(format!(
        "case {}: return active(options, new String[]{{{}}}, {name}, {expected}, {missing});",
        quote(&type_declaration.name),
        profiles.join(", ")
    ))
}

fn profile_literals(type_declaration: &Type) -> Result<Vec<String>, String> {
    let Some(profiles) = type_declaration
        .policies
        .get("profiles")
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    let mut literals = Vec::new();
    for profile in profiles {
        literals.push(literal(Some(profile))?);
    }
    Ok(literals)
}

fn optional_string(value: Option<&Value>, key: &str) -> Result<String, String> {
    match value {
        Some(value) => string(value, key),
        None => Ok("null".to_owned()),
    }
}

fn literal(value: Option<&Value>) -> Result<String, String> {
    match value {
        None | Some(Value::Null) => Ok("null".to_owned()),
        Some(Value::String(value)) => Ok(quote(value)),
        Some(Value::Bool(value)) => Ok(value.to_string()),
        Some(Value::Number(value)) => match value.as_i64() {
            Some(number) => Ok(format!("{number}L")),
            None => Ok(value.to_string()),
        },
        _ => Err("Property metadata must contain scalar literals".to_owned()),
    }
}

fn validation_policy(type_declaration: &Type) -> Result<String, String> {
    let Some(schema) = type_declaration.policies.get("validated") else {
        return Ok(String::new());
    };
    let fields = schema
        .get("fields")
        .and_then(Value::as_object)
        .ok_or("Checked fields missing")?;
    let mut rules = Vec::new();
    for (name, rule) in fields {
        rules.push(validation_rule(name, rule)?);
    }
    Ok(format!(
        "case {}: return new Rule[]{{{}}};",
        quote(&type_declaration.name),
        rules.join(", ")
    ))
}

fn validation_rule(name: &str, rule: &Value) -> Result<String, String> {
    let size = rule.get("size");
    let size_minimum = size.and_then(|size| size.get("min"));
    let size_maximum = size.and_then(|size| size.get("max"));
    Ok(format!(
        "new Rule({}, {}, {}, {}, {}, {}, {})",
        quote(name),
        flag(rule, "notNull"),
        flag(rule, "notBlank"),
        literal(size_minimum)?,
        literal(size_maximum)?,
        literal(rule.get("min"))?,
        literal(rule.get("max"))?
    ))
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn visible_code(source: &str) -> String {
    let mask = crate::lexical::code_mask("java", source);
    let mut code = String::new();
    for (byte, visible) in source.bytes().zip(mask.code) {
        if visible {
            code.push(char::from(byte));
        } else {
            code.push(' ');
        }
    }
    code
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn reject_reserved_names(module: &Module) -> Result<(), String> {
    let code = visible_code(&module.source);
    for identifier in code.split(|character: char| !is_identifier_character(character)) {
        if identifier.starts_with("__ty_") {
            return Err(
                "Java identifiers beginning __ty_ are reserved for compiler-owned behavior lowering"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn eviction_page(method: &Method) -> Result<String, String> {
    let mut source = format!(
        "if (arguments.length != {}) throw fail(\"cache-key\"); Object[] normalized = arguments.clone();",
        method.parameters.len()
    );
    let Some(page) = method.metadata.get("pageable") else {
        return Ok(source);
    };
    let argument = number(page, "argument")?;
    let _ = write!(
        source,
        "normalized[{argument}] = Behaviors.page(normalized[{argument}], {}, {}, new String[]{{{}}});",
        number(page, "defaultSize")?,
        number(page, "maxSize")?,
        strings(page, "sort")?
    );
    Ok(source)
}

fn constructor(type_declaration: &Type) -> Result<String, String> {
    let Some(fields) = &type_declaration.constructor else {
        return Ok(String::new());
    };
    let mut parameters = Vec::new();
    let mut assignments = String::new();
    let declarations = declared_fields(type_declaration, |field| {
        format!("\nfinal {} {};", field.type_source, field.name)
    });
    for name in fields {
        let field = type_declaration
            .fields
            .iter()
            .find(|field| &field.name == name)
            .ok_or("Needs needs explicit typed fields")?;
        if field.type_source.trim().is_empty() {
            return Err("Needs field needs a concrete type".to_owned());
        }
        parameters.push(format!("{} {name}", field.type_source));
        let _ = write!(assignments, "this.{name} = {name};");
    }
    Ok(format!(
        "{declarations}\npublic {}({}) {{ {assignments} }}\n",
        type_declaration.name,
        parameters.join(", ")
    ))
}

fn validate_signature(module: &Module, method: &Method) -> Result<(), String> {
    if method.is_static || method.asynchronous {
        return Err(
            "Java behavior methods must be synchronous concrete instance methods".to_owned(),
        );
    }
    let signature = &module.source[method.signature.clone()];
    if signature.contains("throws ") || signature.contains("synchronized ") {
        return Err(
            "Behavior methods cannot declare checked exceptions or synchronized execution"
                .to_owned(),
        );
    }
    Ok(())
}

fn wrap(
    module: &Module,
    type_declaration: &Type,
    method: &Method,
) -> Result<(String, String), String> {
    validate_signature(module, method)?;
    if let Some(policy) = method.metadata.get("gateway") {
        return Ok((gateway_body(method, policy), String::new()));
    }
    if let Some(query) = method.metadata.get("query") {
        return Ok((query_body(type_declaration, method, query)?, String::new()));
    }
    if let Some(policy) = method.metadata.get("mapping") {
        return Ok((mapping_body(method, policy)?, String::new()));
    }
    let policies = policies(method)?;
    let managed = is_managed(method);
    let resilient = policies.resilient();
    validate_managed(
        type_declaration,
        method,
        managed,
        resilient,
        &module.environment,
    )?;
    let declared = declared_exchange(method);
    let (helper_name, helper) = if declared {
        (String::new(), String::new())
    } else {
        helper(module, method)?
    };
    let mut arguments = parameter_names(method);
    let mut prefix = valid_arguments(method, &mut arguments)?;
    pagination_prologue(method, &mut arguments, &mut prefix)?;
    let call = if declared {
        String::from("new java.util.LinkedHashMap<String, Object>()")
    } else {
        format!("{helper_name}({})", arguments.join(", "))
    };
    let body = if managed || resilient {
        let wrapped = managed_body(type_declaration, method, &policies, &arguments, &call)?;
        JavaTypedClient::body(method, &module.environment, &wrapped)?
    } else {
        plain_body(method, &call)
    };
    Ok((format!("{prefix}{body}"), helper))
}

fn gateway_body(method: &Method, policy: &Value) -> String {
    let key = policy["key"].as_str().unwrap_or_default();
    format!(
        "return Behaviors.gatewayResult(YonGateway.call({}, new Object[]{{{}}}));",
        quote(key),
        parameter_names(method).join(", ")
    )
}

fn is_managed(method: &Method) -> bool {
    method.metadata.contains_key("cacheable")
        || method.metadata.contains_key("httpExchange")
        || mutation_metadata(method).is_some()
}

fn mutation_metadata(method: &Method) -> Option<&Value> {
    method
        .metadata
        .get("cachePut")
        .or_else(|| method.metadata.get("cacheEvict"))
}

fn parameter_names(method: &Method) -> Vec<String> {
    let mut names = Vec::new();
    for parameter in &method.parameters {
        names.push(parameter.name.clone());
    }
    names
}

fn plain_body(method: &Method, call: &str) -> String {
    if method.metadata.contains_key("valid") {
        return format!("return Behaviors.copy({call}, 1048576);");
    }
    if method.return_type.trim() == "void" {
        return format!("{call};");
    }
    format!("return {call};")
}

fn managed_body(
    type_declaration: &Type,
    method: &Method,
    policies: &Policies,
    arguments: &[String],
    call: &str,
) -> Result<String, String> {
    let cached = method.metadata.get("cacheable");
    let policy = cache_policy_when_cached(type_declaration, cached)?;
    let selected = match cached {
        Some(cacheable) => indices(cacheable)?,
        None => "new int[]{}".to_owned(),
    };
    let exchange = exchange_policy(method.metadata.get("httpExchange"))?;
    if let Some(metadata) = mutation_metadata(method) {
        return mutation_body(
            type_declaration,
            method,
            metadata,
            &exchange,
            policies,
            call,
        );
    }
    let Policies {
        retry,
        limit,
        transactional,
    } = policies;
    Ok(format!(
        "return this.behaviors.invoke({}, {}, new Object[]{{{}}}, {selected}, {policy}, {exchange}, {retry}, {limit}, {transactional}, () -> {call});",
        quote(&type_declaration.name),
        quote(&method.name),
        arguments.join(", ")
    ))
}

fn cache_policy_when_cached(
    type_declaration: &Type,
    cached: Option<&Value>,
) -> Result<String, String> {
    if cached.is_none() {
        return Ok("null".to_owned());
    }
    match &type_declaration.cache {
        Some(cache) => policy(cache),
        None => Ok("null".to_owned()),
    }
}

fn pagination_prologue(
    method: &Method,
    arguments: &mut [String],
    prefix: &mut String,
) -> Result<(), String> {
    let Some(pageable) = method.metadata.get("pageable") else {
        return Ok(());
    };
    let index = usize::try_from(number(pageable, "argument")?)
        .map_err(|_| "Pageable argument is too large")?;
    let parameter = method
        .parameters
        .get(index)
        .ok_or("Pageable argument is outside method parameters")?;
    let normalized_type = parameter.type_source.replace(' ', "");
    if !matches!(
        normalized_type.as_str(),
        "Map<String,Object>" | "java.util.Map<String,Object>" | "Object"
    ) {
        return Err("Java Pageable argument must be Object or Map<String, Object>".to_owned());
    }
    let _ = writeln!(
        prefix,
        "java.util.Map<String, Object> __ty_page = Behaviors.page({}, {}, {}, new String[]{{{}}});",
        arguments[index],
        number(pageable, "defaultSize")?,
        number(pageable, "maxSize")?,
        strings(pageable, "sort")?
    );
    "__ty_page".clone_into(&mut arguments[index]);
    Ok(())
}

fn valid_arguments(method: &Method, arguments: &mut [String]) -> Result<String, String> {
    let Some(valid) = method.metadata.get("valid") else {
        return Ok(String::new());
    };
    if !returns_object(method) {
        return Err("Java Check methods must return Object bounded JSON".to_owned());
    }
    let index =
        usize::try_from(number(valid, "argument")?).map_err(|_| "Check argument too large")?;
    let parameter = method
        .parameters
        .get(index)
        .ok_or("Check argument out of bounds")?;
    let normalized_type = parameter.type_source.replace(' ', "");
    if !matches!(
        normalized_type.as_str(),
        "Map<String,Object>" | "java.util.Map<String,Object>" | "Object" | "java.lang.Object"
    ) {
        return Err("Java Check argument must be Object or Map<String, Object>".to_owned());
    }
    let prefix = format!(
        "java.util.Map<String, Object> __ty_valid = Behaviors.check({}, {});",
        string(valid, "type")?,
        parameter.name
    );
    "__ty_valid".clone_into(&mut arguments[index]);
    Ok(prefix)
}

fn exchange_policy(exchange: Option<&Value>) -> Result<String, String> {
    let Some(value) = exchange else {
        return Ok("null".to_owned());
    };
    Ok(format!(
        "new Behaviors.Exchange({}, {}, {}, {}, {}, {}, {})",
        string(value, "client")?,
        string(value, "method")?,
        string(value, "path")?,
        argument_pairs(value, "query"),
        argument_pairs(value, "headers"),
        value.get("body").and_then(Value::as_i64).unwrap_or(-1),
        argument_pairs(value, "pathVariables")
    ))
}

fn argument_pairs(exchange: &Value, key: &str) -> String {
    let pairs = argument_pair_list(exchange, key);
    format!("new Object[][]{{{pairs}}}")
}

fn argument_pair_list(exchange: &Value, key: &str) -> String {
    let Some(entries) = exchange.get(key).and_then(Value::as_object) else {
        return String::new();
    };
    let mut pairs = Vec::new();
    for (name, index) in entries {
        pairs.push(format!(
            "new Object[]{{{}, {}}}",
            quote(name),
            index.as_i64().unwrap_or_default()
        ));
    }
    pairs.join(", ")
}

fn mutation_body(
    type_declaration: &Type,
    method: &Method,
    metadata: &Value,
    exchange: &str,
    policies: &Policies,
    call: &str,
) -> Result<String, String> {
    let Policies {
        retry,
        limit,
        transactional,
    } = policies;
    let target = metadata
        .get("target")
        .ok_or("Cache mutation requires a resolved target")?;
    let cacheable_target = target
        .get("cacheable")
        .ok_or("Cache mutation requires Cached target")?;
    let selected = indices(cacheable_target)?;
    let mapped = mutation_arguments(method, metadata)?;
    let mut prefix = format!(
        "Object[] __ty_target = new Object[]{{{}}};",
        mapped.join(", ")
    );
    prefix.push_str(&target_page(target)?);
    let cache = type_declaration
        .cache
        .as_ref()
        .ok_or("Cache mutation requires Cache")?;
    let policy = policy(cache)?;
    let put = method.metadata.contains_key("cachePut");
    Ok(format!(
        "{prefix}return this.behaviors.mutate({}, {}, __ty_target, {selected}, {policy}, {put}, {exchange}, {retry}, {limit}, {transactional}, () -> {call});",
        quote(&type_declaration.name),
        string(target, "method")?
    ))
}

fn target_page(target: &Value) -> Result<String, String> {
    let Some(page) = target.get("pageable") else {
        return Ok(String::new());
    };
    let index = number(page, "argument")?;
    Ok(format!(
        "__ty_target[{index}] = Behaviors.page(__ty_target[{index}], {}, {}, new String[]{{{}}});",
        number(page, "defaultSize")?,
        number(page, "maxSize")?,
        strings(page, "sort")?
    ))
}

fn mutation_arguments(method: &Method, metadata: &Value) -> Result<Vec<String>, String> {
    let indexes = metadata
        .get("arguments")
        .and_then(Value::as_array)
        .ok_or("Cache mutation requires argument mapping")?;
    let valid_argument = method
        .metadata
        .get("valid")
        .and_then(|valid| valid.get("argument"))
        .and_then(Value::as_u64);
    let mut names = Vec::new();
    for index in indexes {
        names.push(mutation_argument(method, index, valid_argument)?);
    }
    Ok(names)
}

fn mutation_argument(
    method: &Method,
    index: &Value,
    valid_argument: Option<u64>,
) -> Result<String, String> {
    let index = usize::try_from(index.as_u64().ok_or("Invalid mutation argument")?)
        .map_err(|_| "Invalid mutation argument")?;
    let Some(parameter) = method.parameters.get(index) else {
        return Err("Mutation argument out of bounds".to_owned());
    };
    if valid_argument == u64::try_from(index).ok() {
        return Ok("__ty_valid".to_owned());
    }
    Ok(parameter.name.clone())
}

fn mapping_body(method: &Method, policy: &Value) -> Result<String, String> {
    let result = policy.get("result").and_then(Value::as_str);
    if result.is_none() && !returns_object(method) {
        return Err("Java mapping methods must return Object bounded JSON".to_owned());
    }
    let (argument, fields) = super::super::mapper::plan(policy)?;
    let value = method
        .parameters
        .get(argument)
        .ok_or("Convert reads an argument the method does not declare")?;
    let mut entries = Vec::new();
    for field in &fields {
        entries.push(mapping_entry(field)?);
    }
    let plan = format!("new Object[][]{{{}}}", entries.join(", "));
    let Some(result) = result else {
        return Ok(format!(
            "return Behaviors.mapValue({}, {plan});",
            value.name
        ));
    };
    if policy["realm"] == "tac" {
        return Ok(format!(
            "return TacModels.read{result}(Behaviors.mapValue(TacJsonReader.read(TacJsonWriter.write({})), {plan}));",
            value.name
        ));
    }
    Ok(format!(
        "try {{ return ({result}) YonModels.read({result}.class, Behaviors.mapValue(Json.parse(Json.write({})), {plan})); }} catch (RuntimeException failure) {{ throw failure; }} catch (Exception failure) {{ throw new IllegalArgumentException(\"The mapped value does not fit {result}.\", failure); }}",
        value.name
    ))
}

fn mapping_entry(field: &Plan) -> Result<String, String> {
    let read = match &field.source {
        Some(path) => format!("new String[]{{{}}}", quoted_list(path)),
        None => "null".to_owned(),
    };
    let nested = mapping_nested(field)?;
    Ok(format!(
        "{{{}, {read}, {}, {nested}}}",
        quote(&field.target),
        literal(Some(&field.constant))?
    ))
}

fn mapping_nested(field: &Plan) -> Result<String, String> {
    let Some(fields) = &field.fields else {
        return Ok("null".to_owned());
    };
    let entries = fields
        .iter()
        .map(mapping_entry)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("new Object[][]{{{}}}", entries.join(", ")))
}

fn quoted_list(names: &[String]) -> String {
    let mut quoted = Vec::new();
    for name in names {
        quoted.push(quote(name));
    }
    quoted.join(", ")
}

fn query_body(type_declaration: &Type, method: &Method, query: &Value) -> Result<String, String> {
    if !returns_object(method) {
        return Err("Java query methods must return Object bounded JSON".to_owned());
    }
    if !has_behaviors_context(type_declaration) {
        return Err(
            "Query services require an explicit Behaviors.Context behaviors field".to_owned(),
        );
    }
    let entity = query.get("entity").ok_or("Query entity metadata missing")?;
    let predicates = query_predicates(method, query)?;
    let order = query_ordering(query)?;
    let page = match query.get("pageArgument") {
        Some(index) if !index.is_null() => query_argument(method, index)?,
        _ => "null".to_owned(),
    };
    Ok(format!(
        "return this.behaviors.query(new Behaviors.Entity({}, {}, {}, {}), {}, new String[]{{{}}}, new Object[][]{{{}}}, new String[][]{{{}}}, {page}, {}, {});",
        string(entity, "name")?,
        string(entity, "adapter")?,
        string(entity, "table")?,
        string(entity, "id")?,
        string(query, "operation")?,
        strings(query, "fields")?,
        predicates.join(", "),
        order.join(", "),
        number(query, "defaultSize")?,
        number(query, "maxSize")?
    ))
}

fn query_predicates(method: &Method, query: &Value) -> Result<Vec<String>, String> {
    let predicates = query
        .get("where")
        .and_then(Value::as_array)
        .ok_or("Query predicates missing")?;
    let mut rendered = Vec::new();
    for predicate in predicates {
        let mut values = vec![string(predicate, "field")?, string(predicate, "operator")?];
        if let Some(index) = predicate.get("argument") {
            values.push(query_argument(method, index)?);
        }
        rendered.push(format!("new Object[]{{{}}}", values.join(", ")));
    }
    Ok(rendered)
}

fn query_ordering(query: &Value) -> Result<Vec<String>, String> {
    let orderings = query
        .get("orderBy")
        .and_then(Value::as_array)
        .ok_or("Query ordering missing")?;
    let mut rendered = Vec::new();
    for order in orderings {
        rendered.push(format!(
            "new String[]{{{}, {}}}",
            string(order, "field")?,
            string(order, "direction")?
        ));
    }
    Ok(rendered)
}

fn query_argument(method: &Method, index: &Value) -> Result<String, String> {
    let index = usize::try_from(index.as_u64().ok_or("Invalid query argument")?)
        .map_err(|_| "Invalid query argument")?;
    let Some(parameter) = method.parameters.get(index) else {
        return Err("Query argument out of bounds".to_owned());
    };
    Ok(parameter.name.clone())
}

fn helper(module: &Module, method: &Method) -> Result<(String, String), String> {
    let signature = &module.source[method.signature.clone()];
    let name = format!("__ty_behavior_{}", method.name);
    let position =
        method_name_position(signature, &method.name).ok_or("Invalid behavior method signature")?;
    let mut signature = signature.to_owned();
    signature.replace_range(position..position + method.name.len(), &name);
    let signature = without_visibility(signature.trim_start());
    let body = &module.source[method.body.clone()];
    Ok((name, format!("\nprivate {signature} {{ {body} }}\n")))
}

fn method_name_position(signature: &str, method_name: &str) -> Option<usize> {
    let mut last_position = None;
    for (position, _) in signature.match_indices(method_name) {
        let after_name = &signature[position + method_name.len()..];
        if after_name.trim_start().starts_with('(') {
            last_position = Some(position);
        }
    }
    last_position
}

fn without_visibility(signature: &str) -> &str {
    for visibility in ["public ", "protected ", "private "] {
        if let Some(rest) = signature.strip_prefix(visibility) {
            return rest;
        }
    }
    signature
}

fn returns_object(method: &Method) -> bool {
    matches!(method.return_type.trim(), "Object" | "java.lang.Object")
}

fn has_behaviors_context(type_declaration: &Type) -> bool {
    type_declaration
        .fields
        .iter()
        .any(|field| field.name == "behaviors" && field.type_source.trim() == "Behaviors.Context")
}

fn validate_managed(
    type_declaration: &Type,
    method: &Method,
    managed: bool,
    resilient: bool,
    environment: &str,
) -> Result<(), String> {
    let typed = JavaTypedClient::typed(method, environment).is_some();
    if managed && !returns_object(method) && !typed {
        return Err("Java cache/WebCall methods must return Object bounded JSON, not Future or an application model".to_owned());
    }
    if (managed || resilient) && !has_behaviors_context(type_declaration) {
        return Err(
            "Behavior services require an explicit Behaviors.Context behaviors field".to_owned(),
        );
    }
    Ok(())
}

fn declared_exchange(method: &Method) -> bool {
    method
        .metadata
        .get("httpExchange")
        .and_then(|exchange| exchange.get("generated"))
        .is_some()
}

struct Policies {
    retry: String,
    limit: String,
    transactional: String,
}

impl Policies {
    fn resilient(&self) -> bool {
        self.retry != "null" || self.limit != "null" || self.transactional != "null"
    }
}

fn policies(method: &Method) -> Result<Policies, String> {
    Ok(Policies {
        retry: resilience(method, "retryable")?,
        limit: resilience(method, "concurrencyLimit")?,
        transactional: resilience(method, "transactional")?,
    })
}

fn resilience(method: &Method, key: &str) -> Result<String, String> {
    let Some(value) = method.metadata.get(key) else {
        return Ok("null".to_owned());
    };
    if key == "transactional" {
        return Ok(transaction_policy(value));
    }
    if key == "retryable" {
        return retry_policy(value);
    }
    Ok(format!(
        "new Behaviors.Limit({}, {})",
        number(value, "limit")?,
        string(value, "policy")? == "\"block\""
    ))
}

fn transaction_policy(value: &Value) -> String {
    let adapter = value["adapter"].as_str().unwrap_or_default();
    format!("new Behaviors.Transaction({})", quote(adapter))
}

fn retry_policy(value: &Value) -> Result<String, String> {
    Ok(format!(
        "new Behaviors.Retry({}, {}, {:?}, {}, {})",
        number(value, "maxRetries")?,
        number(value, "delayMs")?,
        value
            .get("multiplier")
            .and_then(Value::as_f64)
            .ok_or("Retry needs multiplier")?,
        number(value, "maxDelayMs")?,
        number(value, "jitterMs")?
    ))
}

fn policy(value: &Value) -> Result<String, String> {
    Ok(format!(
        "new Behaviors.Policy({}, {}, {}, {})",
        string(value, "name")?,
        number(value, "ttlMs")?,
        number(value, "maxBytes")?,
        number(value, "maxEntries")?
    ))
}

fn indices(value: &Value) -> Result<String, String> {
    let key_arguments = value
        .get("keyArgs")
        .and_then(Value::as_array)
        .ok_or("Cached needs keyArgs")?;
    let mut indexes = Vec::new();
    for argument in key_arguments {
        let index = argument.as_u64().ok_or("Invalid cache argument index")?;
        indexes.push(index.to_string());
    }
    Ok(format!("new int[]{{{}}}", indexes.join(", ")))
}

fn strings(value: &Value, key: &str) -> Result<String, String> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("Missing {key}"))?;
    let mut quoted = Vec::new();
    for item in items {
        let text = item.as_str().ok_or_else(|| format!("Invalid {key}"))?;
        quoted.push(quote(text));
    }
    Ok(quoted.join(", "))
}

fn number(value: &Value, key: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Invalid {key}"))
}

fn string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(quote)
        .ok_or_else(|| format!("Invalid {key}"))
}

fn quote(value: &str) -> String {
    Value::String(value.to_owned()).to_string()
}

fn test_members(unit: &Unit, cleared_fakes: &str) -> String {
    let calls = super::super::ValueSource::jvm_calls(&unit.cases, &unit.inputs);
    let case_names: Vec<String> = calls.iter().map(|(name, _)| name.clone()).collect();
    let names = quoted_list(&case_names);
    let dispatch = super::super::testing::joined(&calls, |(name, call)| {
        format!("case {}: {call}; return;", quote(name))
    });
    let profiles = profile_list(unit);
    let properties = property_entries(unit);
    format!(
        "\nBehaviors.Context behaviors;\n\
         void startBehaviors() {{ stopBehaviors(); this.behaviors = Behaviors.create({}, \
         new Behaviors.Options({}, new Behaviors.Signal(), 10000, java.util.List.of({profiles}), java.util.Map.ofEntries({properties}))); }}\n\
         void stopBehaviors() {{ if (this.behaviors != null) {{ this.behaviors.close(); this.behaviors = null; }} }}\n\
         void runCase(String name) {{ switch (name) {{ {dispatch} default: return; }} }}\n\
         Object POST() {{\n\
         java.util.List<Object> cases = new java.util.ArrayList<>();\n\
         for (String name : new String[]{{{names}}}) {{\n\
         {cleared_fakes}\
         startBehaviors();\n\
         BehaviorCalls.take();\n\
         try {{ runCase(name); cases.add(Behaviors.record(\"name\", name, \"passed\", true, \"called\", BehaviorCalls.take())); }}\n\
         catch (RuntimeException | AssertionError error) {{ cases.add(Behaviors.record(\"name\", name, \"passed\", false, \"error\", java.util.Objects.requireNonNullElse(error.getMessage(), error.getClass().getName()), \"called\", BehaviorCalls.take())); }}\n\
         finally {{ stopBehaviors(); }}\n\
         }}\n\
         return Behaviors.record(\"unit\", {}, \"cases\", cases);\n\
         }}\n",
        quote(&unit.configuration),
        quote(&unit.partition),
        quote(&unit.name),
    )
}

fn profile_list(unit: &Unit) -> String {
    let Some(profiles) = unit.profiles.as_array() else {
        return String::new();
    };
    let mut quoted = Vec::new();
    for profile in profiles {
        if let Some(name) = profile.as_str() {
            quoted.push(quote(name));
        }
    }
    quoted.join(", ")
}

fn property_entries(unit: &Unit) -> String {
    let Some(entries) = unit.properties.as_object() else {
        return String::new();
    };
    let mut pairs = Vec::new();
    for (key, value) in entries {
        let name = key.replace('.', "_").to_uppercase();
        let text = value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned);
        pairs.push(format!(
            "java.util.Map.entry({}, {})",
            quote(&name),
            quote(&text)
        ));
    }
    pairs.join(", ")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    fn lower_client(source: &str) -> String {
        super::super::lower(
            std::path::Path::new("server/remotes/ReportClient.java"),
            source,
            "yon",
        )
        .expect("Java gateway lowering")
        .expect("lowered source")
    }

    #[test]
    fn a_gateway_method_calls_the_transport_without_a_behavior_context() {
        let lowered = lower_client(
            "class ReportClient {\n    @Gateway({\"class\":\"server/services/ReportService.py\"})\n    Object summarize(Object mode, Object detail) {}\n}\n",
        );
        assert!(lowered.contains(
            "Object summarize(Object mode, Object detail) {return Behaviors.gatewayResult(YonGateway.call(\"ReportClient.summarize\", new Object[]{mode, detail}));}"
        ));
        assert!(!lowered.contains("Behaviors.Context behaviors"));
    }

    #[test]
    fn a_gateway_interface_becomes_a_class_without_a_behavior_context() {
        let lowered = lower_client(
            "interface ReportClient {\n    @Gateway({\"command\":[\"python3\"]})\n    Object ping();\n}\n",
        );
        assert!(lowered.contains("class ReportClient {"));
        assert!(lowered.contains(
            "Object ping() { return Behaviors.gatewayResult(YonGateway.call(\"ReportClient.ping\", new Object[]{})); }"
        ));
        assert!(!lowered.contains("Behaviors.Context behaviors"));
    }

    #[test]
    fn a_gateway_parameter_must_be_object() {
        let failure = super::super::lower(
            std::path::Path::new("server/remotes/ReportClient.java"),
            "class ReportClient {\n    @Gateway({\"command\":[\"python3\"]})\n    Object ping(String mode) {}\n}\n",
            "yon",
        )
        .expect_err("typed parameter");
        let message = failure.to_string();
        assert!(message.contains("TY1312"), "{message}");
        assert!(message.contains("must take Object parameters"), "{message}");
    }
}

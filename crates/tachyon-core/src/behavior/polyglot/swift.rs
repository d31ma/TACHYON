use super::super::call_style::CallStyle;
use super::super::mapper;
use super::super::testing::{Unit, joined, polyglot_unit};
use super::string_literal::StringLiteral;
use super::{Edit, Method, Module, Parameter, Type, block, concrete_edit, declared_fields, render};
use serde_json::{Map, Value};
use std::fmt::Write as _;
use swift_client_answer::SwiftClientAnswer;

mod swift_client_answer;

pub(super) fn lower(module: &Module) -> Result<String, String> {
    let mut edits = Vec::new();
    let mut registrations = Map::new();
    let mut configurations = String::new();
    let mut entities = String::new();
    let mut policies = String::new();
    for type_declaration in &module.types {
        edits.extend(concrete_edit(module, type_declaration));
        policies.push_str(&policy_line(type_declaration));
        if type_declaration.configuration {
            let _ = writeln!(
                configurations,
                "{}",
                configuration_arm(module, type_declaration)?
            );
        }
        if let Some(entity) = &type_declaration.entity {
            entities.push_str(&entity_line(type_declaration, entity));
        }
        if let Some(fields) = &type_declaration.constructor {
            edits.push(constructor_edit(type_declaration, fields)?);
        }
        if let Some(unit) = polyglot_unit(type_declaration) {
            let construction = super::typed_test_construction::TypedTestConstruction::members(
                module,
                type_declaration,
                &unit,
                "swift",
            );
            edits.push(Edit {
                range: type_declaration.body.start..type_declaration.body.start,
                replacement: format!("{}{construction}", test_members(&unit)),
            });
        }
        let methods = lower_methods(module, type_declaration, &mut edits)?;
        registrations.insert(
            type_declaration.name.clone(),
            serde_json::json!({"cache": type_declaration.cache, "methods": methods}),
        );
    }
    let registry = Value::Object(registrations).to_string();
    let generated = include_str!("swift_runtime.swift")
        .replace("__TAC_BEHAVIOR_MODULE__", &quote(&module.module_id))
        .replace("__TAC_BEHAVIOR_ENVIRONMENT__", &quote(&module.environment))
        .replace("__TAC_BEHAVIOR_REGISTRY__", &quote(&registry))
        .replace("__TAC_BEHAVIOR_CONFIGURATIONS__", &configurations)
        .replace("__TAC_BEHAVIOR_ENTITIES__", &entities)
        .replace("__TAC_BEHAVIOR_TYPES__", &policies);
    let helper = super::super::testing::unit_helper(&module.types, super::super::TestAssert::SWIFT);
    let generated = format!("{generated}\n{}{helper}", super::SwiftPlatform::FOUNDATION);
    render(module, edits, &generated)
}

fn policy_line(type_declaration: &Type) -> String {
    let policies = Value::Object(type_declaration.policies.clone()).to_string();
    format!(
        "if ObjectIdentifier(type) == ObjectIdentifier({}.self) {{ return try BehaviorJSON.parse({}) }}\n",
        type_declaration.name,
        quote(&policies)
    )
}

fn entity_line(type_declaration: &Type, entity: &Value) -> String {
    let mapping = serde_json::json!({
        "entity": type_declaration.name,
        "adapter": entity["adapter"],
        "idField": entity["id"],
        "table": type_declaration.table
    });
    format!(
        "if ObjectIdentifier(entity) == ObjectIdentifier({}.self) {{ return try BehaviorJSON.parse({}) }}\n",
        type_declaration.name,
        quote(&mapping.to_string())
    )
}

fn constructor_edit(type_declaration: &Type, fields: &[String]) -> Result<Edit, String> {
    let parameters = constructor_parameters(type_declaration, fields)?;
    let mut assignments = Vec::new();
    for name in fields {
        assignments.push(format!("self.{name} = {name}"));
    }
    let assignments = assignments.join("; ");
    let declarations = declared_fields(type_declaration, |field| {
        format!("\nlet {}: {}", field.name, field.type_source)
    });
    Ok(Edit {
        range: type_declaration.body.start..type_declaration.body.start,
        replacement: format!("{declarations}\ninit({parameters}) {{ {assignments} }}\n"),
    })
}

fn constructor_parameters(type_declaration: &Type, fields: &[String]) -> Result<String, String> {
    let mut parameters = Vec::new();
    for name in fields {
        let Some(field) = type_declaration
            .fields
            .iter()
            .find(|field| field.name == *name)
        else {
            return Err(format!(
                "Required constructor field '{name}' has no explicit Swift type"
            ));
        };
        parameters.push(format!("{name}: {}", field.type_source));
    }
    Ok(parameters.join(", "))
}

fn configuration_arm(module: &Module, type_declaration: &Type) -> Result<String, String> {
    let configure = type_declaration
        .methods
        .iter()
        .find(|method| method.name == "configure")
        .ok_or("Swift Setup requires static configure")?;
    let signature = &module.source[configure.signature.clone()];
    let attempt = if declares_throws(signature) {
        "try "
    } else {
        ""
    };
    let label = first_parameter_label(signature)
        .ok_or("Swift configure requires an explicit execution parameter")?;
    let argument = if label == "_" {
        "execution".to_owned()
    } else {
        format!("{label}: execution")
    };
    Ok(format!(
        "if ObjectIdentifier(configuration) == ObjectIdentifier({}.self) {{ return {attempt}{}.configure({argument}) }}",
        type_declaration.name, type_declaration.name
    ))
}

fn first_parameter_label(signature: &str) -> Option<&str> {
    let (_, parameters) = signature.split_once('(')?;
    let (label, _) = parameters.split_once(':')?;
    label.split_whitespace().next()
}

fn declares_throws(signature: &str) -> bool {
    signature.split_whitespace().any(|word| word == "throws")
}

fn lower_methods(
    module: &Module,
    type_declaration: &Type,
    edits: &mut Vec<Edit>,
) -> Result<Map<String, Value>, String> {
    let mut methods = Map::new();
    for method in &type_declaration.methods {
        if method.metadata.is_empty() {
            continue;
        }
        let signature = &module.source[method.signature.clone()];
        if !declares_throws(signature) {
            return Err("Swift behavior methods must declare throws".to_owned());
        }
        if let Some(replacement) = written_body(method)? {
            edits.push(Edit {
                range: method.body.clone(),
                replacement: block(method, &replacement),
            });
            continue;
        }
        let mut prefix = validation_prefix(method)?;
        if let Some(policy) = method.metadata.get("pageable") {
            prefix.push_str(&pageable_line(method, policy)?);
        }
        let body = &module.source[method.body.clone()];
        let replacement = method_body(type_declaration, method, &prefix, body)?;
        edits.push(Edit {
            range: method.body.clone(),
            replacement: block(method, &replacement),
        });
        methods.insert(method.name.clone(), Value::Object(method.metadata.clone()));
    }
    Ok(methods)
}

fn method_body(
    type_declaration: &Type,
    method: &Method,
    prefix: &str,
    body: &str,
) -> Result<String, String> {
    if is_validation_only(method) {
        if method.return_type.trim() != "BehaviorJSON" {
            return Err("Swift Check methods require BehaviorJSON".to_owned());
        }
        return Ok(format!("\n{prefix}{body}\n"));
    }
    if uses_invocation(method) {
        return invocation_body(type_declaration, method, prefix, body);
    }
    Ok(format!(
        "\n{prefix}return try self.behaviors.paginate {{ {body} }}\n"
    ))
}

fn is_validation_only(method: &Method) -> bool {
    method.metadata.len() == 1 && method.metadata.contains_key("valid")
}

fn uses_invocation(method: &Method) -> bool {
    method.metadata.contains_key("cacheable")
        || method.metadata.contains_key("httpExchange")
        || method.asynchronous
        || method.metadata.contains_key("cacheEvict")
        || method.metadata.contains_key("cachePut")
}

fn is_declared_exchange(method: &Method) -> bool {
    let Some(exchange) = method.metadata.get("httpExchange") else {
        return false;
    };
    exchange.get("generated").is_some()
}

fn invocation_body(
    type_declaration: &Type,
    method: &Method,
    prefix: &str,
    body: &str,
) -> Result<String, String> {
    let declared = is_declared_exchange(method);
    let answer = SwiftClientAnswer::of(method.return_type.trim(), declared);
    if !method.asynchronous || !answer.allowed() {
        return Err("Swift asynchronous behaviors require async throws -> BehaviorJSON".to_owned());
    }
    let arguments = json_arguments(method);
    let authored = if declared { ".null" } else { body };
    let call = format!(
        "try await self.behaviors.invoke({}, {}, arguments: [{arguments}]) {{ () async throws -> BehaviorJSON in {authored} }}",
        quote(&type_declaration.name),
        quote(&method.name)
    );
    Ok(format!("\n{prefix}{}\n", answer.written(&call)))
}

fn pageable_line(method: &Method, policy: &Value) -> Result<String, String> {
    let parameter = parameter_at(method, policy, "Pageable")?;
    if parameter.type_source.trim() != "BehaviorJSON" {
        return Err("Swift Pageable parameters must have type BehaviorJSON".to_owned());
    }
    let validates_same_argument = method
        .metadata
        .get("valid")
        .is_some_and(|valid| valid["argument"] == policy["argument"]);
    let declaration = if validates_same_argument { "" } else { "let " };
    Ok(format!(
        "{declaration}{} = try BehaviorPage.normalize({}, policy: BehaviorJSON.parse({}))\n",
        parameter.name,
        parameter.name,
        quote(&policy.to_string())
    ))
}

fn parameter_at<'a>(
    method: &'a Method,
    policy: &Value,
    annotation_name: &str,
) -> Result<&'a Parameter, String> {
    let Some(position) = policy["argument"].as_u64() else {
        return Err(format!("Missing {annotation_name} argument"));
    };
    let out_of_range = format!("{annotation_name} argument is out of range");
    let Ok(index) = usize::try_from(position) else {
        return Err(out_of_range);
    };
    method.parameters.get(index).ok_or(out_of_range)
}

fn validation_prefix(method: &Method) -> Result<String, String> {
    let Some(policy) = method.metadata.get("valid") else {
        return Ok(String::new());
    };
    let parameter = parameter_at(method, policy, "Check")?;
    if parameter.type_source.trim() != "BehaviorJSON" {
        return Err("Swift Check parameters require BehaviorJSON".to_owned());
    }
    let pages_same_argument = method
        .metadata
        .get("pageable")
        .is_some_and(|page| page["argument"] == policy["argument"]);
    let binding = if pages_same_argument { "var" } else { "let" };
    Ok(format!(
        "{binding} {} = try Behaviors.validateSchema(BehaviorJSON.parse({}), {})\n",
        parameter.name,
        quote(&policy["schema"].to_string()),
        parameter.name
    ))
}

fn written_body(method: &Method) -> Result<Option<String>, String> {
    if let Some(query) = method.metadata.get("query") {
        return query_body(method, query).map(Some);
    }
    if let Some(policy) = method.metadata.get("mapping") {
        return mapping_body(method, policy).map(Some);
    }
    Ok(None)
}

fn mapping_body(method: &Method, policy: &Value) -> Result<String, String> {
    if policy.get("result").is_none() && method.return_type.trim() != "BehaviorJSON" {
        return Err("Swift mapping methods require throws -> BehaviorJSON".to_owned());
    }
    let (argument, _) = mapper::plan(policy)?;
    let value = method
        .parameters
        .get(argument)
        .ok_or("Convert reads an argument the method does not declare")?;
    if let Some(result) = policy.get("result").and_then(Value::as_str) {
        return Ok(format!(
            "\nlet mapped = try BehaviorContext.mapValue(BehaviorJSON.parse({}), BehaviorJSON.parse(tacEncode(tacModelValue({}))))\nreturn try tacModel({result}.self, tacParseJSON(mapped.encoded()))\n",
            quote(&policy.to_string()),
            value.name
        ));
    }
    Ok(format!(
        "\nreturn try BehaviorContext.mapValue(BehaviorJSON.parse({}), try BehaviorJSON.from({}))\n",
        quote(&policy.to_string()),
        value.name
    ))
}

fn query_body(method: &Method, query: &Value) -> Result<String, String> {
    if !method.asynchronous || method.return_type.trim() != "BehaviorJSON" {
        return Err("Swift query methods require async throws -> BehaviorJSON".to_owned());
    }
    let arguments = json_arguments(method);
    Ok(format!(
        "\nreturn try await self.behaviors.query(BehaviorJSON.parse({}), arguments: [{arguments}])\n",
        quote(&query.to_string())
    ))
}

fn json_arguments(method: &Method) -> String {
    let mut arguments = Vec::new();
    for parameter in &method.parameters {
        arguments.push(format!("try BehaviorJSON.from({})", parameter.name));
    }
    arguments.join(", ")
}

fn quote(value: &str) -> String {
    StringLiteral::SWIFT.of(value)
}

fn test_members(unit: &Unit) -> String {
    let calls = super::super::ValueSource::calls(&unit.cases, &unit.inputs, &CallStyle::SWIFT);
    let names = joined(&calls, |(name, _)| format!("{}, ", quote(name)));
    let dispatch = joined(&calls, |(name, call)| {
        format!("case {}: try await {call}\n", quote(name))
    });
    let profiles = profile_list(unit);
    let mut properties = property_list(unit);
    if properties.is_empty() {
        properties = String::from(":");
    }
    format!(
        "\nvar behaviors: BehaviorContext? = nil\n\
         func startBehaviors() throws {{\n\
         stopBehaviors()\n\
         behaviors = try Behaviors.create({}.self, partition: {}, signal: BehaviorSignal(), timeoutMs: 10000, profiles: [{profiles}], properties: [{properties}])\n\
         }}\n\
         func stopBehaviors() {{ behaviors?.dispose(); behaviors = nil }}\n\
         func runCase(_ name: String) async throws {{\n\
         switch name {{\n{dispatch}default: break\n}}\n\
         }}\n\
         func POST() async throws -> BehaviorJSON {{\n\
         var cases: [BehaviorJSON] = []\n\
         for name in [{names}] {{\n\
         do {{\n\
         try startBehaviors()\n\
         _ = BehaviorCalls.take()\n\
         try await runCase(name)\n\
         cases.append(.object([\"name\": .string(name), \"passed\": .bool(true), \"called\": BehaviorCalls.take()]))\n\
         }} catch let error as BehaviorError {{\n\
         cases.append(.object([\"name\": .string(name), \"passed\": .bool(false), \"error\": .string(error.code), \"called\": BehaviorCalls.take()]))\n\
         }} catch {{\n\
         cases.append(.object([\"name\": .string(name), \"passed\": .bool(false), \"error\": .string(String(describing: error)), \"called\": BehaviorCalls.take()]))\n\
         }}\n\
         stopBehaviors()\n\
         }}\n\
         return .object([\"unit\": .string({}), \"cases\": .array(cases)])\n\
         }}\n",
        unit.configuration,
        quote(&unit.partition),
        quote(&unit.name),
    )
}

fn profile_list(unit: &Unit) -> String {
    let mut profiles = String::new();
    let Some(values) = unit.profiles.as_array() else {
        return profiles;
    };
    for value in values {
        if let Some(profile) = value.as_str() {
            let _ = write!(profiles, "{}, ", quote(profile));
        }
    }
    profiles
}

fn property_list(unit: &Unit) -> String {
    let mut properties = String::new();
    let Some(entries) = unit.properties.as_object() else {
        return properties;
    };
    for (key, value) in entries {
        let _ = write!(
            properties,
            "{}: {}, ",
            quote(key),
            quote(&value.to_string())
        );
    }
    properties
}

#[cfg(all(test, target_os = "macos"))]
#[allow(clippy::expect_used)]
mod tests {
    fn annotated_source() -> String {
        let annotations = include_str!("../../../tests/fixtures/behavior/swift-annotations.swift")
            .replace(
                "let signal =",
                "try ConfigurationAnnotationTests.run(); try await ValidationAnnotationTests.run(); let signal =",
            );
        format!(
            "{}\n{}\n{}",
            annotations,
            include_str!("../../../tests/fixtures/behavior/swift-configuration.swift"),
            include_str!("../../../tests/fixtures/behavior/swift-validation.swift")
        )
    }

    async fn compile_and_execute(emitted: String) {
        let directory = tempfile::tempdir().expect("emitted source directory");
        let path = directory.path().join("Emitted.swift");
        std::fs::write(&path, emitted).expect("emitted Swift source");
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/behavior-native-runtime-test.mjs");
        let mut command = tokio::process::Command::new("node");
        command.arg(script).arg("swift").arg("--emitted").arg(path);
        let result = crate::external_command::run(
            &mut command,
            std::time::Duration::from_mins(4),
            1024 * 1024,
        )
        .await
        .expect("Swift compile and execution tool");
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[tokio::test]
    async fn swift_all_annotations_compile_and_execute() {
        let path = std::path::Path::new("client/pages/tac.swift");
        let emitted = super::super::lower(path, &annotated_source(), "tac")
            .expect("Swift annotation lowering")
            .expect("annotated source");
        assert!(emitted.contains("init(behaviors: BehaviorContext)"));
        compile_and_execute(emitted).await;
    }
}

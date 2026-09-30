use super::super::testing::{Unit, polyglot_unit};
use super::{Edit, Method, Module, Type, block, concrete_edit, declared_fields, render};
use serde_json::{Map, Value};
use std::fmt::Write as _;

pub(super) fn lower(module: &Module) -> Result<String, String> {
    let mut edits = Vec::new();
    let mut records = Map::new();
    let mut configurations = String::new();
    let mut constructors = String::new();
    for type_declaration in &module.types {
        edits.extend(concrete_edit(module, type_declaration));
        let mut record = type_record(type_declaration);
        if type_declaration.configuration {
            let _ = writeln!(
                configurations,
                "case '{}': return {}::configure($execution);",
                type_declaration.name, type_declaration.name
            );
        }
        if let Some(fields) = &type_declaration.constructor {
            edits.push(constructor_edit(type_declaration, fields));
            let _ = writeln!(
                constructors,
                "{}",
                constructor_case(type_declaration, fields)
            );
        }
        if let Some(unit) = polyglot_unit(type_declaration) {
            edits.push(Edit {
                range: type_declaration.body.start..type_declaration.body.start,
                replacement: test_members(&unit),
            });
        }
        lower_methods(module, type_declaration, &mut record, &mut edits)?;
        records.insert(type_declaration.name.clone(), record);
    }
    edits.extend(closing_tag_removal(module));
    let encoded_records = serde_json::to_string(&records).map_err(|error| error.to_string())?;
    let runtime = include_str!("php_runtime.php")
        .replace(
            "__BEHAVIOR_RECORDS__",
            &escape_single_quoted(&encoded_records),
        )
        .replace("__BEHAVIOR_ENVIRONMENT__", &module.environment)
        .replace("__BEHAVIOR_MODULE_ID__", &module.module_id)
        .replace("__BEHAVIOR_CONFIGURATIONS__", &configurations)
        .replace("__BEHAVIOR_CONSTRUCTORS__", &constructors);
    let helper = super::super::testing::unit_helper(&module.types, super::super::TestAssert::PHP);
    render(module, edits, &format!("{runtime}\n{helper}"))
}

fn type_record(type_declaration: &Type) -> Value {
    let mut record = serde_json::json!({
        "name": type_declaration.name,
        "configuration": type_declaration.configuration,
        "methods": {}
    });
    for (key, value) in &type_declaration.policies {
        record[key] = value.clone();
    }
    if let Some(value) = &type_declaration.cache {
        record["cache"] = value.clone();
    }
    if let Some(value) = &type_declaration.entity {
        record["entity"] = value.clone();
    }
    if let Some(value) = &type_declaration.table {
        record["table"] = Value::String(value.clone());
    }
    if let Some(fields) = &type_declaration.constructor {
        record["constructor"] = serde_json::json!(fields);
    }
    record
}

fn constructor_edit(type_declaration: &Type, fields: &[String]) -> Edit {
    let mut parameters = Vec::new();
    for name in fields {
        let field_type = field_type(type_declaration, name);
        parameters.push(format!("{field_type} ${name}"));
    }
    let parameters = parameters.join(", ");
    let mut body = String::new();
    for name in fields {
        let _ = write!(body, "$this->{name} = ${name};");
    }
    let declarations = declared_fields(type_declaration, |field| {
        format!("\npublic {} ${};", field.type_source, field.name)
    });
    Edit {
        range: type_declaration.body.start..type_declaration.body.start,
        replacement: format!(
            "{declarations}\npublic function __construct({parameters}) {{{body}}}\n"
        ),
    }
}

fn field_type<'a>(type_declaration: &'a Type, name: &str) -> &'a str {
    for field in &type_declaration.fields {
        if field.name == name {
            return field.type_source.as_str();
        }
    }
    "mixed"
}

fn constructor_case(type_declaration: &Type, fields: &[String]) -> String {
    let mut values = Vec::new();
    for field in fields {
        values.push(format!("$values['{field}']"));
    }
    format!(
        "case '{}': return new {}({});",
        type_declaration.name,
        type_declaration.name,
        values.join(",")
    )
}

fn closing_tag_removal(module: &Module) -> Option<Edit> {
    let trimmed_source = module.source.trim_end();
    let before_closing_tag = trimmed_source.strip_suffix("?>")?;
    Some(Edit {
        range: before_closing_tag.len()..trimmed_source.len(),
        replacement: String::new(),
    })
}

fn escape_single_quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\'', "\\'")
}

fn lower_methods(
    module: &Module,
    type_declaration: &Type,
    record: &mut Value,
    edits: &mut Vec<Edit>,
) -> Result<(), String> {
    for method in &type_declaration.methods {
        check_query_result(method)?;
        record["methods"][&method.name] = Value::Object(method.metadata.clone());
        let arguments = argument_names(method);
        let original = &module.source[method.body.clone()];
        let replacement = if let Some(policy) = gateway_only_policy(method) {
            gateway_body(policy, &arguments)
        } else if let Some(policy) = validation_only_policy(method) {
            validation_body(policy, &arguments, original)
        } else {
            invocation_body(type_declaration, method, &arguments, original)
        };
        edits.push(Edit {
            range: method.body.clone(),
            replacement: block(method, &replacement),
        });
    }
    Ok(())
}

fn check_query_result(method: &Method) -> Result<(), String> {
    let Some(query) = method.metadata.get("query") else {
        return Ok(());
    };
    let result = match query["operation"].as_str() {
        Some("find") => "array",
        Some("findOne") => "?array",
        Some("count" | "delete") => "int",
        _ => "bool",
    };
    if ["", "mixed", result].contains(&method.return_type.as_str()) {
        return Ok(());
    }
    Err(format!(
        "PHP {} query requires {result}, mixed or an untyped result, never void or an incompatible result type",
        method.name
    ))
}

fn argument_names(method: &Method) -> Vec<String> {
    let mut names = Vec::new();
    for parameter in &method.parameters {
        names.push(format!("${}", parameter.name));
    }
    names
}

fn gateway_only_policy(method: &Method) -> Option<&Value> {
    if method.metadata.len() != 1 {
        return None;
    }
    method.metadata.get("gateway")
}

fn gateway_body(policy: &Value, arguments: &[String]) -> String {
    let key = policy["key"].as_str().unwrap_or_default();
    format!(
        "\nreturn BehaviorBoundary::gateway('{key}', [{}]);\n",
        arguments.join(",")
    )
}

fn validation_only_policy(method: &Method) -> Option<&Value> {
    if method.metadata.len() != 1 {
        return None;
    }
    method.metadata.get("valid")
}

fn validation_body(policy: &Value, arguments: &[String], original: &str) -> String {
    let index = policy["argument"]
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or_default();
    let schema = escape_single_quoted(&policy["schema"].to_string());
    let argument = &arguments[index];
    format!(
        "\n{argument} = BehaviorValidation::apply(json_decode('{schema}',true,32,JSON_THROW_ON_ERROR), {argument});\n{original}\n"
    )
}

fn invocation_body(
    type_declaration: &Type,
    method: &Method,
    arguments: &[String],
    original: &str,
) -> String {
    let mut bindings = String::new();
    for (index, name) in arguments.iter().enumerate() {
        let _ = writeln!(bindings, "{name} = $__behavior_args[{index}];");
    }
    let call = format!(
        "$this->behaviors->invoke($this, '{}', '{}', [{}], function($__behavior_args) {{\n{bindings}{original}\n}})",
        type_declaration.name,
        method.name,
        arguments.join(",")
    );
    let argument_check = query_argument_check(method, arguments.len());
    let prefix = if method.return_type == "void" {
        ""
    } else {
        "return "
    };
    format!("\n{argument_check}{prefix}{call};\n")
}

fn query_argument_check(method: &Method, argument_count: usize) -> String {
    if !method.metadata.contains_key("query") {
        return String::new();
    }
    format!(
        "if (func_num_args() !== {argument_count}) BehaviorBoundary::invalid('Query requires its exact declared positional arguments.');\n"
    )
}

fn test_members(unit: &Unit) -> String {
    let cases = escape_single_quoted(
        &super::super::ValueSource::case_table(&unit.cases, &unit.inputs).to_string(),
    );
    let properties = unit.properties.to_string().replace('\'', "\\'");
    format!(
        "\npublic mixed $behaviors = null;\n\
         public function startBehaviors(): void {{\n\
         $this->stopBehaviors();\n\
         $this->behaviors = Behaviors::create({}::class, ['partition' => {}, 'signal' => new BehaviorSignal(), 'profiles' => {}, 'properties' => json_decode('{properties}', true)]);\n\
         }}\n\
         public function stopBehaviors(): void {{\n\
         if ($this->behaviors !== null) {{ $this->behaviors->dispose(); $this->behaviors = null; }}\n\
         }}\n\
         public function POST($request): mixed {{\n\
         $cases = [];\n\
         foreach (json_decode('{cases}', true) as [$name, $method, $input]) {{\n\
         $this->startBehaviors();\n\
         \\BehaviorCalls::take();\n\
         try {{ $this->$method(...$input); $cases[] = ['name' => $name, 'passed' => true, 'called' => \\BehaviorCalls::take()]; }}\n\
         catch (\\Throwable $error) {{ $cases[] = ['name' => $name, 'passed' => false, 'error' => $error->getMessage(), 'called' => \\BehaviorCalls::take()]; }}\n\
         finally {{ $this->stopBehaviors(); }}\n\
         }}\n\
         return ['unit' => {}, 'cases' => $cases];\n\
         }}\n",
        unit.configuration,
        serde_json::Value::String(unit.partition.clone()),
        unit.profiles,
        serde_json::Value::String(unit.name.clone()),
    )
}

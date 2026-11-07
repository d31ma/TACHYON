use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub(super) fn constraint(name: &str) -> bool {
    matches!(name, "Required" | "NotEmpty" | "Length" | "Min" | "Max")
}

pub(super) fn field_marker(name: &str) -> bool {
    constraint(name) || matches!(name, "Setting" | "Id")
}

pub(super) fn apply_field(
    input: &Input<'_>,
    marker: &Marker,
    language: &str,
    policies: &mut Map<String, Value>,
) -> Result<(), Failure> {
    if marker.name == "Setting" {
        let field = field_name(input, marker, language)?;
        let descriptor = super::configuration::value(input, marker)?;
        return super::configuration::bind_field(input, marker, policies, &field, descriptor);
    }
    if marker.name == "Id" {
        return identifier_field(input, marker, language, policies);
    }
    let go_required_field = language == "go"
        && marker.name == "Required"
        && !policies.contains_key("validated")
        && !policies.contains_key("configurationProperties");
    if go_required_field {
        return Ok(());
    }
    field(input, marker, language, policies)
}

pub(super) fn identifier_agrees(
    input: &Input<'_>,
    at: usize,
    policies: &Map<String, Value>,
    entity: Option<&Value>,
) -> Result<(), Failure> {
    if !policies.contains_key("identifier") || entity.is_some() {
        return Ok(());
    }
    Err(input.failure(
        at,
        "Id requires Entity on its owning type, which names the adapter the identifier belongs to",
    ))
}

fn identifier_field(
    input: &Input<'_>,
    marker: &Marker,
    language: &str,
    policies: &mut Map<String, Value>,
) -> Result<(), Failure> {
    if !marker.arguments.trim().is_empty() {
        return Err(input.failure(
            marker.span.start,
            crate::annotation_rules::AnnotationRules::no_arguments("Id"),
        ));
    }
    let field = field_name(input, marker, language)?;
    if policies
        .insert(String::from("identifier"), Value::String(field))
        .is_some()
    {
        return Err(input.failure(marker.target, "One Id per type"));
    }
    Ok(())
}

pub(super) fn field(
    input: &Input<'_>,
    marker: &Marker,
    language: &str,
    policies: &mut Map<String, Value>,
) -> Result<(), Failure> {
    let name = field_name(input, marker, language)?;
    let rules = field_rules(input, marker, policies, &name)?;
    let (key, value) = rule(input, marker)?;
    if rules.insert(key.into(), value).is_some() {
        return Err(input.failure(marker.target, "Duplicate validation field constraint"));
    }
    if min_exceeds_max(rules) {
        return Err(input.failure(marker.target, "Min cannot exceed Max"));
    }
    Ok(())
}

fn field_rules<'policies>(
    input: &Input<'_>,
    marker: &Marker,
    policies: &'policies mut Map<String, Value>,
    name: &str,
) -> Result<&'policies mut Map<String, Value>, Failure> {
    let settings = policies.contains_key("configurationProperties");
    if settings && policies.contains_key("validated") {
        return Err(input.failure(
            marker.target,
            "A class is either Settings or Checked, not both",
        ));
    }
    if settings {
        return super::configuration::field_bounds(input, marker, policies, name);
    }
    let schema = policies.get_mut("validated").ok_or_else(|| {
        input.failure(
            marker.target,
            "Field constraints require Checked on their owning type",
        )
    })?;
    let fields = schema["fields"]
        .as_object_mut()
        .ok_or_else(|| input.failure(marker.target, "Invalid validation schema"))?;
    if !fields.contains_key(name) && fields.len() >= 64 {
        return Err(input.failure(marker.target, "Checked type exceeds 64 constrained fields"));
    }
    fields
        .entry(name.to_owned())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| input.failure(marker.target, "Invalid validation field"))
}

fn min_exceeds_max(rules: &Map<String, Value>) -> bool {
    let min = rules.get("min").and_then(Value::as_f64);
    let max = rules.get("max").and_then(Value::as_f64);
    let (Some(min), Some(max)) = (min, max) else {
        return false;
    };
    min > max
}

pub(super) fn field_name(
    input: &Input<'_>,
    marker: &Marker,
    language: &str,
) -> Result<String, Failure> {
    let code = input.code[marker.target..]
        .lines()
        .next()
        .unwrap_or_default();
    let before_assignment = code.split('=').next().unwrap_or_default();
    let is_static = code.split_whitespace().any(|word| word == "static");
    if before_assignment.contains(['(', '{', '}']) || is_static {
        return Err(input.failure(
            marker.target,
            "Field annotations require direct instance field declarations",
        ));
    }
    let declaration = before_assignment.trim().trim_end_matches([';', ',']);
    let name = declared_name(declaration, language);
    if !super::identifier(name) {
        return Err(input.failure(
            marker.target,
            "Field annotations require a safe named declaration",
        ));
    }
    Ok(name.to_owned())
}

fn declared_name<'code>(declaration: &'code str, language: &str) -> &'code str {
    if language == "go" {
        return declaration.split_whitespace().next().unwrap_or_default();
    }
    if language == "rb" {
        return ruby_field_name(declaration);
    }
    let name = if matches!(language, "js" | "ts" | "py" | "swift" | "kt" | "rs") {
        let before_type = declaration.split(':').next().unwrap_or_default();
        before_type.split_whitespace().last().unwrap_or_default()
    } else {
        let last_word = declaration.split_whitespace().last().unwrap_or_default();
        last_word.trim_start_matches('$')
    };
    if language != "ts" {
        return name;
    }
    name.strip_suffix('!')
        .or_else(|| name.strip_suffix('?'))
        .unwrap_or(name)
}

fn ruby_field_name(declaration: &str) -> &str {
    let Some((word, symbol)) = declaration.split_once(char::is_whitespace) else {
        return "";
    };
    let is_field = matches!(word, "attr_reader" | "attr_accessor" | "attr_writer");
    let name = symbol.trim().strip_prefix(':').unwrap_or_default();
    if is_field && !name.contains(',') {
        name
    } else {
        ""
    }
}

fn rule(input: &Input<'_>, marker: &Marker) -> Result<(&'static str, Value), Failure> {
    match marker.name.as_str() {
        "Required" | "NotEmpty" => presence_rule(input, marker),
        "Length" => size_rule(input, marker),
        "Min" | "Max" => limit_rule(input, marker),
        _ => Err(input.failure(marker.span.start, "Unknown validation field constraint")),
    }
}

fn presence_rule(input: &Input<'_>, marker: &Marker) -> Result<(&'static str, Value), Failure> {
    if !marker.arguments.trim().is_empty() {
        return Err(input.failure(
            marker.span.start,
            "Required and NotEmpty accept no arguments",
        ));
    }
    let key = if marker.name == "Required" {
        "notNull"
    } else {
        "notBlank"
    };
    Ok((key, Value::Bool(true)))
}

fn size_rule(input: &Input<'_>, marker: &Marker) -> Result<(&'static str, Value), Failure> {
    let object = super::object(input, marker, &["min", "max"])?;
    super::bounded_integer(input, marker, &object, "min", 0, 1_048_576)?;
    super::bounded_integer(input, marker, &object, "max", 0, 1_048_576)?;
    if object["min"].as_u64() > object["max"].as_u64() {
        return Err(input.failure(marker.span.start, "Length.min cannot exceed Length.max"));
    }
    Ok(("size", Value::Object(object)))
}

fn limit_rule(input: &Input<'_>, marker: &Marker) -> Result<(&'static str, Value), Failure> {
    let value: Value = serde_json::from_str(&marker.arguments)
        .map_err(|_| input.failure(marker.span.start, "Min/Max require a literal finite number"))?;
    let is_bounded_number = value
        .as_f64()
        .is_some_and(|number| number.is_finite() && number.abs() <= 9_007_199_254_740_991.0);
    if !is_bounded_number {
        return Err(input.failure(
            marker.span.start,
            "Min/Max require a finite bounded JSON number",
        ));
    }
    let key = if marker.name == "Min" { "min" } else { "max" };
    Ok((key, value))
}

pub(super) fn schema(input: &Input<'_>, marker: &Marker) -> Result<Value, Failure> {
    if !marker.arguments.trim().is_empty() {
        return Err(input.failure(
            marker.span.start,
            crate::annotation_rules::AnnotationRules::no_arguments("Checked"),
        ));
    }
    Ok(json!({"fields":{}}))
}

pub(super) fn resolve(
    input: &Input<'_>,
    at: usize,
    metadata: &mut Map<String, Value>,
    schemas: &BTreeMap<String, Value>,
) -> Result<(), Failure> {
    if metadata.contains_key("query") && metadata.contains_key("valid") {
        return Err(input.failure(at, "Check belongs on the service before a repository query"));
    }
    if let Some(valid) = metadata.get_mut("valid") {
        let name = valid["type"].as_str().unwrap_or_default();
        valid["schema"] = schemas
            .get(name)
            .cloned()
            .ok_or_else(|| input.failure(at, "Check requires a source-local Checked type"))?;
    }
    Ok(())
}

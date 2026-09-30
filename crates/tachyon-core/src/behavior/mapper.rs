use super::model_mapping::ModelMapping;
use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};

struct MappingLimits;

impl MappingLimits {
    const MAX_FIELDS: usize = 32;
    const MAX_DEPTH: usize = 8;
    const MAX_NAME: usize = 256;
    const JSON_TYPE_NAMES: [&str; 13] = [
        "unknown",
        "any",
        "object",
        "Object",
        "BehaviorJsonValue",
        "dict",
        "Any",
        "array",
        "mixed",
        "BehaviorValue",
        "Promise",
        "Record",
        "Hash",
    ];
}

pub(super) struct ConvertSignature<'a> {
    pub method: String,
    pub parameter_types: Vec<&'a str>,
    pub return_type: &'a str,
}

pub(super) fn mapping(
    input: &Input<'_>,
    marker: &Marker,
    signature: &ConvertSignature<'_>,
) -> Result<Value, Failure> {
    let fail = |message: String| {
        input.failure(
            marker.span.start,
            format!("{}: {message}", signature.method),
        )
    };
    let written = super::literal::parse(&marker.arguments).ok();
    if written
        .as_ref()
        .is_some_and(|value| value.get("argument").is_some())
    {
        return Err(fail(
            "Convert reads its one parameter, so it has no \"argument\" key: remove it".to_owned(),
        ));
    }
    if signature.parameter_types.len() != 1 {
        return Err(fail(
            "A Convert method takes exactly one parameter: the value it converts".to_owned(),
        ));
    }
    let mut object = options(input, marker, written)?;
    let models = match object.remove("models") {
        Some(Value::Object(models)) => models,
        _ => Map::new(),
    };
    let mapping = ModelMapping::new(&models);
    let source = mapping
        .model_in(signature.parameter_types[0])
        .map_err(fail)?;
    let target = mapping.model_in(signature.return_type).map_err(fail)?;
    match (source, target) {
        (Some(source), Some(target)) => {
            if input.extension() == "rb"
                && object.get("realm").and_then(Value::as_str) == Some("tac")
            {
                return Err(fail("Typed-model converters in Ruby Tac require a native model reader; use an explicit JSON-value field plan until that adapter is available".to_owned()));
            }
            let written = written_entries(input, marker, object.get("fields"), 0)?;
            let implicit = mapping.entries(&source, &target, &written).map_err(fail)?;
            Ok(typed(written, implicit, &target, object.get("realm")))
        }
        (None, None) => untyped(input, marker, signature, &object),
        (Some(model), None) | (None, Some(model)) => Err(fail(format!(
            "A mapping method converts a model into a model, or a JSON value into a JSON value: {model} is a model, so declare the other side with a model type too"
        ))),
    }
}

fn options(
    input: &Input<'_>,
    marker: &Marker,
    written: Option<Value>,
) -> Result<Map<String, Value>, Failure> {
    let object = match written {
        Some(Value::Object(object)) => object,
        None if marker.arguments.trim().is_empty() => Map::new(),
        _ => {
            return Err(input.failure(
                marker.span.start,
                "Convert takes a strict JSON object with a \"fields\" list",
            ));
        }
    };
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "fields" | "models" | "realm"))
    {
        return Err(input.failure(
            marker.span.start,
            "Convert requires exactly these keys: fields",
        ));
    }
    Ok(object)
}

fn typed(
    mut written: Vec<Value>,
    implicit: Vec<Value>,
    target: &str,
    realm: Option<&Value>,
) -> Value {
    written.extend(implicit);
    serde_json::json!({"argument": 0, "fields": written, "result": target, "realm": realm.and_then(Value::as_str).unwrap_or("yon")})
}

fn untyped(
    input: &Input<'_>,
    marker: &Marker,
    signature: &ConvertSignature<'_>,
    object: &Map<String, Value>,
) -> Result<Value, Failure> {
    let types = [signature.parameter_types[0], signature.return_type];
    if let Some(model) = types
        .into_iter()
        .find(|kind| names_a_model(input.extension(), kind))
    {
        return Err(input.failure(
            marker.span.start,
            format!("Convert maps JSON values or models, and '{model}' is neither: declare it in the models folder, or use the language's JSON value type"),
        ));
    }
    if !object.contains_key("fields") {
        return Err(input.failure(
            marker.span.start,
            "This mapping method reads and writes JSON values, so Convert lists its field plan: {\"fields\":[{\"source\":\"name\",\"target\":\"title\"}]}. Declare model types to copy same-named fields without a plan",
        ));
    }
    let entries = written_entries(input, marker, object.get("fields"), 1)?;
    Ok(serde_json::json!({"argument": 0, "fields": entries}))
}

fn written_entries(
    input: &Input<'_>,
    marker: &Marker,
    fields: Option<&Value>,
    least: usize,
) -> Result<Vec<Value>, Failure> {
    let empty = Value::Array(Vec::new());
    let Some(entries) = fields.unwrap_or(&empty).as_array() else {
        return Err(too_many_fields(input, marker));
    };
    if entries.len() < least || entries.len() > MappingLimits::MAX_FIELDS {
        return Err(too_many_fields(input, marker));
    }
    let mut targets = std::collections::BTreeSet::new();
    for entry in entries {
        let plan = entry.as_object().ok_or_else(|| {
            input.failure(marker.span.start, "Convert.fields entries are objects")
        })?;
        field_plan(input, marker, plan)?;
        let target = plan["target"].as_str().unwrap_or_default();
        if !targets.insert(target.to_owned()) {
            return Err(input.failure(
                marker.span.start,
                format!("Convert writes the target '{target}' twice"),
            ));
        }
    }
    Ok(entries.clone())
}

fn names_a_model(language: &str, type_text: &str) -> bool {
    if !matches!(language, "ts" | "py" | "php" | "go" | "rb") {
        return false;
    }
    let text = type_text.trim().trim_start_matches(['?', '*']);
    let name = text
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .next()
        .unwrap_or("");
    let capitalized = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_uppercase());
    capitalized && super::identifier(name) && !MappingLimits::JSON_TYPE_NAMES.contains(&name)
}

fn too_many_fields(input: &Input<'_>, marker: &Marker) -> Failure {
    input.failure(
        marker.span.start,
        "Convert.fields lists 1 through 32 target field plans",
    )
}

fn field_plan(
    input: &Input<'_>,
    marker: &Marker,
    plan: &Map<String, Value>,
) -> Result<(), Failure> {
    let fail = |message: &str| input.failure(marker.span.start, message.to_owned());
    for key in plan.keys() {
        if !matches!(key.as_str(), "target" | "source" | "constant") {
            return Err(fail(
                "Convert entries take target and one of source or constant",
            ));
        }
    }
    let target = plan["target"].as_str().unwrap_or_default();
    if target.is_empty() || target.len() > MappingLimits::MAX_NAME {
        return Err(fail("Convert.target names one bounded output field"));
    }
    match (plan.get("source"), plan.get("constant")) {
        (Some(source), None) => {
            let Some(path) = source.as_str() else {
                return Err(fail("Convert.source is a dotted field path"));
            };
            if valid_source_path(path) {
                Ok(())
            } else {
                Err(fail(
                    "Convert.source reads at most 8 bounded nested field names",
                ))
            }
        }
        (None, Some(constant)) => valid_constant(constant).map_err(fail),
        _ => Err(fail(
            "Convert entries take exactly one of source or constant",
        )),
    }
}

fn valid_source_path(path: &str) -> bool {
    let segments: Vec<&str> = path.split('.').collect();
    if segments.is_empty() || segments.len() > MappingLimits::MAX_DEPTH {
        return false;
    }
    for segment in segments {
        if segment.is_empty() || segment.len() > MappingLimits::MAX_NAME {
            return false;
        }
    }
    true
}

fn valid_constant(constant: &Value) -> Result<(), &'static str> {
    match constant {
        Value::String(value) if value.len() > MappingLimits::MAX_NAME => {
            Err("Convert.constant carries a bounded literal")
        }
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(value) if value.as_f64().is_some_and(f64::is_finite) => Ok(()),
        _ => Err("Convert.constant carries a bounded JSON scalar"),
    }
}

pub(super) struct Plan {
    pub target: String,
    pub source: Option<Vec<String>>,
    pub constant: Value,
    pub fields: Option<Vec<Plan>>,
}

pub(super) fn plan(policy: &Value) -> Result<(usize, Vec<Plan>), String> {
    let argument = policy["argument"]
        .as_u64()
        .and_then(|index| usize::try_from(index).ok())
        .ok_or("Convert argument missing")?;
    let entries = policy["fields"]
        .as_array()
        .ok_or("Convert fields missing")?;
    let mut fields = Vec::with_capacity(entries.len());
    for entry in entries {
        fields.push(field_from_entry(entry)?);
    }
    Ok((argument, fields))
}

fn field_from_entry(entry: &Value) -> Result<Plan, String> {
    let target = entry["target"].as_str().ok_or("Convert target missing")?;
    let source = entry
        .get("source")
        .and_then(Value::as_str)
        .map(|path| path.split('.').map(str::to_owned).collect());
    Ok(Plan {
        target: target.to_owned(),
        source,
        constant: entry.get("constant").cloned().unwrap_or(Value::Null),
        fields: entry
            .get("fields")
            .and_then(Value::as_array)
            .map(|entries| entries.iter().map(field_from_entry).collect())
            .transpose()?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::behavior::lower;
    use std::path::Path;

    fn lower_tac_js(source: &str) -> Result<Option<String>, crate::Failure> {
        lower(Path::new("tac.js"), source, "tac")
    }

    fn source(plan: &str) -> String {
        format!(
            "@Converter()\nclass ProductMapper {{\n@Convert({plan})\nasync toSummary(product) {{}}\n}}\n"
        )
    }

    #[test]
    fn a_mapping_plan_reaches_the_registration() {
        let plan = "{\"fields\":[{\"source\":\"detail.id\",\"target\":\"productId\"},{\"target\":\"kind\",\"constant\":\"product\"}]}";
        let output = lower_tac_js(&source(plan))
            .expect("valid mapping")
            .expect("lowered");
        assert!(output.contains("\"mapping\""), "{output}");
        assert!(output.contains("detail.id"), "{output}");
    }

    #[test]
    fn the_one_parameter_is_the_converted_value() {
        let plan = "{\"fields\":[{\"source\":\"id\",\"target\":\"id\"}]}";
        let output = lower_tac_js(&source(plan))
            .expect("valid mapping")
            .expect("lowered");
        assert!(output.contains("\"argument\":0"), "{output}");
    }

    #[test]
    fn invalid_plans_fail_closed() {
        for plan in [
            "{\"fields\":[]}",
            "{\"fields\":[{\"target\":\"id\"}]}",
            "{\"fields\":[{\"target\":\"id\",\"source\":\"a\",\"constant\":1}]}",
            "{\"fields\":[{\"target\":\"\",\"source\":\"a\"}]}",
            "{\"fields\":[{\"target\":\"id\",\"source\":\"a..b\"}]}",
            "{\"fields\":[{\"target\":\"id\",\"source\":\"a.b.c.d.e.f.g.h.i\"}]}",
            "{\"fields\":[{\"target\":\"id\",\"source\":\"a\"},{\"target\":\"id\",\"source\":\"b\"}]}",
            "{\"fields\":[{\"target\":\"id\",\"constant\":{}}]}",
            "{\"fields\":[{\"target\":\"id\",\"source\":\"a\",\"extra\":1}]}",
            "{\"argument\":0,\"fields\":[{\"target\":\"id\",\"source\":\"a\"}]}",
        ] {
            assert!(lower_tac_js(&source(plan)).is_err(), "accepted {plan}");
        }
    }

    #[test]
    fn convert_takes_one_json_parameter() {
        let plan = "{\"fields\":[{\"source\":\"id\",\"target\":\"id\"}]}";
        let two = format!(
            "@Converter()\nclass ProductMapper {{\n@Convert({plan})\nasync toSummary(product, other) {{}}\n}}\n"
        );
        let failure = lower_tac_js(&two).expect_err("two parameters");
        assert!(
            failure.to_string().contains("exactly one parameter"),
            "{failure}"
        );
        let model = format!(
            "@Converter()\nclass ProductMapper {{\n@Convert({plan})\nasync toSummary(product: Product): Promise<unknown> {{}}\n}}\n"
        );
        let failure = lower(Path::new("tac.ts"), &model, "tac").expect_err("model parameter");
        assert!(failure.to_string().contains("'Product'"), "{failure}");
        let old = "{\"argument\":0,\"fields\":[{\"source\":\"id\",\"target\":\"id\"}]}";
        let failure = lower_tac_js(&source(old)).expect_err("argument key");
        assert!(
            failure.to_string().contains("no \"argument\" key"),
            "{failure}"
        );
    }

    #[test]
    fn a_mapping_method_writes_no_body() {
        let occupied = "@Converter()\nclass ProductMapper {\n@Convert({\"fields\":[{\"source\":\"id\",\"target\":\"id\"}]})\nasync toSummary(product) { return product; }\n}\n";
        assert!(lower_tac_js(occupied).is_err());
    }
}

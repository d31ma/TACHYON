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
    pub parameter_types: Vec<&'a str>,
    pub return_type: &'a str,
}

pub(super) fn mapping(
    input: &Input<'_>,
    marker: &Marker,
    signature: &ConvertSignature<'_>,
) -> Result<Value, Failure> {
    let written = super::literal::parse(&marker.arguments).ok();
    if written
        .as_ref()
        .is_some_and(|value| value.get("argument").is_some())
    {
        return Err(input.failure(
            marker.span.start,
            "Convert reads its one parameter, so it has no \"argument\" key: remove it",
        ));
    }
    if signature.parameter_types.len() != 1 {
        return Err(input.failure(
            marker.span.start,
            "A Convert method takes exactly one parameter: the value it converts",
        ));
    }
    let types = [signature.parameter_types[0], signature.return_type];
    if let Some(model) = types
        .into_iter()
        .find(|kind| names_a_model(input.extension(), kind))
    {
        return Err(input.failure(
            marker.span.start,
            format!("Convert reads and writes JSON values; a model type such as '{model}' in its signature is not supported yet. Declare the parameter and result with the language's JSON value type"),
        ));
    }
    let mut object = super::object(input, marker, &["fields"])?;
    let Some(fields) = object["fields"].as_array() else {
        return Err(too_many_fields(input, marker));
    };
    if fields.is_empty() || fields.len() > MappingLimits::MAX_FIELDS {
        return Err(too_many_fields(input, marker));
    }
    let entries = fields.clone();
    let mut targets = std::collections::BTreeSet::new();
    for entry in &entries {
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
    object.insert("argument".to_owned(), Value::from(0));
    object.insert("fields".to_owned(), Value::Array(entries));
    Ok(Value::Object(object))
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

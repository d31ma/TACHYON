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
    let written = super::literal::parse(&marker.arguments).ok();
    signature.check_signature(input, marker, written.as_ref())?;
    let object = ConvertSignature::options(input, marker, written)?;
    signature.resolve_models(input, marker, object)
}

impl ConvertSignature<'_> {
    fn failure(&self, input: &Input<'_>, marker: &Marker, message: &str) -> Failure {
        input.failure(marker.span.start, format!("{}: {message}", self.method))
    }

    fn check_signature(
        &self,
        input: &Input<'_>,
        marker: &Marker,
        written: Option<&Value>,
    ) -> Result<(), Failure> {
        self.check_argument(input, marker, written)?;
        self.check_arity(input, marker)
    }

    fn check_argument(
        &self,
        input: &Input<'_>,
        marker: &Marker,
        written: Option<&Value>,
    ) -> Result<(), Failure> {
        if written.is_some_and(|value| value.get("argument").is_some()) {
            return Err(self.failure(
                input,
                marker,
                "Convert reads its one parameter, so it has no \"argument\" key: remove it",
            ));
        }
        Ok(())
    }

    fn check_arity(&self, input: &Input<'_>, marker: &Marker) -> Result<(), Failure> {
        if self.parameter_types.len() != 1 {
            return Err(self.failure(
                input,
                marker,
                "A Convert method takes exactly one parameter: the value it converts",
            ));
        }
        Ok(())
    }

    fn resolve_models(
        &self,
        input: &Input<'_>,
        marker: &Marker,
        mut object: Map<String, Value>,
    ) -> Result<Value, Failure> {
        let models = object
            .remove("models")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        let mapping = ModelMapping::new(&models);
        let source = mapping
            .model_in(self.parameter_types[0])
            .map_err(|message| self.failure(input, marker, &message))?;
        let target = mapping
            .model_in(self.return_type)
            .map_err(|message| self.failure(input, marker, &message))?;
        self.model_policy(input, marker, &object, &mapping, (source, target))
    }

    fn model_policy(
        &self,
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
        mapping: &ModelMapping<'_>,
        models: (Option<String>, Option<String>),
    ) -> Result<Value, Failure> {
        match models {
            (Some(source), Some(target)) => {
                self.typed_policy(input, marker, object, mapping, &source, &target)
            }
            (None, None) => self.untyped(input, marker, object),
            (Some(model), None) | (None, Some(model)) => {
                Err(self.mixed_model(input, marker, &model))
            }
        }
    }

    fn mixed_model(&self, input: &Input<'_>, marker: &Marker, model: &str) -> Failure {
        let message = format!(
            "A mapping method converts a model into a model, or a JSON value into a JSON value: {model} is a model, so declare the other side with a model type too"
        );
        self.failure(input, marker, &message)
    }

    fn typed_policy(
        &self,
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
        mapping: &ModelMapping<'_>,
        source: &str,
        target: &str,
    ) -> Result<Value, Failure> {
        let written = Self::written_entries(input, marker, object.get("fields"), 0)?;
        let implicit = mapping
            .entries(source, target, &written)
            .map_err(|message| self.failure(input, marker, &message))?;
        let policy = Self::typed(written, implicit, target, object.get("realm"));
        Ok(Self::native_policy(input, object, mapping, policy))
    }

    fn native_policy(
        input: &Input<'_>,
        object: &Map<String, Value>,
        mapping: &ModelMapping<'_>,
        mut policy: Value,
    ) -> Value {
        if input.extension() == "rb" && object.get("realm").and_then(Value::as_str) == Some("tac") {
            policy["models"] = mapping.reader_models();
        }
        policy
    }

    fn options(
        input: &Input<'_>,
        marker: &Marker,
        written: Option<Value>,
    ) -> Result<Map<String, Value>, Failure> {
        let object = Self::read_options(input, marker, written)?;
        Self::check_keys(input, marker, &object)?;
        Ok(object)
    }

    fn read_options(
        input: &Input<'_>,
        marker: &Marker,
        written: Option<Value>,
    ) -> Result<Map<String, Value>, Failure> {
        match written {
            Some(Value::Object(object)) => Ok(object),
            None if marker.arguments.trim().is_empty() => Ok(Map::new()),
            _ => Err(input.failure(
                marker.span.start,
                "Convert takes a strict JSON object with a \"fields\" list",
            )),
        }
    }

    fn check_keys(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        if object
            .keys()
            .any(|key| !["fields", "models", "realm"].contains(&key.as_str()))
        {
            return Err(input.failure(
                marker.span.start,
                "Convert requires exactly these keys: fields",
            ));
        }
        Ok(())
    }

    fn typed(
        mut written: Vec<Value>,
        implicit: Vec<Value>,
        target: &str,
        realm: Option<&Value>,
    ) -> Value {
        written.extend(implicit);
        serde_json::json!({"argument":0,"fields":written,"result":target,"realm":realm.and_then(Value::as_str).unwrap_or("yon")})
    }

    fn untyped(
        &self,
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<Value, Failure> {
        let types = [self.parameter_types[0], self.return_type];
        let model = types
            .into_iter()
            .find(|kind| names_a_model(input.extension(), kind));
        Self::json_types(input, marker, model)?;
        Self::json_fields(input, marker, object)?;
        let entries = Self::written_entries(input, marker, object.get("fields"), 1)?;
        Ok(serde_json::json!({"argument":0,"fields":entries}))
    }

    fn json_types(input: &Input<'_>, marker: &Marker, model: Option<&str>) -> Result<(), Failure> {
        if let Some(model) = model {
            return Err(input.failure(marker.span.start,format!("Convert maps JSON values or models, and '{model}' is neither: declare it in the models folder, or use the language's JSON value type")));
        }
        Ok(())
    }

    fn json_fields(
        input: &Input<'_>,
        marker: &Marker,
        object: &Map<String, Value>,
    ) -> Result<(), Failure> {
        if !object.contains_key("fields") {
            return Err(input.failure(marker.span.start,"This mapping method reads and writes JSON values, so Convert lists its field plan: {\"fields\":[{\"source\":\"name\",\"target\":\"title\"}]}. Declare model types to copy same-named fields without a plan"));
        }
        Ok(())
    }

    fn written_entries(
        input: &Input<'_>,
        marker: &Marker,
        fields: Option<&Value>,
        least: usize,
    ) -> Result<Vec<Value>, Failure> {
        let empty = Value::Array(Vec::new());
        let entries = fields
            .unwrap_or(&empty)
            .as_array()
            .ok_or_else(|| too_many_fields(input, marker))?;
        Self::field_count(input, marker, entries.len(), least)?;
        Self::check_entries(input, marker, entries)?;
        Ok(entries.clone())
    }

    fn field_count(
        input: &Input<'_>,
        marker: &Marker,
        count: usize,
        least: usize,
    ) -> Result<(), Failure> {
        if count < least || count > MappingLimits::MAX_FIELDS {
            return Err(too_many_fields(input, marker));
        }
        Ok(())
    }

    fn check_entries(input: &Input<'_>, marker: &Marker, entries: &[Value]) -> Result<(), Failure> {
        let mut targets = std::collections::BTreeSet::new();
        for entry in entries {
            let plan = entry.as_object().ok_or_else(|| {
                input.failure(marker.span.start, "Convert.fields entries are objects")
            })?;
            field_plan(input, marker, plan)?;
            let target = plan["target"].as_str().unwrap_or_default();
            Self::unique_target(input, marker, &mut targets, target)?;
        }
        Ok(())
    }

    fn unique_target(
        input: &Input<'_>,
        marker: &Marker,
        targets: &mut std::collections::BTreeSet<String>,
        target: &str,
    ) -> Result<(), Failure> {
        if !targets.insert(target.to_owned()) {
            return Err(input.failure(
                marker.span.start,
                format!("Convert writes the target '{target}' twice"),
            ));
        }
        Ok(())
    }
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
    let fields: Result<Vec<_>, String> = entries.iter().map(Plan::from_entry).collect();
    let fields = fields?;
    Ok((argument, fields))
}

impl Plan {
    fn from_entry(entry: &Value) -> Result<Self, String> {
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
                .map(|entries| entries.iter().map(Self::from_entry).collect())
                .transpose()?,
        })
    }
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

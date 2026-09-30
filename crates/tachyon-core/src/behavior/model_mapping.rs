use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

pub(super) struct ModelMapping<'a> {
    models: &'a Map<String, Value>,
}

impl<'a> ModelMapping<'a> {
    const MAX_DEPTH: usize = 8;
    const ANY: &'static str = "any";
    const INTEGER: &'static str = "integer";
    const NUMBER: &'static str = "number";
    const LIST: &'static str = "[]";
    const SCALARS: [&'static str; 5] = ["any", "string", "integer", "number", "boolean"];
    const LIST_WORDS: [&'static str; 9] = [
        "List",
        "list",
        "Array",
        "Vec",
        "Iterable",
        "Sequence",
        "Set",
        "seq",
        "IEnumerable",
    ];

    pub(super) fn new(models: &'a Map<String, Value>) -> Self {
        Self { models }
    }

    pub(super) fn model_in(&self, type_text: &str) -> Result<Option<String>, String> {
        let words: Vec<&str> = type_text
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .filter(|word| !word.is_empty())
            .collect();
        let Some(model) = words.iter().find(|word| self.models.contains_key(**word)) else {
            return Ok(None);
        };
        let listed = type_text.contains(Self::LIST)
            || words.iter().any(|word| Self::LIST_WORDS.contains(word));
        if listed {
            return Err(format!(
                "A mapping method converts one {model}, never a list: declare one model in and one model out, and map a list item by item"
            ));
        }
        Ok(Some((*model).to_owned()))
    }

    pub(super) fn entries(
        &self,
        source: &str,
        target: &str,
        written: &[Value],
    ) -> Result<Vec<Value>, String> {
        let written: BTreeSet<String> = written
            .iter()
            .filter_map(|entry| entry["target"].as_str().map(str::to_owned))
            .collect();
        let target_fields = self.fields(target);
        if let Some(unknown) = written
            .iter()
            .find(|name| !target_fields.contains_key(*name))
        {
            return Err(format!(
                "Convert writes '{unknown}', but {target} has no field '{unknown}'. Name a field of {target} as the target"
            ));
        }
        self.nested(source, target, &written, 0)
    }

    fn nested(
        &self,
        source: &str,
        target: &str,
        written: &BTreeSet<String>,
        depth: usize,
    ) -> Result<Vec<Value>, String> {
        if depth > Self::MAX_DEPTH {
            return Err(format!(
                "{target} nests models more than 8 levels deep: map the deeper levels with their own method"
            ));
        }
        let source_fields = self.fields(source);
        let mut entries = Vec::new();
        let mut missing = Vec::new();
        for (name, target_shape) in &self.fields(target) {
            if written.contains(name) {
                continue;
            }
            let Some(source_shape) = source_fields.get(name) else {
                missing.push(format!("'{name}'"));
                continue;
            };
            let described = (source, target, name.as_str());
            let mut entry = json!({"source": name, "target": name});
            if let Some(fields) = self.fitted(source_shape, target_shape, described, depth)? {
                entry["fields"] = Value::Array(fields);
            }
            entries.push(entry);
        }
        if missing.is_empty() {
            return Ok(entries);
        }
        Err(format!(
            "{target} has fields that {source} does not have: {}. Tachyon never leaves a target field unset: give {source} the same field, or add a Convert entry for each (a source path, or a constant such as null)",
            missing.join(", ")
        ))
    }

    fn fitted(
        &self,
        source: &Value,
        target: &Value,
        described: (&str, &str, &str),
        depth: usize,
    ) -> Result<Option<Vec<Value>>, String> {
        let (source_model, target_model, name) = described;
        let source_text = source.as_str().unwrap_or(Self::ANY);
        let target_text = target.as_str().unwrap_or(Self::ANY);
        let (source_required, source_core) = Self::parts(source_text);
        let (target_required, target_core) = Self::parts(target_text);
        if target_required && !source_required {
            return Err(format!(
                "{target_model}.{name} always has a value, but {source_model}.{name} may be missing. Make {target_model}.{name} optional, or fill it with a Convert entry"
            ));
        }
        self.fitted_core(source_core, target_core, depth).map_err(|message| {
            if !message.is_empty() {
                return format!("{target_model}.{name}: {message}");
            }
            format!(
                "{target_model}.{name} is {}, but {source_model}.{name} is {}. Give both fields the same type, or fill {target_model}.{name} with a Convert entry",
                Self::plain_name(target_core),
                Self::plain_name(source_core)
            )
        })
    }

    fn fitted_core(
        &self,
        source: &str,
        target: &str,
        depth: usize,
    ) -> Result<Option<Vec<Value>>, String> {
        if source.starts_with('?') && !target.starts_with('?') {
            return Err("An optional list item cannot fill a required item".to_owned());
        }
        let source = source.trim_start_matches('?');
        let target = target.trim_start_matches('?');
        let widened = [source, target] == [Self::INTEGER, Self::NUMBER];
        if target == Self::ANY || source == target && Self::SCALARS.contains(&source) || widened {
            return Ok(None);
        }
        if let (Some(source_item), Some(target_item)) = (
            source.strip_prefix(Self::LIST),
            target.strip_prefix(Self::LIST),
        ) {
            return self.fitted_core(source_item, target_item, depth);
        }
        let models = [source, target].map(|name| self.models.contains_key(name));
        if models == [true, true] {
            return self
                .nested(source, target, &BTreeSet::new(), depth + 1)
                .map(Some);
        }
        Err(String::new())
    }

    fn fields(&self, model: &str) -> Map<String, Value> {
        self.models
            .get(model)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    }

    fn parts(text: &str) -> (bool, &str) {
        let required = text.starts_with('!');
        let core = text.trim_start_matches(['!', '?']);
        (required, core)
    }

    fn plain_name(core: &str) -> String {
        if let Some(item) = core.strip_prefix(Self::LIST) {
            return format!("a list of {}", Self::plain_name(item));
        }
        match core {
            "any" => "any JSON value".to_owned(),
            "string" => "text".to_owned(),
            "integer" => "a whole number".to_owned(),
            "number" => "a number".to_owned(),
            "boolean" => "true or false".to_owned(),
            model => format!("a {model}"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::ModelMapping;
    use serde_json::{Value, json};

    fn entries(models: &Value, written: &[Value]) -> Result<Vec<Value>, String> {
        ModelMapping::new(models.as_object().expect("models"))
            .entries("Product", "Summary", written)
    }

    #[test]
    fn same_names_widen_numbers_and_drop_unmatched_source_fields() {
        let models = json!({"Product":{"id":"!integer","name":"!string","secret":"!string"},
            "Summary":{"id":"!number","name":"?string"}});
        assert_eq!(
            entries(&models, &[]).expect("compatible"),
            vec![
                json!({"source":"id","target":"id"}),
                json!({"source":"name","target":"name"})
            ]
        );
    }

    #[test]
    fn explicit_targets_override_implicit_types_and_missing_fields() {
        let models =
            json!({"Product":{"id":"!string"},"Summary":{"id":"!integer","name":"!string"}});
        let written = vec![
            json!({"target":"id","constant":7}),
            json!({"target":"name","constant":"product"}),
        ];
        assert!(entries(&models, &written).expect("overrides").is_empty());
        assert!(
            entries(&models, &[json!({"target":"unknown","constant":null})])
                .expect_err("unknown target")
                .contains("has no field 'unknown'")
        );
    }

    #[test]
    fn all_missing_target_fields_are_named_and_case_styles_are_exact() {
        let models = json!({"Product":{"product_id":"!integer"},
            "Summary":{"productId":"!integer","title":"!string"}});
        let error = entries(&models, &[]).expect_err("missing fields");
        assert!(error.contains("'productId', 'title'"), "{error}");
    }

    #[test]
    fn incompatible_shapes_and_optional_values_fail_closed() {
        for (source, target) in [
            ("string", "integer"),
            ("number", "integer"),
            ("any", "string"),
            ("?string", "!string"),
            ("[]?string", "[]string"),
            ("[]string", "string"),
            ("string", "[]string"),
        ] {
            let models = json!({"Product":{"value":source},"Summary":{"value":target}});
            assert!(
                entries(&models, &[]).is_err(),
                "accepted {source} -> {target}"
            );
        }
    }

    #[test]
    fn nested_models_and_model_lists_get_recursive_projection_plans() {
        let models = json!({"Product":{"detail":"!Detail","items":"![]Detail"},
            "Summary":{"detail":"!ShortDetail","items":"![]ShortDetail"},
            "Detail":{"name":"!string","private":"!string"},"ShortDetail":{"name":"!string"}});
        let nested = vec![json!({"source":"name","target":"name"})];
        assert_eq!(
            entries(&models, &[]).expect("nested"),
            vec![
                json!({"source":"detail","target":"detail","fields":nested}),
                json!({"source":"items","target":"items","fields":nested})
            ]
        );
    }

    #[test]
    fn recursive_models_have_a_bounded_plan() {
        let models = json!({"Product":{"next":"?Product"},"Summary":{"next":"?Summary"}});
        assert!(entries(&models, &[]).is_err());
    }

    #[test]
    fn methods_map_one_model_and_allow_language_result_wrappers() {
        let models = json!({"Product":{},"Summary":{}});
        let mapping = ModelMapping::new(models.as_object().expect("models"));
        for text in [
            "Promise<Summary>",
            "BehaviorResult<Summary>",
            "(Summary, error)",
        ] {
            assert_eq!(
                mapping.model_in(text).expect("model"),
                Some("Summary".to_owned())
            );
        }
        for text in ["List<Product>", "[]Product", "Vec<Product>"] {
            assert!(mapping.model_in(text).is_err(), "accepted {text}");
        }
    }
}

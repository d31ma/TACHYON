use serde_json::{Map, Value};
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
            .split(Self::word_boundary)
            .filter(|word| !word.is_empty())
            .collect();
        let model = words.iter().find(|word| self.models.contains_key(**word));
        model
            .map(|model| Self::one_model(type_text, &words, model))
            .transpose()
    }

    fn word_boundary(character: char) -> bool {
        !character.is_ascii_alphanumeric() && character != '_'
    }

    fn one_model(type_text: &str, words: &[&str], model: &str) -> Result<String, String> {
        if type_text.contains(Self::LIST)
            || words.iter().any(|word| Self::LIST_WORDS.contains(word))
        {
            return Err(format!(
                "A mapping method converts one {model}, never a list: declare one model in and one model out, and map a list item by item"
            ));
        }
        Ok(model.to_owned())
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
        Self::known_targets(&written, &target_fields, target)?;
        self.nested(source, target, &written, 0)
    }

    fn known_targets(
        written: &BTreeSet<String>,
        fields: &Map<String, Value>,
        target: &str,
    ) -> Result<(), String> {
        if let Some(unknown) = written.iter().find(|name| !fields.contains_key(*name)) {
            return Err(format!(
                "Convert writes '{unknown}', but {target} has no field '{unknown}'. Name a field of {target} as the target"
            ));
        }
        Ok(())
    }

    fn nested(
        &self,
        source: &str,
        target: &str,
        written: &BTreeSet<String>,
        depth: usize,
    ) -> Result<Vec<Value>, String> {
        Self::bounded_depth(target, depth)?;
        let source_fields = self.fields(source);
        let target_fields = self.fields(target);
        let remaining: Vec<_> = target_fields
            .iter()
            .filter(|(name, _)| !written.contains(*name))
            .collect();
        let entries: Result<Vec<_>, String> = remaining
            .iter()
            .map(|(name, shape)| {
                source_fields
                    .get(*name)
                    .map(|source_shape| {
                        self.project_field(source_shape, shape, (source, target, name), depth)
                    })
                    .transpose()
            })
            .collect();
        let entries: Vec<Value> = entries?.into_iter().flatten().collect();
        let missing: Vec<_> = remaining
            .iter()
            .filter(|(name, _)| !source_fields.contains_key(*name))
            .map(|(name, _)| format!("'{name}'"))
            .collect();
        Self::required_fields(source, target, &missing)?;
        Ok(entries)
    }

    fn bounded_depth(target: &str, depth: usize) -> Result<(), String> {
        if depth > Self::MAX_DEPTH {
            return Err(format!(
                "{target} nests models more than 8 levels deep: map the deeper levels with their own method"
            ));
        }
        Ok(())
    }

    fn required_fields(source: &str, target: &str, missing: &[String]) -> Result<(), String> {
        if !missing.is_empty() {
            return Err(format!(
                "{target} has fields that {source} does not have: {}. Tachyon never leaves a target field unset: give {source} the same field, or add a Convert entry for each (a source path, or a constant such as null)",
                missing.join(", ")
            ));
        }
        Ok(())
    }

    fn project_field(
        &self,
        source: &Value,
        target: &Value,
        described: (&str, &str, &str),
        depth: usize,
    ) -> Result<Value, String> {
        let name = described.2;
        let fields = self.fitted(source, target, described, depth)?;
        let mut entry = Map::new();
        entry.insert("source".to_owned(), Value::String(name.to_owned()));
        entry.insert("target".to_owned(), Value::String(name.to_owned()));
        entry.extend(fields.map(|fields| ("fields".to_owned(), Value::Array(fields))));
        Ok(Value::Object(entry))
    }

    fn fitted(
        &self,
        source: &Value,
        target: &Value,
        described: (&str, &str, &str),
        depth: usize,
    ) -> Result<Option<Vec<Value>>, String> {
        let source_text = source.as_str().unwrap_or(Self::ANY);
        let target_text = target.as_str().unwrap_or(Self::ANY);
        let (source_required, source_core) = Self::parts(source_text);
        let (target_required, target_core) = Self::parts(target_text);
        Self::required_value(source_required, target_required, described)?;
        self.fitted_core(source_core, target_core, depth)
            .map_err(|message| Self::incompatible(&message, source_core, target_core, described))
    }

    fn required_value(
        source_required: bool,
        target_required: bool,
        described: (&str, &str, &str),
    ) -> Result<(), String> {
        let (source_model, target_model, name) = described;
        if target_required && !source_required {
            return Err(format!(
                "{target_model}.{name} always has a value, but {source_model}.{name} may be missing. Make {target_model}.{name} optional, or fill it with a Convert entry"
            ));
        }
        Ok(())
    }

    fn incompatible(
        message: &str,
        source: &str,
        target: &str,
        described: (&str, &str, &str),
    ) -> String {
        let (source_model, target_model, name) = described;
        if !message.is_empty() {
            return format!("{target_model}.{name}: {message}");
        }
        format!(
            "{target_model}.{name} is {}, but {source_model}.{name} is {}. Give both fields the same type, or fill {target_model}.{name} with a Convert entry",
            Self::plain_name(target),
            Self::plain_name(source)
        )
    }

    fn fitted_core(
        &self,
        source: &str,
        target: &str,
        depth: usize,
    ) -> Result<Option<Vec<Value>>, String> {
        Self::required_item(source, target)?;
        let source = source.trim_start_matches('?');
        let target = target.trim_start_matches('?');
        let scalar = Self::compatible_scalar(source, target);
        let items = source
            .strip_prefix(Self::LIST)
            .zip(target.strip_prefix(Self::LIST));
        let models = [source, target].map(|name| self.models.contains_key(name));
        self.fitted_shape(source, target, depth, scalar, items, models)
    }

    fn required_item(source: &str, target: &str) -> Result<(), String> {
        if source.starts_with('?') && !target.starts_with('?') {
            return Err("An optional list item cannot fill a required item".to_owned());
        }
        Ok(())
    }

    fn compatible_scalar(source: &str, target: &str) -> bool {
        let same_scalar = [source == target, Self::SCALARS.contains(&source)] == [true, true];
        [
            target == Self::ANY,
            same_scalar,
            [source, target] == [Self::INTEGER, Self::NUMBER],
        ]
        .contains(&true)
    }

    fn fitted_shape(
        &self,
        source: &str,
        target: &str,
        depth: usize,
        scalar: bool,
        items: Option<(&str, &str)>,
        models: [bool; 2],
    ) -> Result<Option<Vec<Value>>, String> {
        match (scalar, items, models) {
            (true, _, _) => Ok(None),
            (_, Some((source_item, target_item)), _) => {
                self.fitted_core(source_item, target_item, depth)
            }
            (_, _, [true, true]) => self
                .nested(source, target, &BTreeSet::new(), depth + 1)
                .map(Some),
            _ => Err(String::new()),
        }
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
        core.strip_prefix(Self::LIST).map_or_else(
            || Self::scalar_name(core),
            |item| format!("a list of {}", Self::plain_name(item)),
        )
    }

    fn scalar_name(core: &str) -> String {
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

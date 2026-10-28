use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};

pub(super) fn retryable(input: &Input<'_>, marker: &Marker) -> Result<Value, Failure> {
    let mut object = options(
        input,
        marker,
        &["maxRetries"],
        &["delayMs", "multiplier", "maxDelayMs", "jitterMs"],
    )?;
    super::bounded_integer(input, marker, &object, "maxRetries", 1, 8)?;
    object.entry("delayMs").or_insert(Value::from(0));
    object.entry("maxDelayMs").or_insert(Value::from(60_000));
    object.entry("jitterMs").or_insert(Value::from(0));
    super::bounded_integer(input, marker, &object, "delayMs", 0, 60_000)?;
    super::bounded_integer(input, marker, &object, "maxDelayMs", 0, 600_000)?;
    super::bounded_integer(input, marker, &object, "jitterMs", 0, 60_000)?;
    if object["delayMs"].as_u64() > object["maxDelayMs"].as_u64() {
        return Err(input.failure(marker.span.start, "Retry.delayMs cannot exceed maxDelayMs"));
    }
    let multiplier = object.entry("multiplier").or_insert(Value::from(1.0));
    let Some(multiplier) = multiplier.as_f64() else {
        return Err(bad_multiplier(input, marker));
    };
    if !(1.0..=10.0).contains(&multiplier) {
        return Err(bad_multiplier(input, marker));
    }
    object["multiplier"] = Value::from(multiplier);
    Ok(Value::Object(object))
}

fn bad_multiplier(input: &Input<'_>, marker: &Marker) -> Failure {
    input.failure(
        marker.span.start,
        "Retry.multiplier must be a number from 1 to 10",
    )
}

pub(super) fn concurrency_limit(
    input: &Input<'_>,
    language: &str,
    marker: &Marker,
) -> Result<Value, Failure> {
    let mut object = options(input, marker, &["limit"], &["policy"])?;
    super::bounded_integer(input, marker, &object, "limit", 1, 64)?;
    let policy = object.entry("policy").or_insert(Value::from("block"));
    let Some(policy @ ("block" | "reject")) = policy.as_str() else {
        return Err(input.failure(
            marker.span.start,
            "MaxAtOnce.policy must be block or reject",
        ));
    };
    if policy == "block" && matches!(language, "php" | "rb") {
        let name = if language == "php" { "PHP" } else { "Ruby" };
        return Err(input.failure(
            marker.span.start,
            format!(
                "{name} runs a single synchronous call at a time; MaxAtOnce.policy must be reject"
            ),
        ));
    }
    Ok(Value::Object(object))
}

pub(super) fn options(
    input: &Input<'_>,
    marker: &Marker,
    required: &[&str],
    optional: &[&str],
) -> Result<Map<String, Value>, Failure> {
    let value: Value = super::literal::parse(&marker.arguments).map_err(|_| {
        input.failure(
            marker.span.start,
            "Behavior configuration must be a strict JSON object, never an executable expression",
        )
    })?;
    let object = value.as_object().ok_or_else(|| {
        input.failure(
            marker.span.start,
            "Behavior configuration must be a JSON object",
        )
    })?;
    if !has_expected_keys(object, required, optional) {
        return Err(input.failure(
            marker.span.start,
            format!(
                "{} requires {} and accepts only {}",
                marker.name,
                required.join(", "),
                optional.join(", ")
            ),
        ));
    }
    Ok(object.clone())
}

fn has_expected_keys(object: &Map<String, Value>, required: &[&str], optional: &[&str]) -> bool {
    for key in required {
        if !object.contains_key(*key) {
            return false;
        }
    }
    for key in object.keys() {
        let key = key.as_str();
        if !required.contains(&key) && !optional.contains(&key) {
            return false;
        }
    }
    true
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

    fn lower_orders_method(body: &str) -> Result<String, crate::Failure> {
        let source = format!("class Orders {{\n{body}\n}}");
        Ok(lower_file("yon.js", &source, "yon")?.unwrap_or_default())
    }

    #[test]
    fn unsupported_test_annotations_name_the_supported_marker() {
        for marker in [
            "WebMvcTest",
            "WebFluxTest",
            "DataJpaTest",
            "JsonTest",
            "RestClientTest",
            "AutoConfigureMockMvc",
            "SpyBean",
            "MockitoSpyBean",
        ] {
            let source = format!("@{marker}()\nclass Catalog {{}}\n");
            let Err(failure) = lower_file("tac.js", &source, "tac") else {
                panic!("{marker} was accepted");
            };
            let message = &failure.diagnostics()[0].message;
            assert!(
                message.contains("write Test")
                    || message.contains("write Fake")
                    || message.contains("MockMvc"),
                "{marker}: {message}"
            );
            assert!(lower_file("tac.kt", &source, "tac").is_err(), "kt {marker}");
        }
    }

    #[test]
    fn ordinary_test_annotations_stay_inert() {
        for marker in ["BeforeEach", "AfterAll", "Disabled", "Sql", "Deprecated"] {
            let source = format!("@{marker}()\n@Service()\nclass CatalogService {{}}\n");
            assert!(
                lower_file("tac.js", &source, "tac").is_ok(),
                "{marker} should stay inert"
            );
        }
    }

    #[test]
    fn retry_defaults_are_explicit_in_the_lowered_record() {
        let output =
            lower_orders_method("@Retry({\"maxRetries\":3})\nasync send(id) { return id; }")
                .unwrap();
        assert!(output.contains("\"maxRetries\":3"), "{output}");
        assert!(output.contains("\"delayMs\":0"), "{output}");
        assert!(output.contains("\"multiplier\":1.0"), "{output}");
        assert!(output.contains("\"maxDelayMs\":60000"), "{output}");
    }

    #[test]
    fn retry_refuses_an_unbounded_or_inverted_policy() {
        for arguments in [
            "{\"maxRetries\":0}",
            "{\"maxRetries\":9}",
            "{\"maxRetries\":2,\"multiplier\":11}",
            "{\"maxRetries\":2,\"delayMs\":1000,\"maxDelayMs\":10}",
            "{\"delayMs\":10}",
            "{\"maxRetries\":2,\"unknown\":1}",
        ] {
            assert!(
                lower_orders_method(&format!(
                    "@Retry({arguments})\nasync send(id) {{ return id; }}"
                ))
                .is_err(),
                "{arguments} was accepted"
            );
        }
    }

    #[test]
    fn a_concurrency_limit_defaults_to_waiting_for_a_slot() {
        let output =
            lower_orders_method("@MaxAtOnce({\"limit\":4})\nasync send(id) { return id; }")
                .unwrap();
        assert!(output.contains("\"limit\":4"), "{output}");
        assert!(output.contains("\"policy\":\"block\""), "{output}");
    }

    #[test]
    fn description_is_bounded_literal_metadata_beside_a_declared_behavior() {
        let described = lower_file(
            "yon.js",
            "@Description('Charges the card and records the order')\n@CacheSettings({\"name\":\"orders\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8})\nclass Orders {\n@Cached\nasync find(id) { return id; }\n}",
            "yon",
        )
        .unwrap()
        .unwrap_or_default();
        assert!(
            described.contains("\"description\":\"Charges the card and records the order\""),
            "{described}"
        );
        for source in [
            "@Description('Only metadata')\nclass Orders {}",
            "@Description('has a \\\\ backslash')\n@Setup()\nclass Orders { static configure() { return {}; } }",
            "@Description('')\n@Setup()\nclass Orders { static configure() { return {}; } }",
        ] {
            assert!(
                lower_file("yon.js", source, "yon").is_err(),
                "accepted {source}"
            );
        }
    }

    #[test]
    fn autowired_fields_build_the_tac_constructor() {
        let source = "@Service()\nexport default class BasketService {\n@Needs()\nbasketRepository;\n@Needs()\n@Named(\"PricingDelegate\")\npricing;\ntotal = 0;\n}";
        let output = lower_file("tac.js", source, "tac")
            .unwrap_or_else(|failure| panic!("{failure:?}"))
            .unwrap_or_default();
        assert!(
            output.contains("constructor(basketRepository, pricing)"),
            "{output}"
        );
        assert!(output.contains("\"qualifiers\""), "{output}");
        assert!(!output.contains("@Needs"), "{output}");
        for invalid in [
            "@Service()\nclass BasketService {\n@Needs()\ntotal = 0;\n}",
            "@Service()\nclass BasketService {\n@Needs(BasketRepository)\nbasketRepository;\n}",
            "@Service()\nclass BasketService {\n@Named(\"not a name\")\nbasketRepository;\n}",
        ] {
            assert!(
                lower_file("tac.js", invalid, "tac").is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[test]
    fn php_refuses_a_waiting_concurrency_limit() {
        let source = "#[Service]\nclass Orders {\n#[MaxAtOnce({\"limit\":2,\"policy\":\"block\"})]\npublic function send(int $id): int { return $id; }\n}";
        let failure = lower_file("yon.php", source, "yon").unwrap_err();
        assert!(
            format!("{failure:?}").contains("must be reject"),
            "{failure:?}"
        );
    }
}

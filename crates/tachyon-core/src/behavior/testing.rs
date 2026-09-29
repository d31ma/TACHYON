use super::{Input, Marker};
use crate::Failure;
use serde_json::{Map, Value};

struct TestPropertyLimits;

impl TestPropertyLimits {
    const MAX_PROPERTIES: usize = 64;
}

pub(super) fn unit(input: &Input<'_>, marker: &Marker) -> Result<Value, Failure> {
    let extension = input.extension();
    let example = super::literal::class_reference_example(extension, "CatalogTestSetup");
    if marker.arguments.trim_start().starts_with('{') {
        return Err(input.failure(
            marker.span.start,
            format!(
                "Test takes the setup class itself, not a name in an object: write Test({example})"
            ),
        ));
    }
    let (reference, options) = super::literal::reference_and_options(&marker.arguments);
    let Some(configuration) = super::literal::class_reference(extension, reference) else {
        return Err(input.failure(
            marker.span.start,
            format!("Test names one Setup class in this source, for example Test({example})"),
        ));
    };
    let mut object = match options {
        Some(options) => test_options(input, marker, options)?,
        None => Map::new(),
    };
    object.insert(
        "configuration".to_owned(),
        Value::String(configuration.to_owned()),
    );
    if let Some(partition) = object.get("partition")
        && partition
            .as_str()
            .is_none_or(|value| value.trim().is_empty() || value.len() > 256)
    {
        return Err(input.failure(
            marker.span.start,
            "Test.partition is a bounded nonempty string",
        ));
    }
    Ok(Value::Object(object))
}

fn test_options(
    input: &Input<'_>,
    marker: &Marker,
    options: &str,
) -> Result<Map<String, Value>, Failure> {
    let parsed = super::literal::parse(options).ok();
    let Some(Value::Object(object)) = parsed else {
        return Err(input.failure(
            marker.span.start,
            "Test options are one JSON object, for example {\"partition\":\"tests\"}",
        ));
    };
    if object.keys().any(|key| key != "partition") {
        return Err(input.failure(marker.span.start, "Test options accept only partition"));
    }
    Ok(object)
}

pub(super) fn properties(
    input: &Input<'_>,
    marker: &Marker,
    environment: &str,
) -> Result<Value, Failure> {
    let value: Value = super::literal::parse(&marker.arguments).map_err(|_| {
        input.failure(
            marker.span.start,
            "TestSettings takes one literal object of properties",
        )
    })?;
    let entries = match value.as_object() {
        Some(entries) if entries.len() <= TestPropertyLimits::MAX_PROPERTIES => entries,
        _ => {
            return Err(input.failure(
                marker.span.start,
                "TestSettings takes at most 64 literal properties",
            ));
        }
    };
    for (path, item) in entries {
        check_property(input, marker, environment, path, item)?;
    }
    Ok(Value::Object(entries.clone()))
}

fn check_property(
    input: &Input<'_>,
    marker: &Marker,
    environment: &str,
    path: &str,
    item: &Value,
) -> Result<(), Failure> {
    let prefix = format!("{environment}.");
    if path.is_empty() || path.len() > 256 || !path.starts_with(&prefix) {
        return Err(input.failure(
            marker.span.start,
            format!("TestSettings paths are bounded and begin with '{environment}.'"),
        ));
    }
    let is_simple = matches!(item, Value::Null | Value::Bool(_) | Value::String(_));
    let is_finite_number = item.as_f64().is_some_and(f64::is_finite);
    if !is_simple && !is_finite_number {
        return Err(input.failure(
            marker.span.start,
            "TestSettings values are bounded JSON scalars",
        ));
    }
    Ok(())
}

pub(super) fn mocked(
    input: &Input<'_>,
    marker: &Marker,
    name: &str,
    policies: &mut Map<String, Value>,
) -> Result<(), Failure> {
    if !marker.arguments.trim().is_empty() {
        return Err(input.failure(
            marker.span.start,
            crate::annotation_rules::AnnotationRules::no_arguments("Fake"),
        ));
    }
    let mocks = policies
        .entry("mockBeans")
        .or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(names) = mocks else {
        return Err(input.failure(marker.span.start, "Invalid Fake declaration"));
    };
    let entry = Value::String(name.to_owned());
    if names.contains(&entry) {
        return Err(input.failure(marker.span.start, "One Fake per field"));
    }
    if names.len() >= 32 {
        return Err(input.failure(marker.span.start, "A test declares at most 32 MockBeans"));
    }
    names.push(entry);
    Ok(())
}

pub(super) fn joined<T>(items: impl IntoIterator<Item = T>, spell: impl Fn(T) -> String) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(&spell(item));
    }
    text
}

pub(super) struct Unit {
    pub name: String,
    pub configuration: String,
    pub partition: String,
    pub profiles: Value,
    pub properties: Value,
    pub cases: Vec<String>,
    pub inputs: Map<String, Value>,
    pub mocks: Vec<String>,
}

pub(super) fn polyglot_unit(ty: &super::polyglot::Type) -> Option<Unit> {
    let declared = ty.policies.get("test")?.as_object()?;
    let configuration = declared.get("configuration")?.as_str()?.to_owned();
    let partition = declared
        .get("partition")
        .and_then(Value::as_str)
        .unwrap_or("test")
        .to_owned();
    let profiles = ty
        .policies
        .get("activeProfiles")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let properties = ty
        .policies
        .get("testProperties")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    Some(Unit {
        name: ty.name.clone(),
        configuration,
        partition,
        profiles,
        properties,
        cases: string_list(&ty.policies, "cases"),
        inputs: super::ValueSource::inputs(&ty.policies),
        mocks: string_list(&ty.policies, "mockBeans"),
    })
}

pub(super) fn unit_helper(types: &[super::polyglot::Type], helper: &'static str) -> &'static str {
    let tested = types.iter().any(|ty| ty.policies.contains_key("test"));
    helper_when(tested, helper)
}

fn helper_when(tested: bool, helper: &'static str) -> &'static str {
    if tested {
        return helper;
    }
    ""
}

fn string_list(policies: &Map<String, Value>, key: &str) -> Vec<String> {
    let Some(values) = policies.get(key).and_then(Value::as_array) else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub(super) fn controller_marker(language: &str) -> &'static str {
    match language {
        "rs" => "#[RequestRoute()]",
        "go" => "// @RequestRoute()",
        "php" | "rb" => "#[RequestRoute]",
        "cs" => "[RequestRoute]",
        "py" => "@request_route()",
        _ => "@RequestRoute",
    }
}

fn case_table(registration: &super::Registration) -> Value {
    let cases = string_list(&registration.data, "cases");
    let inputs = super::ValueSource::inputs(&registration.data);
    super::ValueSource::case_table(&cases, &inputs)
}

pub(super) fn members(registration: &super::Registration) -> Option<String> {
    let unit = registration.data.get("test")?.as_object()?;
    let configuration = unit.get("configuration")?.as_str()?;
    let partition = unit
        .get("partition")
        .and_then(Value::as_str)
        .unwrap_or("test");
    let profiles = registration
        .data
        .get("activeProfiles")
        .map_or_else(|| String::from("[]"), ToString::to_string);
    let properties = registration
        .data
        .get("testProperties")
        .map_or_else(|| String::from("{}"), ToString::to_string);
    let mocks = mock_bindings(&registration.data);
    Some(format!(
        " async startBehaviors() {{ await this.stopBehaviors();          this.behaviors = await Behaviors.create({configuration}, {{ partition: {},          signal: new AbortController().signal, profiles: {profiles}, properties: {properties} }}); }}          async stopBehaviors() {{ if (this.behaviors) {{ this.behaviors.dispose(); this.behaviors = undefined; }} }}          construct(Type) {{ return Behaviors.constructWith(Type, this.behaviors, {{ {mocks} }}); }}          async POST() {{ const cases = [];          for (const [name, method, input] of {}) {{          await this.startBehaviors(); Behaviors.takeCalled();          try {{ await this[method](...input); cases.push({{ name, passed: true, called: Behaviors.takeCalled() }}); }}          catch (error) {{ cases.push({{ name, passed: false, error: String(error && error.message || error), called: Behaviors.takeCalled() }}); }}          finally {{ await this.stopBehaviors(); }} }}          return {{ unit: {}, cases }}; }} ",
        serde_json::Value::String(partition.to_owned()),
        case_table(registration),
        serde_json::Value::String(registration.name.clone()),
    ))
}

fn mock_bindings(data: &Map<String, Value>) -> String {
    let Some(names) = data.get("mockBeans").and_then(Value::as_array) else {
        return String::new();
    };
    let bindings: Vec<String> = names
        .iter()
        .filter_map(Value::as_str)
        .map(|name| format!("{name}: this.{name}"))
        .collect();
    bindings.join(", ")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::behavior::lower;
    use std::path::Path;

    fn lower_tac_js(source: &str) -> Result<Option<String>, crate::Failure> {
        lower(Path::new("tac.js"), source, "tac")
    }

    fn with_configuration(body: &str) -> String {
        format!(
            "@Setup()\nclass CatalogConfiguration {{\n  static configure() {{ return {{}}; }}\n}}\n{body}"
        )
    }

    #[test]
    fn a_test_unit_records_its_configuration_profiles_properties_and_mocks() {
        let source = with_configuration(
            "@Test(CatalogConfiguration, {\"partition\":\"tests\"})\n@UseProfiles(\"test\")\n@TestSettings({\"tac.catalog.title\":\"Test\"})\n@FreshStart()\nexport default class CatalogServiceTest {\n@Fake()\ncatalogClient;\n}\n",
        );
        let output = lower_tac_js(&source).expect("test unit").expect("lowered");
        for expected in [
            "\"test\":{\"configuration\":\"CatalogConfiguration\",\"partition\":\"tests\"}",
            "\"activeProfiles\":[\"test\"]",
            "\"testProperties\":{\"tac.catalog.title\":\"Test\"}",
            "\"dirtiesContext\":true",
            "\"mockBeans\":[\"catalogClient\"]",
        ] {
            assert!(
                output.contains(expected),
                "{expected} missing from {output}"
            );
        }
    }

    #[test]
    fn invalid_test_declarations_fail_closed() {
        for body in [
            "@Test()\nclass CatalogServiceTest {}\n",
            "@Test(\"CatalogConfiguration\")\nclass CatalogServiceTest {}\n",
            "@Test(CatalogConfiguration, {\"profile\":\"x\"})\nclass CatalogServiceTest {}\n",
            "@Test(CatalogConfiguration, \"tests\")\nclass CatalogServiceTest {}\n",
            "@Test(CatalogConfiguration, {\"partition\":\"\"})\nclass CatalogServiceTest {}\n",
            "@Test(CatalogConfiguration)\n@TestSettings({\"yon.secret\":\"x\"})\nclass CatalogServiceTest {}\n",
            "@Test(CatalogConfiguration)\n@TestSettings({\"tac.a\":{}})\nclass CatalogServiceTest {}\n",
            "@Test(CatalogConfiguration)\nclass CatalogServiceTest {\n@Fake()\nstatic client;\n}\n",
            "@Test(CatalogConfiguration)\nclass CatalogServiceTest {\n@Fake()\nclient = 1;\n}\n",
            "@Test(CatalogConfiguration)\nclass CatalogServiceTest {\n@Fake(\"client\")\nclient;\n}\n",
            "@FreshStart(\"always\")\nclass CatalogServiceTest {}\n",
        ] {
            assert!(
                lower_tac_js(&with_configuration(body)).is_err(),
                "accepted {body}"
            );
        }
    }

    #[test]
    fn the_old_object_form_is_refused_with_the_new_form() {
        let source = with_configuration(
            "@Test({\"configuration\":\"CatalogConfiguration\"})\nclass CatalogServiceTest {}\n",
        );
        let refused = lower_tac_js(&source).expect_err("old form");
        assert!(
            refused.to_string().contains("write Test(CatalogTestSetup)"),
            "{refused}"
        );
        let java = "@Test({\"configuration\":\"C\"})\nclass CatalogServiceTest {}\n";
        let refused = lower(Path::new("tac.java"), java, "tac").expect_err("old form");
        assert!(
            refused.to_string().contains("Test(CatalogTestSetup.class)"),
            "{refused}"
        );
    }

    fn native_test_units() -> [(&'static str, String); 5] {
        let options = "CatalogConfiguration, {\"partition\":\"tests\"}";
        [
            (
                "py",
                format!(
                    "@setup()\nclass CatalogConfiguration:\n    @staticmethod\n    def configure(execution):\n        return {{}}\n\n@Test({options})\n@UseProfiles(\"test\")\nclass CatalogServiceTest:\n    @Fake()\n    catalog_client: object\n"
                ),
            ),
            (
                "java",
                format!(
                    "@Setup\nclass CatalogConfiguration {{\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {{ return new Behaviors.Configuration(java.util.Map.of(), java.util.Map.of(), java.util.Map.of(), null, null); }}\n}}\n@Test({options})\n@UseProfiles(\"test\")\nclass CatalogServiceTest {{\n    @Fake()\n    CatalogClient catalogClient;\n}}\n"
                ),
            ),
            (
                "rs",
                format!(
                    "#[Setup()]\nstruct CatalogConfiguration {{}}\nimpl CatalogConfiguration {{\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {{ Ok(BehaviorConfiguration::default()) }}\n}}\n#[Test({options})]\n#[UseProfiles(\"test\")]\nstruct CatalogServiceTest {{\n    #[Fake()]\n    catalog_client: BehaviorValue,\n}}\n"
                ),
            ),
            (
                "swift",
                format!(
                    "@Setup()\nfinal class CatalogConfiguration {{\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {{ return BehaviorAdapters() }}\n}}\n@Test({options})\n@UseProfiles(\"test\")\nfinal class CatalogServiceTest {{\n    @Fake()\n    let catalogClient: BehaviorJSON\n}}\n"
                ),
            ),
            (
                "go",
                format!(
                    "// @Setup()\ntype CatalogConfiguration struct{{}}\n\nfunc (CatalogConfiguration) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {{ return NewBehaviorConfiguration(), nil }}\n\n// @Test({options})\n// @UseProfiles(\"test\")\ntype CatalogServiceTest struct {{\n\tbehaviors *BehaviorContext\n\t// @Fake()\n\tcatalogClient BehaviorValue\n}}\n\nfunc (test *CatalogServiceTest) TestHasAContext() error {{ return nil }}\n"
                ),
            ),
        ]
    }

    #[test]
    fn a_test_unit_reads_the_same_in_every_language() {
        for (extension, source) in native_test_units() {
            let lowered = lower(Path::new(&format!("tac.{extension}")), &source, "tac")
                .unwrap_or_else(|error| panic!("{extension}: {error:?}"))
                .unwrap_or_else(|| panic!("{extension} produced no output"));
            assert!(lowered.contains("CatalogConfiguration"), "{extension}");
            for marker in ["Test", "UseProfiles", "Fake"] {
                assert!(
                    !lowered.contains(&format!("@{marker}"))
                        && !lowered.contains(&format!("#[{marker}")),
                    "{extension} kept {marker}: {lowered}"
                );
            }
        }
    }

    #[test]
    fn a_go_test_unit_becomes_a_controller_whose_post_reports_its_cases() {
        let source = "// @Setup()\ntype CatalogConfiguration struct{}\n\n// @Test(CatalogConfiguration, {\"partition\":\"tests\"})\ntype CatalogServiceTest struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (test *CatalogServiceTest) TestFirst() error { return nil }\n\nfunc (test *CatalogServiceTest) helper() {}\n";
        let lowered = lower(
            Path::new("server/tests/CatalogServiceTest.go"),
            source,
            "yon",
        )
        .expect("Go unit")
        .expect("lowered");
        assert!(
            lowered.contains("// @RequestRoute()\ntype CatalogServiceTest"),
            "{lowered}"
        );
        assert!(
            lowered.contains("case \"TestFirst\":\n\t\treturn __tachyon_receiver.TestFirst()"),
            "{lowered}"
        );
        assert!(
            lowered.contains("func (__tachyon_receiver *CatalogServiceTest) POST() any {"),
            "{lowered}"
        );
        assert!(!lowered.contains("\"helper\""), "{lowered}");
        let executable = crate::yon::lower_with_routes(
            Path::new("server/tests/CatalogServiceTest.go"),
            source,
            None,
        )
        .expect("test lowering")
        .expect("test unit");
        assert!(executable.routes.is_empty());
        assert!(executable.source.contains("type Handler struct"));
        let wrong = source.replace("TestFirst() error", "TestFirst() (BehaviorValue, error)");
        let failure = lower(
            Path::new("server/tests/CatalogServiceTest.go"),
            &wrong,
            "yon",
        )
        .expect_err("a case returns error only");
        assert!(
            failure.to_string().contains("must return error"),
            "{failure}"
        );
    }

    #[test]
    fn a_javascript_test_unit_keeps_its_protocol_entry_without_publishing_a_route() {
        let source = with_configuration(
            "@Test(CatalogConfiguration)\nexport class CatalogServiceTest {\nasync testReady() { Assert.isTrue(Boolean(this.behaviors)); }\n}\n",
        );
        crate::convention::check_handler(Path::new("server/tests/CatalogServiceTest.js"), &source)
            .expect("test preflight");
        let lowered = crate::yon::lower_with_routes(
            Path::new("server/tests/CatalogServiceTest.js"),
            &source,
            None,
        )
        .expect("test lowering")
        .expect("test unit");
        assert!(lowered.routes.is_empty());
        assert!(lowered.source.contains("class Handler"));
        assert!(lowered.source.contains("static POST"));
    }

    #[test]
    fn an_argument_free_marker_needs_no_parentheses() {
        let bare = "@Setup\nclass CatalogConfiguration {\n  static configure() { return {}; }\n}\n@Converter\nclass ProductMapper {\n  @Needs\n  behaviors;\n  @Convert({\"fields\":[{\"source\":\"id\",\"target\":\"id\"}]})\n  async toSummary(product) {}\n}\n";
        let output = lower_tac_js(bare).expect("bare markers").expect("lowered");
        assert!(output.contains("\"mapper\":true"), "{output}");
        assert!(
            output.contains("\"requiredArgs\":[\"behaviors\"]"),
            "{output}"
        );
        let extra = "@Setup(\"catalog\")\nclass CatalogConfiguration {\n  static configure() { return {}; }\n}\n";
        assert!(lower_tac_js(extra).is_err());
        let missing =
            "@Converter\nclass ProductMapper {\n  @Convert\n  async toSummary(product) {}\n}\n";
        assert!(lower_tac_js(missing).is_err());
    }

    #[test]
    fn slicing_and_spying_name_the_supported_marker() {
        for (marker, expected) in [
            ("WebMvcTest", "write Test"),
            ("DataJpaTest", "write Test"),
            ("SpyBean", "write Fake"),
        ] {
            let source = format!("@{marker}()\nclass CatalogServiceTest {{}}\n");
            let Err(failure) = lower_tac_js(&source) else {
                panic!("{marker} was accepted");
            };
            let message = &failure.diagnostics()[0].message;
            assert!(message.contains(expected), "{marker}: {message}");
        }
    }
}

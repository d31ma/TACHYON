use super::DeclaredContract;
use std::path::Path;

struct ConverterSamples;

impl ConverterSamples {
    const PLAN: &'static str = r#"{"fields":[{"source":"name","target":"title"}]}"#;
    const NO_IMPLEMENTATION: &'static str =
        "A Converter has no implementation: Tachyon generates each mapping.";

    fn path(extension: &str) -> String {
        format!("server/converters/ProductConverter.{extension}")
    }

    fn filled(source: &str) -> String {
        source.replace("PLAN", Self::PLAN)
    }

    fn refusal(extension: &str, source: &str) -> String {
        let source = Self::filled(source);
        DeclaredContract::check(Path::new(&Self::path(extension)), &source)
            .expect_err(&format!("{extension} must be refused: {source}"))
            .to_string()
    }

    fn lowered(extension: &str, source: &str, realm: &str) -> Option<String> {
        let source = Self::filled(source);
        DeclaredContract::concrete(Path::new(&Self::path(extension)), &source, realm)
    }

    fn rewritten(extension: &str, source: &str) -> String {
        let source = Self::filled(source);
        let mut edits = DeclaredContract::rewrite(Path::new(&Self::path(extension)), &source)
            .unwrap_or_else(|problem| panic!("{extension}: {problem}"));
        edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut output = source;
        for (range, text) in edits {
            output.replace_range(range, &text);
        }
        output
    }
}

#[test]
fn a_converter_method_with_a_body_is_refused() {
    let bodies = [
        (
            "java",
            "@Converter\ninterface ProductConverter {\n    @Convert(PLAN)\n    default Object toSummary(Object product) { return product; }\n}\n",
        ),
        (
            "ts",
            "@Converter\nexport default class ProductConverter {\n  @Convert(PLAN)\n  async toSummary(product: BehaviorJsonValue): Promise<BehaviorJsonObject | void> { return {}; }\n}\n",
        ),
    ];
    for (extension, source) in bodies {
        let message = ConverterSamples::refusal(extension, source);
        assert!(message.contains("TY1308"), "{extension}: {message}");
        assert!(
            message.contains(ConverterSamples::NO_IMPLEMENTATION),
            "{extension}: {message}"
        );
    }
}

#[test]
fn a_class_converter_is_refused_where_the_language_has_a_contract_form() {
    let classes = [
        (
            "java",
            "@Converter\nclass ProductConverter {\n    @Convert(PLAN)\n    Object toSummary(Object product) {}\n}\n",
            "an `interface`",
        ),
        (
            "dart",
            "@Converter\nabstract class ProductConverter {\n  @Convert(PLAN)\n  Object? toSummary(Object? product);\n}\n",
            "an `abstract interface class`",
        ),
    ];
    for (extension, source, form) in classes {
        let message = ConverterSamples::refusal(extension, source);
        assert!(message.contains(form), "{extension}: {message}");
    }
}

#[test]
fn a_converter_method_without_convert_is_refused() {
    let source =
        "@Converter\ninterface ProductConverter {\n    fun toSummary(product: Any?): Any?\n}\n";
    let message = ConverterSamples::refusal("kt", source);
    assert!(message.contains("is not a mapping"), "{message}");
}

#[test]
fn a_contract_converter_passes_the_check() {
    let source = "@Converter\ninterface ProductConverter {\n    @Convert(PLAN)\n    Object toSummary(Object product);\n}\n";
    let source = ConverterSamples::filled(source);
    let checked = DeclaredContract::check(Path::new(&ConverterSamples::path("java")), &source);
    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn a_script_converter_gets_no_context_field() {
    let javascript = "@Converter\nexport default class ProductConverter {\n  @Convert(PLAN)\n  async toSummary(product) {}\n}\n";
    let typescript = "@Converter\nexport default class ProductConverter {\n  @Convert(PLAN)\n  async toSummary(product: BehaviorJsonValue): Promise<BehaviorJsonObject | void> {}\n}\n";
    for realm in ["yon", "tac"] {
        assert!(ConverterSamples::lowered("js", javascript, realm).is_none());
        assert!(ConverterSamples::lowered("ts", typescript, realm).is_none());
    }
}

#[test]
fn a_converter_contract_becomes_a_class_the_container_builds_without_a_context() {
    let contracts = [
        (
            "java",
            "@Converter\ninterface ProductConverter {\n    @Convert(PLAN)\n    Object toSummary(Object product);\n}\n",
            "class ProductConverter {",
        ),
        (
            "rs",
            "#[Converter]\ntrait ProductConverter {\n    #[Convert(PLAN)]\n    fn to_summary(&self, product: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}\n",
            "struct ProductConverter;",
        ),
        (
            "go",
            "// @Converter\ntype ProductConverter interface {\n\t// @Convert(PLAN)\n\tToSummary(product BehaviorValue) (BehaviorValue, error)\n}\n",
            "func NewProductConverter() (*ProductConverter, error) {\n\treturn &ProductConverter{}, nil\n}",
        ),
    ];
    for realm in ["yon", "tac"] {
        for (extension, source, concrete) in contracts {
            let lowered = ConverterSamples::lowered(extension, source, realm).expect("lowered");
            assert!(
                lowered.contains(concrete),
                "{extension} {realm}:\n{lowered}"
            );
            assert!(
                !lowered.contains("behaviors"),
                "{extension} {realm}:\n{lowered}"
            );
        }
    }
}

#[test]
fn a_converter_contract_in_fsharp_swift_or_python_is_left_for_the_mapping_lowering() {
    let fsharp = "[<Converter>]\ntype ProductConverter =\n    [<Convert(PLAN)>]\n    abstract toSummary: product: BehaviorJSON -> BehaviorJSON\n";
    let swift = "@Converter\nprotocol ProductConverter {\n    @Convert(PLAN)\n    func toSummary(_ product: BehaviorJSON) throws -> BehaviorJSON\n}\n";
    assert!(ConverterSamples::lowered("fs", fsharp, "yon").is_none());
    assert!(ConverterSamples::lowered("swift", swift, "tac").is_none());
}

#[test]
fn migrate_fix_rewrites_a_class_converter_as_a_contract() {
    let source = "@Converter\nclass ProductConverter {\n    @Needs\n    Behaviors.Context behaviors;\n    @Convert(PLAN)\n    Object toSummary(Object product) {}\n}\n";
    let rewritten = ConverterSamples::rewritten("java", source);
    assert!(
        rewritten.contains("interface ProductConverter {"),
        "{rewritten}"
    );
    assert!(
        rewritten.contains("Object toSummary(Object product);"),
        "{rewritten}"
    );
    assert!(!rewritten.contains("behaviors"), "{rewritten}");
    let checked = DeclaredContract::check(Path::new(&ConverterSamples::path("java")), &rewritten);
    assert!(checked.is_ok(), "{checked:?}\n{rewritten}");
}

#[test]
fn migrate_fix_removes_the_context_field_of_a_script_converter() {
    let source = "@Converter\nexport default class ProductConverter {\n  @Needs\n  behaviors!: BehaviorContext;\n  @Convert(PLAN)\n  async toSummary(product: BehaviorJsonValue): Promise<BehaviorJsonObject | void> {}\n}\n";
    let rewritten = ConverterSamples::rewritten("ts", source);
    assert!(!rewritten.contains("behaviors"), "{rewritten}");
    assert!(rewritten.contains("async toSummary("), "{rewritten}");
    let checked = DeclaredContract::check(Path::new(&ConverterSamples::path("ts")), &rewritten);
    assert!(checked.is_ok(), "{checked:?}\n{rewritten}");
}

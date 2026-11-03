#![allow(clippy::expect_used)]
use super::lower;
use crate::Failure;
use std::path::Path;

fn lower_in(environment: &str, extension: &str, source: &str) -> Result<Option<String>, Failure> {
    let path = format!("{environment}.{extension}");
    lower(Path::new(&path), source, environment)
}

fn lower_tac_output(extension: &str, source: &str) -> String {
    lower_in("tac", extension, source)
        .unwrap_or_else(|error| panic!("{extension}: {error:?}"))
        .unwrap_or_else(|| panic!("{extension} produced no output"))
}

#[test]
fn unannotated_sources_and_marker_strings_are_unchanged() {
    for (extension, source) in [
        ("py", "value = '@CacheSettings()'\n"),
        ("php", "<?php $value = '#[CacheSettings()]';\n"),
        ("rs", "const VALUE: &str = \"#[tac::cache_settings()]\";\n"),
        ("swift", "let value = \"@CacheSettings()\"\n"),
        ("kt", "val value = \"@CacheSettings()\"\n"),
        ("java", "class Plain {}\n"),
        ("cs", "class Plain {}\n"),
    ] {
        let lowered = lower_in("tac", extension, source).expect("plain source");
        assert!(lowered.is_none());
    }
}

#[test]
fn inline_known_markers_are_not_silently_ignored() {
    for (extension, marker) in [
        ("py", "@CacheSettings()"),
        ("php", "#[CacheSettings()]"),
        ("rs", "#[tac::cache_settings()]"),
        ("cs", "[Cache]"),
        ("swift", "@CacheSettings()"),
        ("kt", "@CacheSettings()"),
        ("java", "@CacheSettings()"),
    ] {
        let source = format!("class Example {{ {marker} }}\n");
        assert!(lower_in("tac", extension, &source).is_err(), "{extension}");
    }
}

#[test]
fn python_rejects_invalid_declarations_before_execution() {
    let cases = [
        "@setup()\n@setup()\nclass Config:\n    pass\n",
        "@entity({\"adapter\":\"data\",\"id\":\"id\"})\nclass Product:\n    pass\n",
        "@needs_all('nested')\nclass Service:\n    def method(self):\n        nested: object\n",
        "@needs_all('value')\nclass Service(Parent):\n    value: object\n",
        "class Service:\n    def page(self, @page_defaults({\"defaultSize\":1,\"maxSize\":1,\"sort\":[]}) value = None):\n        return value\n",
        "class Service:\n    def page(self, @page_defaults({\"defaultSize\":1,\"maxSize\":1,\"sort\":[]}) *value):\n        return value\n",
        "class Service:\n    @page_defaults({\"argument\":0})\n    def page(self, value):\n        return value\n",
        "class Service:\n    def outer(self):\n        def inner(@page_defaults({\"defaultSize\":1,\"maxSize\":1,\"sort\":[]}) value):\n            return value\n",
        "@setup()\nclass BehaviorContext:\n    pass\n",
    ];
    for source in cases {
        assert!(lower_in("yon", "py", source).is_err(), "accepted {source}");
    }
}

#[test]
fn native_namespaces_and_source_bounds_are_enforced() {
    let wrong_namespace = "#[yon::setup]\nstruct Config;\n";
    assert!(lower_in("tac", "rs", wrong_namespace).is_err());
    let oversized = format!(
        "@setup()\nclass Config:\n    pass\n#{}",
        "x".repeat(1024 * 1024)
    );
    assert!(lower_in("yon", "py", &oversized).is_err());
}

#[test]
fn python_constructor_preserves_two_space_indentation() {
    let spaces = "@needs_all('behaviors')\nclass Catalog:\n  behaviors: object\n";
    let generated = lower_in("yon", "py", spaces)
        .expect("valid fields")
        .expect("lowered");
    assert!(
        generated.contains("  def __init__(self, behaviors):\n    self.behaviors = behaviors\n")
    );
    let tabs = "@needs_all('behaviors')\nclass Catalog:\n# comment indentation is insignificant\n\tbehaviors: object\n";
    let generated = lower_in("yon", "py", tabs)
        .expect("tab fields")
        .expect("lowered");
    assert!(
        generated.contains("\tdef __init__(self, behaviors):\n\t\tself.behaviors = behaviors\n")
    );
}

#[test]
fn generated_binding_names_cannot_collide_with_authored_parameters() {
    let php = "<?php\nclass Catalog {\npublic function page(#[PageDefaults({\"defaultSize\":1,\"maxSize\":1,\"sort\":[]})] array $__behavior_args): array { return $__behavior_args; }\n}";
    let rust = "struct Catalog { behaviors: BehaviorContext }\nimpl Catalog {\nfn page(&self, #[PageDefaults({\"defaultSize\":1,\"maxSize\":1,\"sort\":[]})] __tachyon_arguments: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(__tachyon_arguments) }\n}";
    let go = "type Catalog struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (catalog *Catalog) Page(\n\t// @PageDefaults({\"defaultSize\":1,\"maxSize\":1,\"sort\":[]})\n\t__tachyon_arguments BehaviorValue,\n) (BehaviorValue, error) { return __tachyon_arguments, nil }\n";
    for (extension, source) in [("php", php), ("rs", rust), ("go", go)] {
        assert!(lower_in("yon", extension, source).is_err());
    }
}

fn abstract_repository_cases() -> [(&'static str, String, &'static str); 8] {
    let repository = "Product, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100}";
    [
        (
            "java",
            format!(
                "@Entity({{\"adapter\":\"data\"}})\n@Table(\"products\")\nclass Product {{\n  @Id\n  int id;\n}}\n@Store({repository})\ninterface ProductRepository {{\n  Object findByStatus(Object status);\n}}\n"
            ),
            "class ProductRepository",
        ),
        (
            "cs",
            format!(
                "[Entity({{\"adapter\":\"data\"}})]\n[Table(\"products\")]\nclass Product {{\n  [Id]\n  public int id;\n}}\n[Store({repository})]\npublic interface ProductRepository {{\n  BehaviorJSON FindByStatus(BehaviorJSON status);\n}}\n"
            ),
            "class ProductRepository",
        ),
        (
            "kt",
            format!(
                "@Entity({{\"adapter\":\"data\"}})\n@Table(\"products\")\nclass Product {{\n  @Id\n  var id: Int = 0\n}}\n@Store({repository})\ninterface ProductRepository {{\n  fun findByStatus(status: Any?): Any?\n}}\n"
            ),
            "class ProductRepository",
        ),
        (
            "php",
            format!(
                "<?php\n#[Entity({{\"adapter\":\"data\"}})]\n#[Table(\"products\")]\nclass Product {{\n  #[Id]\n  public int $id = 0;\n}}\n#[Store({repository})]\ninterface ProductRepository {{\n  public function findByStatus(mixed $status): mixed;\n}}\n"
            ),
            "class ProductRepository",
        ),
        (
            "swift",
            format!(
                "@Entity({{\"adapter\":\"data\"}})\n@Table(\"products\")\nfinal class Product {{\n  @Id\n  var id: Int = 0\n}}\n@Store({repository})\nprotocol ProductRepository {{\n  func findByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON\n}}\n"
            ),
            "final class ProductRepository",
        ),
        (
            "rs",
            format!(
                "#[Entity({{\"adapter\":\"data\"}})]\n#[Table(\"products\")]\nstruct Product {{\n  #[Id]\n  id: i64,\n}}\n#[Store({repository})]\ntrait ProductRepository {{\n  fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}}\n"
            ),
            "struct ProductRepository { behaviors: BehaviorContext }\nimpl ProductRepository",
        ),
        (
            "py",
            format!(
                "@entity({{\"adapter\":\"data\"}})\n@table(\"products\")\nclass Product:\n    @id()\n    id: int = 0\n@store({repository})\nclass ProductRepository(Protocol):\n    async def find_by_status(self, status): ...\n"
            ),
            "class ProductRepository:",
        ),
        (
            "go",
            format!(
                "// @Entity({{\"adapter\":\"data\"}})\n// @Table(\"products\")\ntype Product struct {{\n\t// @Id\n\tid int64\n}}\n// @Store({repository})\ntype ProductRepository interface {{\n  FindByStatus(status BehaviorValue) (BehaviorValue, error)\n}}\n"
            ),
            "type ProductRepository struct { behaviors *BehaviorContext }",
        ),
    ]
}

fn keeps_abstract_declaration(lowered: &str) -> bool {
    lowered.contains("ProductRepository(Protocol)")
        || lowered.contains("interface ProductRepository")
        || lowered.contains("protocol ProductRepository")
        || lowered.contains("trait ProductRepository")
}

#[test]
fn an_abstract_declaration_lowers_into_its_concrete_type() {
    for (extension, source, concrete) in abstract_repository_cases() {
        let lowered = lower_tac_output(extension, &source);
        assert!(lowered.contains(concrete), "{extension}: {lowered}");
        assert!(
            lowered.contains("behaviors"),
            "{extension} never received a context: {lowered}"
        );
        assert!(
            !keeps_abstract_declaration(&lowered),
            "{extension} kept its abstract declaration: {lowered}"
        );
    }
}

#[test]
fn a_concrete_type_still_refuses_a_method_without_a_body() {
    let source = "@Store(Product.class, {\"fields\":[\"id\"],\"defaultSize\":20,\"maxSize\":100})\nclass ProductRepository {\n  Object findById(Object id);\n}\n";
    assert!(lower_in("tac", "java", source).is_err());
}

#[test]
fn an_abstract_declaration_refuses_an_authored_body() {
    let source = "@Store(Product.class, {\"fields\":[\"id\"],\"defaultSize\":20,\"maxSize\":100})\ninterface ProductRepository {\n  default Object findById(Object id) { return null; }\n}\n";
    assert!(lower_in("tac", "java", source).is_err());
}

fn mapper_cases() -> [(&'static str, String); 8] {
    let plan = "{\"fields\":[{\"source\":\"detail.id\",\"target\":\"productId\"},{\"target\":\"kind\",\"constant\":\"product\"}]}";
    [
        (
            "java",
            format!(
                "@Converter()\ninterface ProductMapper {{\n  @Convert({plan})\n  Object toSummary(Object product);\n}}\n"
            ),
        ),
        (
            "cs",
            format!(
                "[Converter()]\npublic interface ProductMapper {{\n  [Convert({plan})]\n  BehaviorJSON ToSummary(BehaviorJSON product);\n}}\n"
            ),
        ),
        (
            "kt",
            format!(
                "@Converter()\ninterface ProductMapper {{\n  @Convert({plan})\n  fun toSummary(product: Any?): Any?\n}}\n"
            ),
        ),
        (
            "php",
            format!(
                "<?php\n#[Converter()]\ninterface ProductMapper {{\n  #[Convert({plan})]\n  public function toSummary(mixed $product): mixed;\n}}\n"
            ),
        ),
        (
            "swift",
            format!(
                "@Converter()\nprotocol ProductMapper {{\n  @Convert({plan})\n  func toSummary(_ product: BehaviorJSON) async throws -> BehaviorJSON\n}}\n"
            ),
        ),
        (
            "rs",
            format!(
                "#[Converter()]\ntrait ProductMapper {{\n  #[Convert({plan})]\n  fn to_summary(&self, product: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}}\n"
            ),
        ),
        (
            "py",
            format!(
                "@converter()\nclass ProductMapper(Protocol):\n    @Convert({plan})\n    async def to_summary(self, product): ...\n"
            ),
        ),
        (
            "go",
            format!(
                "// @Converter()\ntype ProductMapper interface {{\n  // @Convert({plan})\n  ToSummary(product BehaviorValue) (BehaviorValue, error)\n}}\n"
            ),
        ),
    ]
}

fn expected_mapper_call(extension: &str) -> &'static str {
    match extension {
        "java" | "kt" => "mapValue(product",
        "cs" => "MapValue(BehaviorJSON.Parse",
        "swift" => "mapValue(BehaviorJSON.parse",
        "rs" => "product.map_value(",
        "go" => "BehaviorMapValue(product,",
        _ => "mapping",
    }
}

#[test]
fn a_mapper_declares_its_field_plan_in_every_language() {
    for (extension, source) in mapper_cases() {
        let lowered = lower_tac_output(extension, &source);
        let generated = expected_mapper_call(extension);
        assert!(lowered.contains(generated), "{extension}: {lowered}");
    }
}

#[test]
fn a_namespaced_rust_setup_is_a_setup() {
    for marker in ["#[Setup]", "#[tac::setup]"] {
        let source = format!(
            "{marker}\nstruct OrdersSetup {{}}\nimpl OrdersSetup {{\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {{\n        Ok(BehaviorConfiguration::default())\n    }}\n}}\n"
        );
        let lowered = lower_tac_output("rs", &source);
        assert!(
            lowered.contains("impl BehaviorConfigurationSource for OrdersSetup"),
            "{marker}"
        );
    }
}

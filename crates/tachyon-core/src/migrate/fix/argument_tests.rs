use super::MigrationFix;
use std::fs;
use std::path::Path;

struct Samples;

impl Samples {
    const JAVASCRIPT_SERVICE: &str = "@CacheSettings({\"name\":\"items\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8})\nexport default class ItemsService {\n  @Cached({\"keyArgs\":[0]})\n  async find(id, view) { return id; }\n  @ClearCache({\"method\":\"find\",\"arguments\":[1]})\n  async remove(value, id) { return value; }\n  @Check({\"argument\":0,\"type\":\"ItemInput\"})\n  async accept(input) { return input; }\n  @PageDefaults({\"argument\":0,\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]})\n  async list(page) { return page; }\n}\n";
    const JAVASCRIPT_EXPECTED: &str = "@CacheSettings({\"name\":\"items\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8})\nexport default class ItemsService {\n  @Cached\n  async find(@CacheKey id, view) { return id; }\n  @ClearCache({\"method\":\"find\"})\n  async remove(value, id) { return value; }\n  /** @param {ItemInput} input */\n  async accept(@Check input) { return input; }\n  async list(@PageDefaults({\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"]}) page) { return page; }\n}\n";
    const GO_SERVICE: &str = "// @CacheSettings({\"name\":\"items\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8})\ntype ItemsService struct {\n\tbehaviors *BehaviorContext\n}\n\n// @Check({\"argument\":0,\"type\":\"ItemInput\"})\nfunc (service *ItemsService) Accept(input BehaviorValue) (BehaviorValue, error) {\n\treturn input, nil\n}\n";
    const GO_EXPECTED: &str = "// @CacheSettings({\"name\":\"items\",\"ttlMs\":1000,\"maxBytes\":1024,\"maxEntries\":8})\ntype ItemsService struct {\n\tbehaviors *BehaviorContext\n}\n\nfunc (service *ItemsService) Accept(\n\t// @Check(ItemInput)\n\tinput BehaviorValue,\n) (BehaviorValue, error) {\n\treturn input, nil\n}\n";
    const JAVA_STORE: &str = "@Store({\"entity\":\"ProductEntity\",\"fields\":[\"id\"],\"defaultSize\":20,\"maxSize\":100})\ninterface ProductsStore {\n    @Query({\"operation\":\"find\",\"where\":[],\"pageArgument\":0,\"orderBy\":[]})\n    Object listed(Object page);\n}\n";
    const JAVA_STORE_EXPECTED: &str = "@Store(ProductEntity.class, {\"defaultSize\":20,\"fields\":[\"id\"],\"maxSize\":100})\ninterface ProductsStore {\n    @Query({\"operation\":\"find\",\"orderBy\":[],\"where\":[]})\n    Object listed(@PageDefaults Object page);\n}\n";
    const JAVA_ENTITY: &str = "@Entity({\"adapter\":\"data\",\"id\":\"code\"})\n@Table(\"products\")\nclass ProductEntity {\n    String code;\n}\n";
    const JAVA_ENTITY_EXPECTED: &str = "@Entity({\"adapter\":\"data\"})\n@Table(\"products\")\nclass ProductEntity {\n    @Id\n    String code;\n}\n";
    const CSHARP_TEST: &str = "[Test({\"configuration\":\"CatalogTestSetup\",\"partition\":\"tests\"})]\npublic sealed class CatalogServiceTest {\n}\n";
    const CSHARP_TEST_EXPECTED: &str = "[Test(typeof(CatalogTestSetup), {\"partition\":\"tests\"})]\npublic sealed class CatalogServiceTest {\n}\n";
    const PYTHON_SETTINGS: &str = "@settings({\"prefix\":\"YON_CATALOG_\",\"fields\":{\"size\":{\"name\":\"SIZE\",\"type\":\"integer\",\"default\":20,\"min\":1,\"max\":100},\"enabled\":{\"name\":\"ENABLED\",\"type\":\"boolean\",\"default\":false}}})\nclass CatalogSettings:\n    pass\n";
    const PYTHON_SETTINGS_EXPECTED: &str = "from typing import Annotated\n@settings({\"prefix\":\"YON_CATALOG_\"})\nclass CatalogSettings:\n    enabled: bool = False\n    size: Annotated[int, min(1), max(100)] = 20\n";
}

fn converted(relative: &str, source: &str) -> String {
    let project = tempfile::tempdir().expect("project");
    let path = project.path().join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(&path, source).expect("source");
    let planned = MigrationFix::plan(project.path()).expect("plan");
    planned.write(project.path()).expect("write");
    let written = fs::read_to_string(&path).expect("converted source");
    let again = MigrationFix::plan(project.path()).expect("second plan");
    assert_eq!(
        again.change_count(),
        0,
        "a second run changes {relative} again"
    );
    written
}

#[test]
fn indexes_become_parameter_markers_and_names() {
    let written = converted(
        "server/services/ItemsService.js",
        Samples::JAVASCRIPT_SERVICE,
    );
    assert_eq!(written, Samples::JAVASCRIPT_EXPECTED);
    let written = converted("server/services/ItemsService.go", Samples::GO_SERVICE);
    assert_eq!(written, Samples::GO_EXPECTED);
}

#[test]
fn names_in_strings_become_class_references() {
    let written = converted("server/stores/ProductsStore.java", Samples::JAVA_STORE);
    assert_eq!(written, Samples::JAVA_STORE_EXPECTED);
    let written = converted("server/entities/ProductEntity.java", Samples::JAVA_ENTITY);
    assert_eq!(written, Samples::JAVA_ENTITY_EXPECTED);
    let written = converted("client/tests/CatalogServiceTest.cs", Samples::CSHARP_TEST);
    assert_eq!(written, Samples::CSHARP_TEST_EXPECTED);
}

#[test]
fn settings_descriptors_become_declared_fields() {
    let written = converted(
        "server/helpers/CatalogSettings.py",
        Samples::PYTHON_SETTINGS,
    );
    assert_eq!(written, Samples::PYTHON_SETTINGS_EXPECTED);
}

#[test]
fn a_mutation_with_other_parameter_names_is_a_manual_step() {
    let project = tempfile::tempdir().expect("project");
    let source =
        Samples::JAVASCRIPT_SERVICE.replace("async remove(value, id)", "async remove(value, key)");
    let path = Path::new("server/services/ItemsService.js");
    fs::create_dir_all(project.path().join("server/services")).expect("folders");
    fs::write(project.path().join(path), source).expect("source");
    let planned = MigrationFix::plan(project.path()).expect("plan");
    assert!(planned.manual_step_count() >= 1);
}

#[test]
fn python_field_and_parameter_markers_move_into_annotated() {
    let source = "@checked\nclass OrderInput:\n    @required\n    @length({\"min\":1,\"max\":80})\n    title: str = ''  # shown\n    @min(1)\n    count = 1\n\n    def find(self, @cache_key key: str, view):\n        return key\n";
    let written = converted("server/helpers/OrderInput.py", source);
    assert_eq!(
        written,
        "from typing import Annotated\n@checked\nclass OrderInput:\n    title: Annotated[str, required, length({\"min\":1,\"max\":80})] = ''  # shown\n    count: Annotated[object, min(1)] = 1\n\n    def find(self, key: Annotated[str, cache_key], view):\n        return key\n"
    );
    let entity = "@entity({\"adapter\":\"data\",\"id\":\"code\"})\n@table(\"products\")\nclass ProductEntity:\n    code: str = ''\n";
    let written = converted("server/entities/ProductEntity.py", entity);
    assert_eq!(
        written,
        "from typing import Annotated\n@entity({\"adapter\":\"data\"})\n@table(\"products\")\nclass ProductEntity:\n    code: Annotated[str, id] = ''\n"
    );
}

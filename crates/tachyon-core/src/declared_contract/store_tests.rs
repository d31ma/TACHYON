use super::DeclaredContract;
use std::path::Path;

struct StoreSamples;

impl StoreSamples {
    const OPTIONS: &'static str = r#"{"fields":["id","status"],"defaultSize":20,"maxSize":100}"#;
    const QUERY: &'static str = r#"{"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]}"#;

    const CONTRACTS: [(&'static str, &'static str); 13] = [
        (
            "java",
            "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore {\n    Object findByStatus(String status);\n    @Query(QUERY)\n    Object excluding(String status, @PageDefaults java.util.Map<String, Object> page);\n}\n",
        ),
        (
            "groovy",
            "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore {\n    Object findByStatus(String status)\n}\n",
        ),
        (
            "kt",
            "@Store(ProductEntity::class, OPTIONS)\ninterface ProductStore {\n    fun findByStatus(status: String): Any?\n    @Query(QUERY)\n    fun excluding(status: String, @PageDefaults page: Map<String, Any?>): Any?\n}\n",
        ),
        (
            "cs",
            "[Store(typeof(ProductEntity), OPTIONS)]\ninterface ProductStore {\n    System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status);\n}\n",
        ),
        (
            "php",
            "<?php\n#[Store(ProductEntity::class, OPTIONS)]\ninterface ProductStore {\n    public function findByStatus(string $status): mixed;\n}\n",
        ),
        (
            "go",
            "// @Store(ProductEntity, OPTIONS)\ntype ProductStore interface {\n\tFindByStatus(status BehaviorValue) (BehaviorValue, error)\n\t// @Query(QUERY)\n\tExcluding(\n\t\tstatus BehaviorValue,\n\t\t// @PageDefaults\n\t\tpage BehaviorValue,\n\t) (BehaviorValue, error)\n}\n",
        ),
        (
            "dart",
            "@Store(ProductEntity, OPTIONS)\nabstract interface class ProductStore {\n  Future<Object?> findByStatus(String status);\n}\n",
        ),
        (
            "rs",
            "#[Store(ProductEntity, OPTIONS)]\ntrait ProductStore {\n    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}\n",
        ),
        (
            "swift",
            "@Store(ProductEntity, OPTIONS)\nprotocol ProductStore {\n    func findByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON\n}\n",
        ),
        (
            "py",
            "from typing import Protocol\n\n@store(ProductEntity, OPTIONS)\nclass ProductStore(Protocol):\n    async def find_by_status(self, status: str) -> None: ...\n",
        ),
        (
            "js",
            "@Store(ProductEntity, OPTIONS)\nexport default class ProductStore {\n  /** @param {string} status */\n  async findByStatus(status) {}\n}\n",
        ),
        (
            "ts",
            "@Store(ProductEntity, OPTIONS)\nexport default class ProductStore {\n  async findByStatus(status: string): Promise<ProductEntity[] | void> {}\n}\n",
        ),
        (
            "rb",
            "#[Store(ProductEntity, OPTIONS)]\nclass ProductStore\n  def find_by_status(status)\n  end\nend\n",
        ),
    ];

    const LOWERED: [(&'static str, [&'static str; 3]); 12] = [
        (
            "java",
            [
                "@Store\n@Store(ProductEntity.class",
                "Object findByStatus(String status) {}",
                "Behaviors.create(\"ProductSetup\", new Behaviors.Options(\"product\"",
            ],
        ),
        (
            "groovy",
            ["@Store\n@Store(", "class ProductStore", "@OnStart"],
        ),
        (
            "kt",
            [
                "@Store\n@Store(",
                "fun findByStatus(status: String): Any? {}",
                "lateinit var behaviors",
            ],
        ),
        (
            "cs",
            [
                "[Store]\n[Store(",
                "public async System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status) {}",
                "typeof(ProductSetup)",
            ],
        ),
        (
            "php",
            [
                "#[Store]\n#[Store(",
                "public function findByStatus(string $status): mixed {}",
                "ProductSetup::class",
            ],
        ),
        (
            "go",
            [
                "// @Store\n// @Store(",
                "func (store *ProductStore) FindByStatus(",
                "store.behaviors = context",
            ],
        ),
        (
            "dart",
            [
                "@Store\n@Store(",
                "Future<Object?> findByStatus(String status) async {}",
                "late BehaviorContext behaviors;",
            ],
        ),
        (
            "rs",
            [
                "#[Store]\n#[Store(",
                "struct ProductStore { behaviors: BehaviorContext }\nimpl ProductStore {",
                "Behaviors::create::<ProductSetup>",
            ],
        ),
        (
            "py",
            [
                "@store\n@store(",
                "class ProductStore:",
                "async def start_context(self):",
            ],
        ),
        (
            "js",
            [
                "@Store\n@Store(",
                "async findByStatus(status) {}",
                "@OnStart",
            ],
        ),
        (
            "ts",
            [
                "@Store\n@Store(",
                "behaviors!: BehaviorContext;",
                "partition: 'product'",
            ],
        ),
        (
            "rb",
            [
                "#[Store]\n#[Store(",
                "def start_context",
                "Behaviors.create(ProductSetup, partition: \"product\"",
            ],
        ),
    ];

    fn filled(source: &str) -> String {
        source
            .replace("OPTIONS", Self::OPTIONS)
            .replace("QUERY", Self::QUERY)
    }

    fn contract(extension: &str) -> String {
        Self::CONTRACTS
            .iter()
            .find(|(language, _)| *language == extension)
            .map(|(_, source)| Self::filled(source))
            .expect("sample")
    }

    fn path(extension: &str) -> String {
        format!("server/stores/ProductStore.{extension}")
    }

    fn refusal(extension: &str, source: &str) -> String {
        let source = Self::filled(source);
        DeclaredContract::check(Path::new(&Self::path(extension)), &source)
            .expect_err(&format!("{extension} must be refused: {source}"))
            .to_string()
    }

    fn concrete(extension: &str, realm: &str) -> String {
        let source = Self::contract(extension);
        DeclaredContract::concrete(Path::new(&Self::path(extension)), &source, realm)
            .unwrap_or_else(|| panic!("{extension} was not lowered"))
    }
}

#[test]
fn every_language_accepts_its_store_contract() {
    for (extension, _) in StoreSamples::CONTRACTS {
        let source = StoreSamples::contract(extension);
        let checked = DeclaredContract::check(Path::new(&StoreSamples::path(extension)), &source);
        assert!(checked.is_ok(), "{extension}: {checked:?}");
    }
}

#[test]
fn a_class_store_is_refused_where_the_language_has_a_contract_form() {
    let source = "@Store(ProductEntity.class, OPTIONS)\nclass ProductStore {\n    @Needs\n    final Behaviors.Context behaviors;\n    Object findByStatus(String status) {}\n}\n";
    let message = StoreSamples::refusal("java", source);
    assert!(message.contains("TY1308"), "{message}");
    assert!(
        message.contains("declare it as an `interface`"),
        "{message}"
    );
    assert!(
        message.contains("A Store has no implementation"),
        "{message}"
    );
}

#[test]
fn a_store_method_with_a_body_is_refused() {
    let bodies = [
        (
            "java",
            "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore {\n    default Object findByStatus(String status) { return null; }\n}\n",
        ),
        (
            "ts",
            "@Store(ProductEntity, OPTIONS)\nexport default class ProductStore {\n  bounded(request: BehaviorPageInput): BehaviorPageInput { return request; }\n}\n",
        ),
        (
            "py",
            "from typing import Protocol\n\n@store(ProductEntity, OPTIONS)\nclass ProductStore(Protocol):\n    async def find_by_status(self, status: str) -> None:\n        return None\n",
        ),
    ];
    for (extension, source) in bodies {
        let message = StoreSamples::refusal(extension, source);
        assert!(message.contains("has a body"), "{extension}: {message}");
    }
}

#[test]
fn a_store_method_that_is_not_a_query_is_refused() {
    let source =
        "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore {\n    Object all();\n}\n";
    let message = StoreSamples::refusal("java", source);
    assert!(
        message.contains("ProductStore.all is not a query"),
        "{message}"
    );
    assert!(message.contains("@Query"), "{message}");
}

#[test]
fn a_store_that_keeps_state_is_refused() {
    let source = "@Store(ProductEntity, OPTIONS)\nexport default class ProductStore {\n  @Needs\n  behaviors!: BehaviorContext;\n  async findByStatus(status: string): Promise<ProductEntity[] | void> {}\n}\n";
    let message = StoreSamples::refusal("ts", source);
    assert!(
        message.contains("a store keeps no other state"),
        "{message}"
    );
}

#[test]
fn a_store_without_an_entity_is_refused() {
    let sources = [
        (
            "java",
            "@Store\nclass BillingStore {\n  int subtotal() { return 40; }\n}\n",
        ),
        (
            "js",
            "@Store\nexport default class BillingStore {\n  subtotal() { return 40; }\n}\n",
        ),
        ("go", "// @Store\ntype BillingStore struct{}\n"),
    ];
    for (extension, source) in sources {
        let message = StoreSamples::refusal(extension, source);
        assert!(
            message.contains("names no Entity"),
            "{extension}: {message}"
        );
    }
}

#[test]
fn a_store_with_an_all_or_nothing_method_is_refused_and_the_service_owns_the_transaction() {
    let source = "@Store(EntryEntity.class, OPTIONS)\nclass EntryStore {\n    @Needs\n    Behaviors.Context behaviors;\n    Object findByStatus(String status) {}\n\n    @AllOrNothing({\"adapter\":\"ledger\"})\n    Object settle(String status) {\n        return findByStatus(status);\n    }\n}\n";
    let message = StoreSamples::refusal("java", source);
    assert!(
        message.contains("declare it as an `interface`"),
        "{message}"
    );
    assert!(!message.contains("One exception"), "{message}");
    let path = Path::new("server/stores/EntryStore.java");
    let problem = DeclaredContract::rewrite(path, &StoreSamples::filled(source))
        .expect_err("the transaction moves into a service by hand");
    assert!(
        problem.contains("A Store has no implementation"),
        "{problem}"
    );
}

#[test]
fn every_store_contract_lowers_into_an_injected_store_that_owns_its_context() {
    for (extension, parts) in StoreSamples::LOWERED {
        let lowered = StoreSamples::concrete(extension, "yon");
        for part in parts {
            assert!(
                lowered.contains(part),
                "{extension} lacks {part:?}:\n{lowered}"
            );
        }
        for contract in ["interface ProductStore", "trait ProductStore", "(Protocol)"] {
            assert!(
                !lowered.contains(contract),
                "{extension} kept {contract:?}:\n{lowered}"
            );
        }
    }
}

#[test]
fn a_named_setup_builds_the_store_context() {
    let source = StoreSamples::contract("java").replace(
        "\"maxSize\":100}",
        "\"maxSize\":100,\"setup\":\"CatalogSetup\"}",
    );
    let lowered =
        DeclaredContract::concrete(Path::new(&StoreSamples::path("java")), &source, "yon")
            .expect("lowered");
    assert!(
        lowered.contains("Behaviors.create(\"CatalogSetup\""),
        "{lowered}"
    );
}

#[test]
fn native_tac_stores_open_their_own_context() {
    let expected = [
        (
            "dart",
            "late final BehaviorContext behaviors = Behaviors.create(ProductSetup",
        ),
        ("kt", "init {"),
        ("go", "func NewProductStore() (*ProductStore, error) {"),
        ("rs", "impl Default for ProductStore {"),
        ("rb", "def initialize\n      @behaviors = Behaviors.create(ProductSetup"),
        (
            "java",
            "ProductStore() {\n        behaviors = Behaviors.create(\"ProductSetup\"",
        ),
    ];
    for (extension, part) in expected {
        let lowered = StoreSamples::concrete(extension, "tac");
        assert!(
            lowered.contains(part),
            "{extension} lacks {part:?}:\n{lowered}"
        );
        assert!(!lowered.contains("OnStart"), "{extension}:\n{lowered}");
        assert!(
            !lowered.contains("Store\n"),
            "{extension} is not a Tac provider:\n{lowered}"
        );
    }
}

struct StoreMigrations;

impl StoreMigrations {
    const CLASS_STORES: [(&'static str, &'static str); 6] = [
        (
            "java",
            "@Store(ProductEntity.class, OPTIONS)\nclass ProductStore {\n    @Needs\n    final Behaviors.Context behaviors;\n    Object findByStatus(String status) {}\n    @Query(QUERY)\n    Object excluding(String status, @PageDefaults java.util.Map<String, Object> page) {}\n}\n",
        ),
        (
            "kt",
            "@Store(ProductEntity::class, OPTIONS)\nclass ProductStore {\n    @Needs\n    val behaviors: Behaviors.Context\n    fun findByStatus(status: String): Any? {}\n}\n",
        ),
        (
            "cs",
            "[Store(typeof(ProductEntity), OPTIONS)]\npublic sealed class ProductStore {\n    [Needs]\n    public BehaviorContext behaviors;\n    public async System.Threading.Tasks.Task<BehaviorJSON> findByStatus(BehaviorJSON status) {}\n}\n",
        ),
        (
            "rs",
            "#[Store(ProductEntity, OPTIONS)]\nstruct ProductStore {\n    #[Needs]\n    behaviors: BehaviorContext,\n}\nimpl ProductStore {\n    fn find_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}\n}\n",
        ),
        (
            "go",
            "// @Store(ProductEntity, OPTIONS)\ntype ProductStore struct {\n\t// @Needs\n\tbehaviors *BehaviorContext\n}\n\nfunc (repository *ProductStore) FindByStatus(status BehaviorValue) (BehaviorValue, error) {}\n",
        ),
        (
            "py",
            "from typing import Annotated\n\n@store(ProductEntity, OPTIONS)\nclass ProductStore:\n    behaviors: Annotated[object, needs]\n    async def find_by_status(self, status: str) -> None:\n        pass\n",
        ),
    ];

    fn rewritten(extension: &str, source: &str) -> String {
        let path = StoreSamples::path(extension);
        let mut edits = DeclaredContract::rewrite(Path::new(&path), source)
            .unwrap_or_else(|problem| panic!("{extension}: {problem}"));
        assert!(!edits.is_empty(), "{extension} was not rewritten");
        edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
        let mut output = source.to_owned();
        for (range, text) in edits {
            output.replace_range(range, &text);
        }
        output
    }
}

#[test]
fn the_fix_turns_every_class_store_into_a_contract_that_passes_the_check() {
    for (extension, source) in StoreMigrations::CLASS_STORES {
        let source = StoreSamples::filled(source);
        let rewritten = StoreMigrations::rewritten(extension, &source);
        let path = StoreSamples::path(extension);
        let checked = DeclaredContract::check(Path::new(&path), &rewritten);
        assert!(checked.is_ok(), "{extension}: {checked:?}\n{rewritten}");
        assert!(
            !rewritten.contains("behaviors"),
            "{extension}:\n{rewritten}"
        );
    }
}

#[test]
fn the_fix_leaves_store_logic_to_the_author() {
    let bare = "@Store\nclass BillingStore {\n  int subtotal() { return 40; }\n}\n";
    let problem = DeclaredContract::rewrite(Path::new("server/stores/BillingStore.java"), bare)
        .expect_err("a store without an entity stays manual");
    assert!(problem.contains("names no Entity"), "{problem}");
    let written = StoreSamples::filled(
        "@Store(ProductEntity.class, OPTIONS)\nclass ProductStore {\n    Object bounded(java.util.Map<String, Object> request) { return request; }\n}\n",
    );
    let problem = DeclaredContract::rewrite(Path::new(&StoreSamples::path("java")), &written)
        .expect_err("logic stays manual");
    assert!(
        problem.contains("A Store has no implementation"),
        "{problem}"
    );
}

#[test]
fn an_fsharp_store_is_an_interface_that_lowers_into_an_injected_class() {
    let contract = StoreSamples::filled(
        "open System.Threading.Tasks\n\n[<Store(typeof<ProductEntity>, OPTIONS)>]\ntype ProductStore =\n    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>\n",
    );
    let path = Path::new("server/stores/ProductStore.fs");
    let accepted = crate::annotation_layout::validate(path, &contract);
    assert!(accepted.is_ok(), "{accepted:?}");
    let lowered = DeclaredContract::concrete(path, &contract, "yon").expect("lowered");
    assert!(
        lowered.contains("[<Store>]\n[<Store(typeof<ProductEntity>"),
        "{lowered}"
    );
    assert!(
        lowered.contains("type ProductStore() =\n    member val behaviors : BehaviorContext = Behaviors.create(typeof<ProductSetup>, \"product\", new BehaviorSignal())"),
        "{lowered}"
    );
    let class_form = StoreSamples::filled(
        "open System.Threading.Tasks\n\n[<Store(typeof<ProductEntity>, OPTIONS)>]\ntype ProductStore(context: BehaviorContext) =\n    member val behaviors : BehaviorContext = context\n\n    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>\n",
    );
    let refused = crate::annotation_layout::validate(path, &class_form)
        .expect_err("an F# class store is refused")
        .to_string();
    assert!(refused.contains("TY1308"), "{refused}");
}

#[test]
fn the_fix_turns_an_fsharp_class_store_into_a_contract() {
    let path = Path::new("server/stores/ProductStore.fs");
    let class_form = StoreSamples::filled(
        "open System.Threading.Tasks\n\n[<Store(typeof<ProductEntity>, OPTIONS)>]\ntype ProductStore(context: BehaviorContext) =\n    member val behaviors : BehaviorContext = context\n\n    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>\n",
    );
    let mut edits = DeclaredContract::rewrite(path, &class_form).expect("rewritten");
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut rewritten = class_form.clone();
    for (range, text) in edits {
        rewritten.replace_range(range, &text);
    }
    assert!(rewritten.contains("type ProductStore =\n"), "{rewritten}");
    let checked = crate::annotation_layout::validate(path, &rewritten);
    assert!(checked.is_ok(), "{checked:?}\n{rewritten}");
    let written = class_form.replace(
        "    abstract findByStatus",
        "    member this.bounded(request: BehaviorJSON) : BehaviorJSON = request\n\n    abstract findByStatus",
    );
    let problem = DeclaredContract::rewrite(path, &written).expect_err("logic stays manual");
    assert!(
        problem.contains("A Store has no implementation"),
        "{problem}"
    );
}

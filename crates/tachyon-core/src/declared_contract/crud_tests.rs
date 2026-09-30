use super::DeclaredContract;
use std::path::Path;

struct CrudSamples;

impl CrudSamples {
    const OPTIONS: &'static str = r#"{"fields":["id","status"],"defaultSize":20,"maxSize":100}"#;

    const CONTRACTS: [(&'static str, &'static str); 13] = [
        (
            "java",
            "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore extends Repository<ProductEntity, Long> {\n    Object deleteByStatus(String status);\n}\n",
        ),
        (
            "groovy",
            "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore extends Repository<ProductEntity, Long> {\n    Object deleteByStatus(String status)\n}\n",
        ),
        (
            "kt",
            "@Store(ProductEntity::class, OPTIONS)\ninterface ProductStore : Repository<ProductEntity, Long> {\n    fun deleteByStatus(status: String): Any?\n}\n",
        ),
        (
            "cs",
            "[Store(typeof(ProductEntity), OPTIONS)]\npublic interface ProductStore : Repository<ProductEntity, long> {\n    System.Threading.Tasks.Task<BehaviorJSON> deleteByStatus(BehaviorJSON status);\n}\n",
        ),
        (
            "php",
            "<?php\n#[Store(ProductEntity::class, OPTIONS)]\ninterface ProductStore extends Repository {\n    public function deleteByStatus(string $status): int;\n}\n",
        ),
        (
            "go",
            "// @Store(ProductEntity, OPTIONS)\ntype ProductStore interface {\n\tRepository[ProductEntity, int64]\n\tDeleteByStatus(status BehaviorValue) (BehaviorValue, error)\n}\n",
        ),
        (
            "dart",
            "@Store(ProductEntity, OPTIONS)\nabstract interface class ProductStore implements Repository<ProductEntity, int> {\n  Future<Object?> deleteByStatus(String status);\n}\n",
        ),
        (
            "rs",
            "#[Store(ProductEntity, OPTIONS)]\ntrait ProductStore: Repository<ProductEntity, i64> {\n    fn delete_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}\n",
        ),
        (
            "swift",
            "@Store(ProductEntity, OPTIONS)\nprotocol ProductStore: Repository {\n    func deleteByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON\n}\n",
        ),
        (
            "py",
            "from typing import Protocol\n\n@store(ProductEntity, OPTIONS)\nclass ProductStore(Repository[ProductEntity, int], Protocol):\n    async def delete_by_status(self, status: str) -> int: ...\n",
        ),
        (
            "js",
            "/** @extends {Repository<ProductEntity, number>} */\n@Store(ProductEntity, OPTIONS)\nexport default class ProductStore extends Repository {\n  /** @param {string} status */\n  async deleteByStatus(status) {}\n}\n",
        ),
        (
            "ts",
            "@Store(ProductEntity, OPTIONS)\nexport default class ProductStore extends Repository<ProductEntity, number> {\n  async deleteByStatus(status: string): Promise<number | void> {}\n}\n",
        ),
        (
            "rb",
            "#[Store(ProductEntity, OPTIONS)]\nclass ProductStore < Repository\n  def delete_by_status(status)\n  end\nend\n",
        ),
    ];

    const LOWERED: [(&'static str, [&'static str; 4]); 13] = [
        (
            "java",
            [
                "class ProductStore {",
                "Object findById(Long id) {}",
                "Object findAll(java.util.Map<String, Object> page) {}",
                "return behaviors.repository(\"ProductEntity\").save(entity);",
            ],
        ),
        (
            "groovy",
            [
                "class ProductStore {",
                "Object deleteById(Long id) {}",
                "Object deleteByStatus(String status) {}",
                "return behaviors.repository(\"ProductEntity\").save(entity);",
            ],
        ),
        (
            "kt",
            [
                "class ProductStore {",
                "fun findById(id: Long): Any? {}",
                "fun deleteByStatus(status: String): Any? {}",
                "fun save(entity: Any?): Any? = behaviors.repository(\"ProductEntity\").save(entity)",
            ],
        ),
        (
            "cs",
            [
                "public class ProductStore {",
                "public async System.Threading.Tasks.Task<BehaviorJSON> findById(long id) {}",
                "findAll(BehaviorJSON page) {}",
                "=> behaviors.repository(typeof(ProductEntity)).save(entity);",
            ],
        ),
        (
            "php",
            [
                "class ProductStore {",
                "public function findById(int|string $id): ?array {}",
                "public function deleteById(int|string $id): int {}",
                "return $this->behaviors->repository(ProductEntity::class)->save($entity);",
            ],
        ),
        (
            "go",
            [
                "func (store *ProductStore) DeleteByStatus(",
                "func (store *ProductStore) FindById(id BehaviorValue) (BehaviorValue, error) {\n}",
                "func (store *ProductStore) FindAll(page BehaviorValue) (BehaviorValue, error) {\n}",
                "BehaviorsRepository[ProductEntity](store.behaviors)",
            ],
        ),
        (
            "dart",
            [
                "class ProductStore {",
                "Future<Object?> findById(int id) async {}",
                "Future<Object?> deleteByStatus(String status) async {}",
                "=> behaviors.repository(ProductEntity).save(entity);",
            ],
        ),
        (
            "rs",
            [
                "struct ProductStore { behaviors: BehaviorContext }\nimpl ProductStore {",
                "fn find_by_id(&self, id: BehaviorValue) -> BehaviorResult<BehaviorValue> {}",
                "fn delete_by_status(&self, status: BehaviorValue) -> BehaviorResult<BehaviorValue> {}",
                "self.behaviors.repository::<ProductEntity>()?.save(entity)",
            ],
        ),
        (
            "swift",
            [
                "final class ProductStore {",
                "func findById(_ id: BehaviorJSON) async throws -> BehaviorJSON {}",
                "func deleteByStatus(_ status: BehaviorJSON) async throws -> BehaviorJSON {}",
                "try await behaviors.repository(ProductEntity.self).save(entity)",
            ],
        ),
        (
            "py",
            [
                "class ProductStore:",
                "    async def find_by_id(self, id): ...",
                "    async def find_all(self, page): ...",
                "        return await self.behaviors.repository(ProductEntity).save(entity)",
            ],
        ),
        (
            "js",
            [
                "export default class ProductStore {",
                "  async findById(id) {}",
                "  async deleteByStatus(status) {}",
                "    return this.behaviors.repository(ProductEntity).save(entity)",
            ],
        ),
        (
            "ts",
            [
                "export default class ProductStore {",
                "  async findById(id: number): Promise<ProductEntity | null | void> {}",
                "  async findAll(page: BehaviorPageInput): Promise<ProductEntity[] | void> {}",
                "    return this.behaviors.repository(ProductEntity).save(entity)",
            ],
        ),
        (
            "rb",
            [
                "class ProductStore\n",
                "  def find_by_id(id)\n  end",
                "  def delete_by_id(id)\n  end",
                "    @behaviors.repository(ProductEntity).save(entity)",
            ],
        ),
    ];

    fn contract(extension: &str) -> String {
        Self::CONTRACTS
            .iter()
            .find(|(language, _)| *language == extension)
            .map(|(_, source)| source.replace("OPTIONS", Self::OPTIONS))
            .expect("sample")
    }

    fn path(extension: &str) -> String {
        format!("server/stores/ProductStore.{extension}")
    }

    fn check(extension: &str, source: &str) -> Result<(), String> {
        let source = source.replace("OPTIONS", Self::OPTIONS);
        DeclaredContract::check(Path::new(&Self::path(extension)), &source)
            .map_err(|failure| failure.to_string())
    }
}

#[test]
fn every_language_accepts_a_store_that_extends_repository() {
    for (extension, source) in CrudSamples::CONTRACTS {
        let checked = CrudSamples::check(extension, source);
        assert!(checked.is_ok(), "{extension}: {checked:?}");
    }
}

#[test]
fn a_repository_store_gets_find_by_id_find_all_save_and_delete_by_id() {
    for (extension, expected) in CrudSamples::LOWERED {
        let source = CrudSamples::contract(extension);
        let lowered =
            DeclaredContract::concrete(Path::new(&CrudSamples::path(extension)), &source, "yon")
                .unwrap_or_else(|| panic!("{extension} was not lowered"));
        let code = lowered
            .replace("/** @extends {Repository<ProductEntity, number>} */", "")
            .replace("BehaviorsRepository", "");
        assert!(!code.contains("Repository"), "{extension}:\n{lowered}");
        for part in expected {
            assert!(
                lowered.contains(part),
                "{extension} lacks {part}:\n{lowered}"
            );
        }
    }
}

#[test]
fn a_browser_store_that_extends_repository_saves_through_its_own_context() {
    let source = CrudSamples::contract("ts");
    let lowered =
        DeclaredContract::concrete(Path::new("client/stores/ProductStore.ts"), &source, "tac")
            .expect("lowered");
    assert!(
        lowered.contains("static tachyonContext(signal: AbortSignal)"),
        "{lowered}"
    );
    assert!(
        lowered.contains("async deleteById(id: number)"),
        "{lowered}"
    );
    assert!(!lowered.contains("extends"), "{lowered}");
}

#[test]
fn only_a_store_extends_repository() {
    let source = "@HTTPClient({\"baseUrl\":\"https://example.com\"})\ninterface CatalogClient extends Repository<ProductEntity, Long> {\n    @GetRequest(\"/products\")\n    Object products();\n}\n";
    let message = CrudSamples::check("java", source).expect_err("client refused");
    assert!(message.contains("TY1308"), "{message}");
    assert!(
        message.contains("only a Store gets the Repository methods"),
        "{message}"
    );
}

#[test]
fn a_repository_names_the_entity_of_its_store() {
    let source = "@Store(ProductEntity.class, OPTIONS)\ninterface ProductStore extends Repository<OrderEntity, Long> {\n}\n";
    let message = CrudSamples::check("java", source).expect_err("other entity refused");
    assert!(
        message.contains("keeps ProductEntity in its Store marker but names OrderEntity"),
        "{message}"
    );
}

#[test]
fn an_fsharp_store_that_inherits_repository_gets_the_repository_methods() {
    let source = "open System.Threading.Tasks\n\n[<Store(typeof<ProductEntity>, {\"fields\":[\"id\",\"status\"],\"defaultSize\":20,\"maxSize\":100})>]\ntype ProductStore =\n    inherit Repository<ProductEntity, int>\n    abstract deleteByStatus: status: BehaviorJSON -> Task<BehaviorJSON>\n";
    let path = Path::new("server/stores/ProductStore.fs");
    let accepted = crate::annotation_layout::validate(path, source);
    assert!(accepted.is_ok(), "{accepted:?}");
    let lowered = DeclaredContract::concrete(path, source, "yon").expect("lowered");
    assert!(!lowered.contains("inherit"), "{lowered}");
    assert_eq!(
        lowered.lines().count(),
        source.lines().count() + 6,
        "{lowered}"
    );
    for part in [
        "    abstract findById: id: BehaviorJSON -> System.Threading.Tasks.Task<BehaviorJSON>\n",
        "    abstract findAll: page: BehaviorJSON -> System.Threading.Tasks.Task<BehaviorJSON>\n",
        "    member this.save(entity: BehaviorJSON) : System.Threading.Tasks.Task<BehaviorJSON> = this.behaviors.repository(typeof<ProductEntity>).save(entity)\n",
        "    abstract deleteByStatus: status: BehaviorJSON -> Task<BehaviorJSON>\n",
    ] {
        assert!(lowered.contains(part), "lacks {part}:\n{lowered}");
    }
}

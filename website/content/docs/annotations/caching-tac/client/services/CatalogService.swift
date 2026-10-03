@Service
@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
final class CatalogService {
    var behaviors: BehaviorContext!
    var sku: String = "anvil"
    var reads: Int = 0

    @OnStart
    func open() {
        behaviors = Self.openCatalog()
    }

    @Cached
    func find(_ sku: BehaviorJSON) async throws -> BehaviorJSON {
        reads += 1
        return entry(sku, .number(Double(reads)))
    }

    @UpdateCache({"method":"find"})
    func write(_ sku: BehaviorJSON, _ reads: BehaviorJSON) async throws -> BehaviorJSON {
        return entry(sku, reads)
    }

    @ClearCache({"method":"find"})
    func invalidate(_ sku: BehaviorJSON) async throws -> BehaviorJSON {
        return entry(sku, .number(Double(reads)))
    }

    private func entry(_ sku: BehaviorJSON, _ reads: BehaviorJSON) -> BehaviorJSON {
        .object(["sku": sku, "reads": reads])
    }

    private static func openCatalog() -> BehaviorContext? {
        try? Behaviors.create(CatalogSetup.self, partition: "public-catalog", signal: BehaviorSignal())
    }
}

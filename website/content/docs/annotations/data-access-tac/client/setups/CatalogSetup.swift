@Setup
final class CatalogSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.entities["publicCatalog"] = BehaviorEntityAdapter { command, _ in
            switch command.get("query").get("kind") {
            case .string("count"): return .number(0)
            default: return .array([])
            }
        }
        adapters.authorizeEntity = { command, _ in command.get("entity") == .string("ProductEntity") }
        return adapters
    }
}

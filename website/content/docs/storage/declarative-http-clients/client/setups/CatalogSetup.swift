@Setup
final class CatalogSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["catalog"] = BehaviorHTTPAdapter { request, execution in
            return try BehaviorJSON.parse("[\"anvil\",\"rope\"]")
        }
        return adapters
    }
}

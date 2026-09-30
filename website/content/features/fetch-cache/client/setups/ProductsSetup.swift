@Setup
final class ProductsSetup {
    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.http["products"] = BehaviorHTTPAdapter { request, execution in
            return try BehaviorJSON.parse("{\"items\":[\"anvil\",\"hammer\"]}")
        }
        return adapters
    }
}

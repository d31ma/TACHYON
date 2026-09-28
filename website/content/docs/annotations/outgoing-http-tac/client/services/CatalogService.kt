@Service
@HTTPClient
class CatalogService {
    lateinit var behaviors: Behaviors.Context
    var count: Int = 0

    @OnStart
    fun initialize() {
        behaviors = Behaviors.create("CatalogSetup", Behaviors.Options("catalog", Behaviors.Signal()))
    }

    @GetRequest("/api/products")
    fun all(): Any? { return mapOf<String, Any?>() }
}

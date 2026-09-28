@HTTPClient
class CatalogClient {
    lateinit var behaviors: Behaviors.Context

    @GetRequest("/products")
    fun all(@RequestParam("page") page: Long): Any? {}

    @PostRequest("/products")
    fun add(@RequestBody product: Any?): Any? {}

    @GetRequest("/products")
    fun first(): Any? { return mapOf("query" to mapOf("page" to 1)) }

    @OnStart
    fun initialize() {
        behaviors = Behaviors.create("CatalogSetup", Behaviors.Options("catalog", Behaviors.Signal()))
    }
}

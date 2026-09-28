@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class CatalogService {
    lateinit var behaviors: Behaviors.Context
    var sku: String = "anvil"
    var reads: Int = 0

    @OnStart
    fun open() {
        val options = Behaviors.Options("public-catalog", Behaviors.Signal())
        behaviors = Behaviors.create("CatalogSetup", options)
    }

    @Cached
    fun find(sku: String): Any? {
        reads++
        return entry(sku, reads)
    }

    @UpdateCache({"method":"find"})
    fun write(sku: String, reads: Int): Any? { return entry(sku, reads) }

    @ClearCache({"method":"find"})
    fun invalidate(sku: String): Any? { return entry(sku, reads) }

    private fun entry(sku: String, reads: Int): Any? = mapOf("sku" to sku, "reads" to reads)
}

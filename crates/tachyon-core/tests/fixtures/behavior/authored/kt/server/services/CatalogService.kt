@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":128})
@Service
class CatalogService {
    lateinit var behaviors: Behaviors.Context
    var calls: Int = 0
    var mutations: Int = 0
    var validCalls: Int = 0
    var priceCalls: Int = 0
    @Cached
    fun list(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) page: Map<String, Any?>): Any? {
        calls++
        return mapOf("calls" to calls, PAGE to page)
    }
    @ClearCache({"method":"list"})
    fun remove(ignored: String, page: Map<String, Any?>): Any? {
        mutations++
        return true
    }
    @UpdateCache({"method":"list"})
    fun update(value: String, page: Map<String, Any?>): Any? {
        mutations++
        return mapOf(UPDATED to value)
    }
    @UpdateCache({"method":"list"})
    fun invalid(page: Map<String, Any?>): Any? { return Any() }
    @Cached
    fun price(@CacheKey sku: String, note: String): Any? {
        priceCalls++
        return priceCalls
    }
    @Cached
    fun checked(@Check(Input::class) input: Any?): Any? {
        validCalls++
        return input
    }
    companion object {
        private const val PAGE = "page"
        private const val UPDATED = "updated"
    }
}

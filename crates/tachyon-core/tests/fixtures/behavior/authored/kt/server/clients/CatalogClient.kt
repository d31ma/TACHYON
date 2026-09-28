@CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":128})
@HTTPClient({"adapter": "api"})
class CatalogClient {
    lateinit var behaviors: Behaviors.Context
    var calls: Int = 0
    var mutations: Int = 0
    var validCalls: Int = 0
    @Cached
    fun list(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) page: Map<String, Any?>): Any? {
        calls++
        return mapOf("calls" to calls, PAGE to page)
    }
    @GetRequest("/products")
    fun remote(): Any? { return mapOf("query" to mapOf(PAGE to 1)) }
    @GetRequest("/products")
    fun search(@RequestParam("page") page: Long): Any? {}
    @GetRequest("/products/{id}")
    fun find(@PathVariable id: Long): Any? {}
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
    @UpdateCache({"method":"list"})
    @PostRequest("/products")
    fun remoteUpdate(page: Map<String, Any?>): Any? { return mapOf(BODY to mapOf(UPDATED to true)) }
    @Cached
    fun checked(@Check(Input::class) input: Any?): Any? {
        validCalls++
        return input
    }
    @PostRequest("/validated")
    fun checkedRemote(@Check(Input::class) input: Any?): Any? {
        validCalls++
        return mapOf(BODY to input)
    }
    companion object {
        private const val PAGE = "page"
        private const val UPDATED = "updated"
        private const val BODY = "body"
    }
}

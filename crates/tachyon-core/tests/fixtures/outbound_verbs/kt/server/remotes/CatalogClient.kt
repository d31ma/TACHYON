@HTTPClient("/api")
interface CatalogClient {
    @GetRequest("/products/{id}")
    fun find(@PathVariable("id") id: String, @RequestParam("view") view: String, @RequestHeader("X-Tenant") tenant: String): Any?

    @PostRequest("/products")
    fun save(@RequestBody product: Any?): Any?

    @PutRequest("/products/{id}")
    fun replace(@PathVariable("id") id: Long, @RequestBody product: Any?): Any?

    @PatchRequest("/products/{id}")
    fun rename(@PathVariable("id") id: Long, @RequestBody product: Any?): Any?

    @DeleteRequest("/products/{id}")
    fun remove(@PathVariable("id") id: Long): Any?
}

@HTTPClient
interface CatalogClient {
    @GetRequest("/products")
    fun all(@RequestParam("page") page: Long): Any?

    @PostRequest("/products")
    fun add(@RequestBody product: Any?): Any?
}

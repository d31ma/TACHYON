@HTTPClient
interface CatalogClient {
    @GetRequest("/api/products")
    fun all(): Any?
}

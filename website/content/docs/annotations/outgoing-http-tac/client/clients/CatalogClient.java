@HTTPClient
interface CatalogClient {
    @GetRequest("/api/products")
    Object all();
}

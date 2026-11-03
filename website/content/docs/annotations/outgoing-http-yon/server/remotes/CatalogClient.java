@HTTPClient
interface CatalogClient {
    @GetRequest("/products")
    Object all(@RequestParam("page") long page);

    @PostRequest("/products")
    Object add(@RequestBody Object product);
}

@HTTPClient("/api")
interface CatalogClient {
    @GetRequest("/products/{id}")
    Object find(@PathVariable("id") String id, @RequestParam("view") String view, @RequestHeader("X-Tenant") String tenant)

    @PostRequest("/products")
    Object save(@RequestBody Object product)

    @PutRequest("/products/{id}")
    Object replace(@PathVariable("id") long id, @RequestBody Object product)

    @PatchRequest("/products/{id}")
    Object rename(@PathVariable("id") long id, @RequestBody Object product)

    @DeleteRequest("/products/{id}")
    Object remove(@PathVariable("id") long id)
}

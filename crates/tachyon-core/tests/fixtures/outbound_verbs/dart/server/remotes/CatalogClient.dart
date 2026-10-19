@HTTPClient("/api")
abstract interface class CatalogClient {
  @GetRequest("/products/{id}")
  Future<Object?> find(@PathVariable("id") String id, @RequestParam("view") String view, @RequestHeader("X-Tenant") String tenant);

  @PostRequest("/products")
  Future<Object?> save(@RequestBody Object? product);

  @PutRequest("/products/{id}")
  Future<Object?> replace(@PathVariable("id") int id, @RequestBody Object? product);

  @PatchRequest("/products/{id}")
  Future<Object?> rename(@PathVariable("id") int id, @RequestBody Object? product);

  @DeleteRequest("/products/{id}")
  Future<Object?> remove(@PathVariable("id") int id);
}

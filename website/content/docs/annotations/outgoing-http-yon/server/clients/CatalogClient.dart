@HTTPClient
abstract interface class CatalogClient {
  @GetRequest("/products")
  Future<Object?> all(@RequestParam("page") int page);

  @PostRequest("/products")
  Future<Object?> add(@RequestBody Object? product);
}

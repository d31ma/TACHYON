@HTTPClient
abstract interface class CatalogClient {
  @GetRequest("/api/products")
  Future<Object?> all();
}

@HTTPClient
class CatalogClient {
  @GetRequest("/products")
  async remote() {}

  @GetRequest("/products")
  async search(@RequestParam("page") page, @RequestHeader("X-Tenant") tenant) {}

  @GetRequest("/products/{id}")
  async lookup(@PathVariable id) {}
}

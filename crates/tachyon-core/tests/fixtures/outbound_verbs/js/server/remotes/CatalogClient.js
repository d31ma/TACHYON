@HTTPClient("/api")
export default class CatalogClient {
  @GetRequest("/products/{id}")
  async find(@PathVariable("id") id, @RequestParam("view") view, @RequestHeader("X-Tenant") tenant) {}

  @PostRequest("/products")
  async save(@RequestBody product) {}

  @PutRequest("/products/{id}")
  async replace(@PathVariable("id") id, @RequestBody product) {}

  @PatchRequest("/products/{id}")
  async rename(@PathVariable("id") id, @RequestBody product) {}

  @DeleteRequest("/products/{id}")
  async remove(@PathVariable("id") id) {}
}

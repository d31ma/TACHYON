@HTTPClient({"adapter": "api", "setup": "RuntimeAdaptersSetup"})
interface CatalogClient {
    @GetRequest("/products")
    Object remote(@RequestParam("page") long page)
    @GetRequest("/products")
    Object search(@RequestParam("page") long page)
    @GetRequest("/products/{id}")
    Object find(@PathVariable long id)
    @PostRequest("/validated")
    Object checkedRemote(@RequestBody @Check(Input.class) Object input)
}

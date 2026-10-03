@HTTPClient({"adapter": "api", "setup": "RuntimeAdaptersSetup"})
interface CatalogClient {
    @GetRequest("/products")
    fun remote(@RequestParam("page") page: Long): Any?
    @GetRequest("/products")
    fun search(@RequestParam("page") page: Long): Any?
    @GetRequest("/products/{id}")
    fun find(@PathVariable id: Long): Any?
    @PostRequest("/validated")
    fun checkedRemote(@RequestBody @Check(Input::class) input: Any?): Any?
}

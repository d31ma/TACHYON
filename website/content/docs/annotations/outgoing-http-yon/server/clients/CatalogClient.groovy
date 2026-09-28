@HTTPClient
class CatalogClient {
    Behaviors.Context behaviors

    @GetRequest("/products")
    Object all(@RequestParam("page") long page) {}

    @PostRequest("/products")
    Object add(@RequestBody Object product) {}

    @GetRequest("/products")
    Object first() { return java.util.Map.of("query", java.util.Map.of("page", 1)) }

    @OnStart
    void initialize() {
        behaviors = Behaviors.create("CatalogSetup", new Behaviors.Options("catalog", new Behaviors.Signal()))
    }
}

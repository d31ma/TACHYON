@HTTPClient
protocol CatalogClient {
    @GetRequest("/api/products")
    func all() async throws -> BehaviorJSON
}

@HTTPClient
protocol ApplicationClient {
    @GetRequest("/manifest.webmanifest")
    func load() async throws -> BehaviorJSON
}

@HTTPClient("/api")
protocol OrderClient {
    @PostRequest("/orders")
    func send(@RequestBody order: BehaviorJSON) async throws -> OrderInput
}

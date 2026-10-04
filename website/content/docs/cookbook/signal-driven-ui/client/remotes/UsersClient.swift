@HTTPClient
protocol UsersClient {
    @PostRequest("/users")
    func save(@RequestBody name: BehaviorJSON) async throws -> BehaviorJSON
}

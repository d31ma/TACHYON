@HTTPClient({"adapter":"users","setup":"UsersSetup"})
protocol UsersClient {
    @PostRequest("/users")
    func save(@RequestBody name: BehaviorJSON) async throws -> BehaviorJSON

    @GetRequest("/users")
    func all() async throws -> BehaviorJSON
}

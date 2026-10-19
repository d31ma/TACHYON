@HTTPClient({"path":"/api","adapter":"orders","setup":"OrdersSetup"})
protocol OrdersClient {
    @PutRequest("/orders/{id}")
    func replace(@PathVariable("id") id: String, @RequestBody order: BehaviorJSON, @RequestHeader("X-Tenant") tenant: String) async throws -> SentRequest

    @PatchRequest("/orders/{id}")
    func change(@PathVariable("id") id: String, @RequestBody change: BehaviorJSON, @RequestHeader("X-Tenant") tenant: String) async throws -> SentRequest

    @DeleteRequest("/orders/{id}")
    func remove(@PathVariable("id") id: String, @RequestHeader("X-Reason") reason: String) async throws -> SentRequest
}

@HTTPClient("/api")
protocol OverrideClient {
    @GetRequest("/never")
    func read() async throws -> Greeting
}

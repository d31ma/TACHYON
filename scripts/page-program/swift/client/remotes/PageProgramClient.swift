@HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})
protocol PageProgramClient {
    @GetRequest("/read")
    func read() async throws -> ProgramState

    @PostRequest("/increment")
    func increment() async throws -> ProgramState
}

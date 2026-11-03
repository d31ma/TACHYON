@HTTPClient({"adapter":"pageprogram","path":"/program","setup":"PageProgramSetup"})
abstract interface class PageProgramClient {
  @GetRequest("/read")
  Future<ProgramState> read();

  @PostRequest("/increment")
  Future<ProgramState> increment();
}

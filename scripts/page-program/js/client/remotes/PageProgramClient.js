@HTTPClient({"adapter":"pageprogram","path":"/program"})
export default class PageProgramClient {
  /** @returns {Promise<ProgramState>} */
  @GetRequest("/read")
  async read() {}

  /** @returns {Promise<ProgramState>} */
  @PostRequest("/increment")
  async increment() {}
}

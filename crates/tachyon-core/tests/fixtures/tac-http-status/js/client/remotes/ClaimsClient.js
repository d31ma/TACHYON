@HTTPClient({"path":"/api","adapter":"claims"})
export default class ClaimsClient {
  /**
   * @param {string} id
   * @returns {Promise<BehaviorJsonValue>}
   */
  @PutRequest("/claims/{id}")
  async claim(@PathVariable("id") id) {}
}

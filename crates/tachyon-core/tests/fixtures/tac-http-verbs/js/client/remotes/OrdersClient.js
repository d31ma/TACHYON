@HTTPClient({"path":"/api","adapter":"orders"})
export default class OrdersClient {
  /**
   * @param {string} id
   * @param {{ item: string }} order
   * @param {string} tenant
   * @returns {Promise<SentRequest>}
   */
  @PutRequest("/orders/{id}")
  async replace(@PathVariable("id") id, @RequestBody order, @RequestHeader("X-Tenant") tenant) {}

  /**
   * @param {string} id
   * @param {{ item: string }} change
   * @param {string} tenant
   * @returns {Promise<SentRequest>}
   */
  @PatchRequest("/orders/{id}")
  async change(@PathVariable("id") id, @RequestBody change, @RequestHeader("X-Tenant") tenant) {}

  /**
   * @param {string} id
   * @param {string} reason
   * @returns {Promise<SentRequest>}
   */
  @DeleteRequest("/orders/{id}")
  async remove(@PathVariable("id") id, @RequestHeader("X-Reason") reason) {}
}

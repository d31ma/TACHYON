@HTTPClient("/api")
export default class OrderClient {
  /**
   * @param {OrderInput} order
   * @returns {Promise<OrderInput | void>}
   */
  @PostRequest("/orders")
  async send(@RequestBody order) {}
}

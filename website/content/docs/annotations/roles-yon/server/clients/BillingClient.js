@HTTPClient("/billing")
export default class BillingClient {
  /** @returns {Promise<BehaviorJsonValue | void>} */
  @GetRequest("/surcharge")
  async surcharge() {}
}

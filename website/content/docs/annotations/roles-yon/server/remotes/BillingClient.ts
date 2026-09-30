@HTTPClient("/billing")
export default class BillingClient {
  @GetRequest("/surcharge")
  async surcharge(): Promise<BehaviorJsonValue | void> {}
}

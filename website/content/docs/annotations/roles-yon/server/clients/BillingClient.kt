@HTTPClient("/billing")
interface BillingClient {
  @GetRequest("/surcharge")
  fun surcharge(): Any?
}

@HTTPClient("/billing")
interface BillingClient {
  @GetRequest("/surcharge")
  Object surcharge();
}

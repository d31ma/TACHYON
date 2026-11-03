@HTTPClient("/billing")
abstract interface class BillingClient {
  @GetRequest("/surcharge")
  Future<Object?> surcharge();
}

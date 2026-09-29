[HTTPClient("/billing")]
interface BillingClient {
  [GetRequest("/surcharge")]
  System.Threading.Tasks.Task<BehaviorJSON> Surcharge();
}

[HTTPClient("/billing")]
public interface BillingClient {
  [GetRequest("/surcharge")]
  System.Threading.Tasks.Task<BehaviorJSON> Surcharge();
}

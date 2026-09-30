#[HTTPClient("/billing")]
trait BillingClient {
    #[GetRequest("/surcharge")]
    fn surcharge(&self) -> BehaviorResult<BehaviorValue>;
}

#[HTTPClient]
struct BillingClient {}
impl BillingClient {
    fn new() -> Self { Self {} }
    fn surcharge(&self) -> i64 { 2 }
}

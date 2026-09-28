#[Helper]
struct BillingHelper {
    #[Needs]
    client: std::sync::Arc<BillingClient>,
}
impl BillingHelper {
    fn settle(&self, subtotal: i64) -> i64 {
        let surcharge = self.client.surcharge();
        subtotal + surcharge
    }
}

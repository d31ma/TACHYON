#[Service]
struct BillingService {
    #[Needs]
    repository: std::sync::Arc<BillingStore>,
    #[Needs]
    helper: std::sync::Arc<BillingHelper>,
}
impl BillingService {
    fn total(&self) -> i64 {
        let subtotal = self.repository.subtotal();
        self.helper.settle(subtotal)
    }
}

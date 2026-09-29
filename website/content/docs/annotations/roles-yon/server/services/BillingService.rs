#[Service]
struct BillingService {
    #[Needs]
    helper: std::sync::Arc<BillingHelper>,
}
impl BillingService {
    const SUBTOTAL: i64 = 40;

    fn total(&self) -> i64 {
        self.helper.settle(Self::SUBTOTAL)
    }
}

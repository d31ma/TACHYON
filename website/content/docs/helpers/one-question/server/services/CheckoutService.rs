#[Service]
struct CheckoutService {
    #[Needs]
    helper: std::sync::Arc<TaxHelper>,
}
impl CheckoutService {
    fn total(&self) -> i64 {
        let subtotal = self.subtotal();
        self.helper.add_tax(subtotal)
    }

    fn subtotal(&self) -> i64 {
        40
    }
}

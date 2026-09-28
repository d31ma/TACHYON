#[Helper]
struct TaxHelper {
    #[Needs]
    store: std::sync::Arc<TaxRateStore>,
}
impl TaxHelper {
    fn add_tax(&self, subtotal: i64) -> i64 {
        let tax = self.store.rate();
        subtotal + tax
    }
}

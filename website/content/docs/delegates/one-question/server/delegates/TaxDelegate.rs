#[Delegate]
struct TaxDelegate {
    #[Needs]
    rates: std::sync::Arc<TaxRateDelegate>,
}
impl TaxDelegate {
    fn add_tax(&self, subtotal: i64) -> i64 {
        let tax = self.rates.rate();
        subtotal + tax
    }
}

#[Helper]
struct TaxHelper {
    #[Needs]
    rates: std::sync::Arc<TaxRateHelper>,
}
impl TaxHelper {
    fn add_tax(&self, subtotal: i64) -> i64 {
        let tax = self.rates.rate();
        subtotal + tax
    }
}

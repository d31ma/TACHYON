#[Delegate]
struct TaxDelegate {
    #[Needs]
    tax_service: std::sync::Arc<TaxService>,
}
impl TaxDelegate {
    fn rate(&self, region: &str) -> i64 {
        self.tax_service.rate(region) * 2
    }

    fn total_with(&self, count: i64, extra: i64) -> i64 {
        self.tax_service.total(count) + extra
    }
}

#[Helper]
#[Lifetime("transient")]
struct LedgerHelper {
    #[Needs]
    repository: std::sync::Arc<LedgerStore>,
}
impl LedgerHelper {
    fn settle(&self, subtotal: i64) -> i64 {
        let surcharge = self.repository.surcharge();
        subtotal + surcharge
    }
}

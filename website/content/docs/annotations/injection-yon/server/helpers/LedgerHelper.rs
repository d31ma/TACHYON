#[Helper]
#[Lifetime("transient")]
struct LedgerHelper {
    #[Needs]
    amounts: std::sync::Arc<LedgerAmountsHelper>,
}
impl LedgerHelper {
    fn settle(&self, subtotal: i64) -> i64 {
        let surcharge = self.amounts.surcharge();
        subtotal + surcharge
    }
}

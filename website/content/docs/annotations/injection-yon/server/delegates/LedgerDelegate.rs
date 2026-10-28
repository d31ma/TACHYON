#[Delegate]
#[Lifetime("transient")]
struct LedgerDelegate {
    #[Needs]
    amounts: std::sync::Arc<LedgerAmountsDelegate>,
}
impl LedgerDelegate {
    fn settle(&self, subtotal: i64) -> i64 {
        let surcharge = self.amounts.surcharge();
        subtotal + surcharge
    }
}

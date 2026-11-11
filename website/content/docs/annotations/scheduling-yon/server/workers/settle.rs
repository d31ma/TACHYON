#[Schedule({"fixedDelaySeconds":300})]
struct Handler {
    #[Needs]
    ledger: std::sync::Arc<LedgerService>,
}
impl Handler {
    fn POST(&self) -> i64 {
        self.ledger.total()
    }
}

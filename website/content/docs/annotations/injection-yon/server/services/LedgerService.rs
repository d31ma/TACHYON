#[Service]
struct LedgerService {
    #[Needs]
    amounts: std::sync::Arc<LedgerAmountsHelper>,
}
impl LedgerService {
    fn subtotal(&self) -> i64 { self.amounts.subtotal() }
}

#[Service]
struct LedgerService {
    #[Needs]
    repository: std::sync::Arc<LedgerStore>,
}
impl LedgerService {
    fn subtotal(&self) -> i64 { self.repository.subtotal() }
}

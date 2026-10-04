#[Service]
struct AuditService {
    #[Needs]
    #[Named("CartHelper")]
    ledger: CartHelper,
}
impl AuditService {
    fn lines(&self) -> i64 {
        self.ledger.count()
    }
}

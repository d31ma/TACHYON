#[Service]
struct AuditService {
    #[Needs]
    #[Named("CartHelper")]
    ledger: CartHelper,
}
impl AuditService {
    pub fn lines(&self) -> i64 {
        self.ledger.count()
    }
}

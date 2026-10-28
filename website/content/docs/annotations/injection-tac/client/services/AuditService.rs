#[Service]
struct AuditService {
    #[Needs]
    #[Named("CartDelegate")]
    ledger: CartDelegate,
}
impl AuditService {
    pub fn lines(&self) -> i64 {
        self.ledger.count()
    }
}

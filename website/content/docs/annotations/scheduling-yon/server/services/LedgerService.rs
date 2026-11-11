#[Service]
struct LedgerService {}
impl LedgerService {
    fn new() -> Self {
        Self {}
    }

    fn total(&self) -> i64 {
        42
    }
}

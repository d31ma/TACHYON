#[Delegate]
#[Lifetime("singleton")]
struct LedgerAmountsDelegate {}
impl LedgerAmountsDelegate {
    fn new() -> Self { Self {} }
    fn subtotal(&self) -> i64 { 40 }
    fn surcharge(&self) -> i64 { 2 }
}

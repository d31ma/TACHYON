#[Store]
#[Lifetime("singleton")]
struct LedgerStore {}
impl LedgerStore {
    fn new() -> Self { Self {} }
    fn subtotal(&self) -> i64 { 40 }
    fn surcharge(&self) -> i64 { 2 }
}

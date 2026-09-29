#[Helper]
#[Lifetime("singleton")]
struct LedgerAmountsHelper {}
impl LedgerAmountsHelper {
    fn new() -> Self { Self {} }
    fn subtotal(&self) -> i64 { 40 }
    fn surcharge(&self) -> i64 { 2 }
}

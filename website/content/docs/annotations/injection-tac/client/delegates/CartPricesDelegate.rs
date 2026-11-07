#[Delegate]
struct CartPricesDelegate {}
impl CartPricesDelegate {
    fn new() -> Self { Self {} }
    fn subtotal(&self) -> i64 { 40 }
    fn surcharge(&self) -> i64 { 2 }
    fn count(&self) -> i64 { 3 }
}

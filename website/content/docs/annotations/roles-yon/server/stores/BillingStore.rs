#[Store]
struct BillingStore {}
impl BillingStore {
    fn new() -> Self { Self {} }
    fn subtotal(&self) -> i64 { 40 }
}

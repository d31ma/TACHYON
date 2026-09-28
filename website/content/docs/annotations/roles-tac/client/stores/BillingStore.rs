#[tac::store()]
struct BillingStore {}
impl BillingStore {
    fn subtotal(&self) -> i64 { 40 }
}

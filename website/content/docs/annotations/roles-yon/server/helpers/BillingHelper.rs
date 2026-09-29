#[Helper]
struct BillingHelper {}
impl BillingHelper {
    const SURCHARGE: i64 = 2;

    fn new() -> Self { Self {} }

    fn settle(&self, subtotal: i64) -> i64 {
        subtotal + Self::SURCHARGE
    }
}

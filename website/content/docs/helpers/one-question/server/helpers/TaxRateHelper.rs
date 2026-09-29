#[Helper]
struct TaxRateHelper {}
impl TaxRateHelper {
    fn new() -> Self { Self {} }
    fn rate(&self) -> i64 { 2 }
}

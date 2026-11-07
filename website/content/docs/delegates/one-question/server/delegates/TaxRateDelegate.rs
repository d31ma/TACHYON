#[Delegate]
struct TaxRateDelegate {}
impl TaxRateDelegate {
    fn new() -> Self { Self {} }
    fn rate(&self) -> i64 { 2 }
}

#[Store]
struct TaxRateStore {}
impl TaxRateStore {
    fn new() -> Self { Self {} }
    fn rate(&self) -> i64 { 2 }
}

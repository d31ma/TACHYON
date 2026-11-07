#[Service]
struct TaxService {}
impl TaxService {
    fn new() -> Self { Self {} }

    fn rate(&self, _region: &str) -> i64 {
        2
    }

    fn total(&self, count: i64) -> i64 {
        count * 10
    }
}

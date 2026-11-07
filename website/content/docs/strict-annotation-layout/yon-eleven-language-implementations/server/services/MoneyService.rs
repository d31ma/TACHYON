#[yon::service]
struct MoneyService {}
impl MoneyService {
    fn new() -> Self { Self {} }
    fn amount(&self) -> i64 { 42 }
}

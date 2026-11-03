#[Service]
struct MoneyService { amount: i64 }
impl MoneyService {
    fn new() -> Self { Self { amount: 42 } }
}

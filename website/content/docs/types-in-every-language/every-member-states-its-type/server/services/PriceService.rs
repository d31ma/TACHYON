use crate::models::Price;

#[Service]
struct PriceService {}
impl PriceService {
    const CURRENCY: &'static str = "EUR";

    fn new() -> Self { Self {} }

    fn price_for(&self, amount: i64, count: i64) -> Price {
        Price { amount: amount * count, currency: String::from(Self::CURRENCY) }
    }
}

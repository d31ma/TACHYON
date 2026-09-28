use crate::models::Money;
use std::sync::Arc;

#[RequestRoute("/money")]
#[Public]
struct MoneyController {
  #[Needs]
  money: Arc<MoneyService>,
}
impl MoneyController {
    #[GetRoute]
    fn get_money(&self) -> Money {
        Money { amount: self.money.amount() }
    }
}

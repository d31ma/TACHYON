use crate::models::OrderInput;

#[yon::service]
struct OrderService {}
impl OrderService {
    fn new() -> Self { Self {} }
    fn accept(&self, #[Check(OrderInput)] order: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(order) }
}

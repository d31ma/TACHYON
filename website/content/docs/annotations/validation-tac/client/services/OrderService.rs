use crate::models::OrderInput;

#[Service]
struct OrderService { status: String }
impl OrderService {
    fn new() -> Self { Self { status: String::from("ready") } }
    fn accept(&self, #[Check(OrderInput)] order: BehaviorValue) -> BehaviorResult<BehaviorValue> { Ok(order) }
}

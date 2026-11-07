use crate::models::Problem;

#[RequestRoute("/orders")]
#[Public]
struct OrdersController {}

impl OrdersController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    fn get_order(&self) -> Result<Problem, OrderNotFound> {
        Err(OrderNotFound)
    }
}

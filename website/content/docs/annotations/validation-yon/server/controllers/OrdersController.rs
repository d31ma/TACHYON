use std::sync::Arc;

#[RequestRoute("/orders")]
#[Public]
struct OrdersController {
  #[Needs]
  orders: Arc<OrderService>,
}
impl OrdersController {
    #[PostRoute]
    fn post_orders(&self, #[FromBody] order: Json) -> Result<Json, BehaviorError> {
        self.orders.accept(order.into()).map(Json::from)
    }
}

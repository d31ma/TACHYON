use std::sync::Arc;

#[RequestRoute("/orders")]
#[Allow(Admin)]
struct OrdersController {
    #[Needs]
    orders: Arc<OrderService>,
}
impl OrdersController {
    #[GetRoute]
    fn get_orders(&self) -> Result<Vec<String>, std::io::Error> {
        Err(std::io::Error::other("the order store is down"))
    }

    #[PostRoute]
    fn post_order(&self, #[FromBody] order: Json) -> Result<Json, BehaviorError> {
        self.orders.accept(order.into()).map(Json::from)
    }
}

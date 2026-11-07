use crate::models::OrderList;

#[RequestRoute("/orders")]
#[Public]
struct OrdersController {}

impl OrdersController {
    fn new() -> Self { Self {} }
    #[GetRoute]
    fn get_orders(&self) -> OrderList {
        let orders = OrderService::list().into_iter().map(String::from).collect();
        OrderList { orders }
    }
}

use std::sync::Arc;

#[RequestRoute("/orders")]
#[Public]
struct OrdersController {
  #[Needs]
  orders: Arc<OrderService>,
}
impl OrdersController {
    #[PostRoute]
    fn post_orders(&self, #[FromBody] order: Json) -> http::Response<Vec<u8>> {
        Self::answer(Self::checked_text(&self.orders, order))
    }

    fn checked_text(orders: &OrderService, order: Json) -> Option<String> {
        let checked = orders.accept(order.into()).ok()?;
        checked.to_json(65_536).ok()
    }

    fn answer(text: Option<String>) -> http::Response<Vec<u8>> {
        match text {
            Some(text) => Self::response(200, text),
            None => Self::response(400, String::from("{\"error\":\"validation\"}")),
        }
    }

    fn response(status: u16, body: String) -> http::Response<Vec<u8>> {
        http::Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(body.into_bytes())
            .unwrap_or_default()
    }
}

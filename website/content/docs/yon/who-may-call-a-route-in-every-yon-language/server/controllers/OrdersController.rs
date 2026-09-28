#[RequestRoute("/orders")]
#[SignedIn]
struct OrdersController {}
impl OrdersController {
    fn new() -> Self {
        Self {}
    }

    #[GetRoute]
    #[Public]
    fn get_orders(&self) -> Vec<String> {
        vec![String::from("lamp"), String::from("desk")]
    }

    #[PostRoute]
    fn post_order(&self) -> Vec<String> {
        vec![String::from("chair")]
    }

    #[DeleteRoute]
    #[Allow(Admin)]
    fn delete_orders(&self) -> Vec<String> {
        Vec::new()
    }
}

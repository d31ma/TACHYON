use crate::models::Item;

#[RequestRoute("/items")]
#[Public]
struct ItemsController {}
impl ItemsController {
    fn new() -> Self { Self {} }

    #[GetRoute("/{id}")]
    fn get_item(&self, #[FromPath] id: i64) -> Item {
        Item { id }
    }
}

use crate::models::Item;

#[RequestRoute("/items")]
#[Public]
struct ItemsController {}
impl ItemsController {
    fn new() -> Self { Self {} }

    #[GetRoute("/{id}")]
    fn get_item(&self, #[FromPath] id: String) -> Item {
        Item { name: id }
    }

    #[PostRoute]
    #[ResponseStatus(201)]
    fn post_item(&self, #[FromBody] item: Item) -> Item {
        item
    }
}

use crate::models::{Item, ItemList};

#[RequestRoute("/items")]
#[Public]
struct ItemsController {}
impl ItemsController {
    fn new() -> Self { Self {} }

    #[GetRoute]
    fn get_items(&self) -> ItemList {
        ItemList { items: Vec::new() }
    }

    #[PostRoute]
    #[ResponseStatus(201)]
    fn post_item(&self, #[FromBody] item: Item) -> Item {
        item
    }
}

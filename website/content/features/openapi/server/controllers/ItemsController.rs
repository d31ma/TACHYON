use crate::models::Item;

#[RequestRoute("/items")]
#[Public]
struct ItemsController {}
impl ItemsController {
    fn new() -> Self { Self {} }

    #[GetRoute("/{id}")]
    fn get_item(&self, #[FromPath] id: i64) -> Item {
        Item { name: String::from("anvil"), level: id }
    }

    #[PatchRoute("/{id}")]
    fn patch_item(&self, #[FromPath] id: i64, #[FromBody] item: Item) -> Item {
        Item { name: item.name, level: id }
    }
}

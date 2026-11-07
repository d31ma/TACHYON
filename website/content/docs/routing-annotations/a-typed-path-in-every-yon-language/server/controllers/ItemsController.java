import models.Item;

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute("/{id}")
    Item getItem(@FromPath long id) {
        return new Item(id);
    }
}

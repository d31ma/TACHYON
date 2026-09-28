import models.Item;

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute("/{id}")
    Item getItem(@FromPath long id) {
        return new Item("anvil", id);
    }

    @PatchRoute("/{id}")
    Item patchItem(@FromPath long id, @FromBody Item item) {
        return new Item(item.name(), id);
    }
}

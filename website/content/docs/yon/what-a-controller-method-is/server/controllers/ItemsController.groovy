import java.util.List
import models.Item
import models.ItemList

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute
    ItemList getItems() {
        return new ItemList(List.of())
    }

    @PostRoute
    @ResponseStatus(201)
    Item postItem(@FromBody Item item) {
        return item
    }
}

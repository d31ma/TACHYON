import models.Item

@RequestRoute("/items")
@Public
class ItemsController {
    @GetRoute("/{id}")
    Item getItem(@FromPath String id) {
        return new Item(id)
    }

    @PostRoute
    @ResponseStatus(201)
    Item postItem(@FromBody Item item) {
        return item
    }
}

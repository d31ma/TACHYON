using Models;

[RequestRoute("/items")]
[Public]
class ItemsController {
    [GetRoute("/{id}")]
    public Item GetItem([FromPath] string id) {
        return new Item(id);
    }

    [PostRoute]
    [ResponseStatus(201)]
    public Item PostItem([FromBody] Item item) {
        return item;
    }
}

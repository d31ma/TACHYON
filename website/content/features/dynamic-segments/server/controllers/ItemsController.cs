using Models;

[RequestRoute("/items")]
[Public]
class ItemsController {
    [GetRoute("/{id}")]
    public Item GetItem([FromPath] long id) {
        return new Item(id);
    }
}

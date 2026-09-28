using Models;

[RequestRoute("/items")]
[Public]
class ItemsController {
    [GetRoute("/{id}")]
    public Item GetItem([FromPath] long id) {
        return new Item("anvil", id);
    }

    [PatchRoute("/{id}")]
    public Item PatchItem([FromPath] long id, [FromBody] Item item) {
        return new Item(item.Name, id);
    }
}

using System.Collections.Generic;
using Models;

[RequestRoute("/items")]
[Public]
class ItemsController {
    [GetRoute]
    public ItemList GetItems() {
        return new ItemList(new List<Item>());
    }

    [PostRoute]
    [ResponseStatus(201)]
    public Item PostItem([FromBody] Item item) {
        return item;
    }
}

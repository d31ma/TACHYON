import type { Item } from '/models/Item.ts'
import type { ItemList } from '/models/ItemList.ts'

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute
  getItems(): ItemList {
    return { items: [] }
  }

  @PostRoute
  @ResponseStatus(201)
  postItem(@FromBody item: Item): Item {
    return item
  }
}

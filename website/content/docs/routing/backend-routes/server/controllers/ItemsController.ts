import type { Item } from '/models/Item.ts'

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute("/{id}")
  getItem(@FromPath id: string): Item {
    return { name: id }
  }

  @PostRoute
  @ResponseStatus(201)
  postItem(@FromBody item: Item): Item {
    return item
  }
}

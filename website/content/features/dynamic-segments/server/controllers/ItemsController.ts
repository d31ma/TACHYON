import type { Item } from '/models/Item.ts'

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute("/{id}")
  getItem(@FromPath id: number): Item {
    return { id }
  }
}

import type { Item } from '/models/Item.ts';

@RequestRoute("/items")
@Public
class ItemsController {
  @GetRoute("/{id}")
  getItem(@FromPath id: number): Item {
    return { name: 'anvil', level: id }
  }

  @PatchRoute("/{id}")
  patchItem(@FromPath id: number, @FromBody item: Item): Item {
    return { name: item.name, level: id }
  }
}

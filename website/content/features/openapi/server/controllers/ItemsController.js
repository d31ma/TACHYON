/** @import { Item } from '/models/Item.ts' */
@RequestRoute("/items")
@Public
class ItemsController {
  /**
   * @param {number} id
   * @returns {Item}
   */
  @GetRoute("/{id}")
  getItem(@FromPath id) {
    return { name: 'anvil', level: id }
  }

  /**
   * @param {number} id
   * @param {Item} item
   * @returns {Item}
   */
  @PatchRoute("/{id}")
  patchItem(@FromPath id, @FromBody item) {
    return { name: item.name, level: id }
  }
}

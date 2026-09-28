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
    return { id }
  }
}

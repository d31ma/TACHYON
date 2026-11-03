@RequestRoute("/items")
@Public
class ItemsController {
  /** @returns {ItemList} */
  @GetRoute
  getItems() {
    return { items: [] }
  }

  /**
   * @param {Item} item
   * @returns {Item}
   */
  @PostRoute
  @ResponseStatus(201)
  postItem(@FromBody item) {
    return item
  }
}
